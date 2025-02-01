use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use anyhow::{Result, Context};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

use crate::vision::{
    processor::Frame,
    detector::Detection,
    analyzer::Analysis
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub stages: Vec<StageConfig>,
    pub max_parallel_stages: usize,
    pub buffer_size: usize,
    pub timeout_ms: u64,
    pub retry_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageConfig {
    pub name: String,
    pub stage_type: StageType,
    pub enabled: bool,
    pub params: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StageType {
    PreProcess,
    Detection,
    Analysis,
    Inference,
    PostProcess,
}

#[async_trait]
pub trait PipelineStage: Send + Sync {
    async fn process(&self, input: PipelineData) -> Result<PipelineData>;
    fn stage_type(&self) -> StageType;
    fn name(&self) -> String;
}

#[derive(Debug, Clone)]
pub struct PipelineData {
    pub frame: Frame,
    pub detections: Vec<Detection>,
    pub analysis: Option<Analysis>,
    pub metadata: HashMap<String, String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct Pipeline {
    config: PipelineConfig,
    stages: Vec<Arc<dyn PipelineStage>>,
    input_channel: mpsc::Sender<PipelineData>,
    output_channel: mpsc::Receiver<PipelineData>,
    state: Arc<RwLock<PipelineState>>,
}

#[derive(Debug, Clone, Serialize)]
struct PipelineState {
    is_running: bool,
    processed_frames: u64,
    errors: u64,
    stage_metrics: HashMap<String, StageMetrics>,
    start_time: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize)]
struct StageMetrics {
    processed: u64,
    errors: u64,
    avg_processing_time: f64,
    last_processed: chrono::DateTime<chrono::Utc>,
}

impl Pipeline {
    pub async fn new(config: PipelineConfig) -> Result<Self> {
        let (tx, rx) = mpsc::channel(config.buffer_size);
        let (output_tx, output_rx) = mpsc::channel(config.buffer_size);

        let mut stages = Vec::new();
        for stage_config in &config.stages {
            let stage = create_stage(stage_config)?;
            stages.push(Arc::new(stage));
        }

        let state = Arc::new(RwLock::new(PipelineState {
            is_running: false,
            processed_frames: 0,
            errors: 0,
            stage_metrics: HashMap::new(),
            start_time: chrono::Utc::now(),
        }));

        let pipeline = Self {
            config,
            stages,
            input_channel: tx,
            output_channel: output_rx,
            state,
        };

        Ok(pipeline)
    }

    pub async fn start(&mut self) -> Result<()> {
        let mut state = self.state.write().await;
        if state.is_running {
            return Ok(());
        }

        state.is_running = true;
        state.start_time = chrono::Utc::now();
        drop(state);

        self.spawn_workers().await?;
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        let mut state = self.state.write().await;
        if !state.is_running {
            return Ok(());
        }

        state.is_running = false;
        drop(state);

        Ok(())
    }

    async fn spawn_workers(&self) -> Result<()> {
        let max_parallel = self.config.max_parallel_stages;
        let stages = self.stages.clone();
        let state = self.state.clone();

        for i in 0..max_parallel {
            let stages = stages.clone();
            let state = state.clone();
            
            tokio::spawn(async move {
                while let Ok(mut data) = self.input_channel.recv().await {
                    for stage in &stages {
                        match stage.process(data.clone()).await {
                            Ok(processed_data) => {
                                data = processed_data;
                                update_metrics(&state, &stage.name(), true).await;
                            }
                            Err(e) => {
                                log::error!("Stage {} error: {}", stage.name(), e);
                                update_metrics(&state, &stage.name(), false).await;
                                break;
                            }
                        }
                    }
                }
            });
        }

        Ok(())
    }

    pub async fn process(&self, frame: Frame) -> Result<()> {
        let data = PipelineData {
            frame,
            detections: Vec::new(),
            analysis: None,
            metadata: HashMap::new(),
            timestamp: chrono::Utc::now(),
        };

        self.input_channel.send(data).await
            .context("Failed to send data to pipeline")?;
        Ok(())
    }

    pub async fn get_result(&mut self) -> Option<PipelineData> {
        self.output_channel.recv().await
    }

    pub async fn get_metrics(&self) -> PipelineMetrics {
        let state = self.state.read().await;
        PipelineMetrics {
            processed_frames: state.processed_frames,
            errors: state.errors,
            stage_metrics: state.stage_metrics.clone(),
            uptime: chrono::Utc::now() - state.start_time,
            is_running: state.is_running,
        }
    }
}

async fn update_metrics(state: &Arc<RwLock<PipelineState>>, stage_name: &str, success: bool) {
    let mut state = state.write().await;
    let metrics = state.stage_metrics.entry(stage_name.to_string())
        .or_insert_with(|| StageMetrics {
            processed: 0,
            errors: 0,
            avg_processing_time: 0.0,
            last_processed: chrono::Utc::now(),
        });

    if success {
        metrics.processed += 1;
    } else {
        metrics.errors += 1;
    }
    metrics.last_processed = chrono::Utc::now();
}

#[derive(Debug, Serialize)]
pub struct PipelineMetrics {
    pub processed_frames: u64,
    pub errors: u64,
    pub stage_metrics: HashMap<String, StageMetrics>,
    pub uptime: chrono::Duration,
    pub is_running: bool,
}

fn create_stage(config: &StageConfig) -> Result<Box<dyn PipelineStage>> {
    match config.stage_type {
        StageType::PreProcess => Ok(Box::new(PreProcessStage::new(config.clone()))),
        StageType::Detection => Ok(Box::new(DetectionStage::new(config.clone()))),
        StageType::Analysis => Ok(Box::new(AnalysisStage::new(config.clone()))),
        StageType::Inference => Ok(Box::new(InferenceStage::new(config.clone()))),
        StageType::PostProcess => Ok(Box::new(PostProcessStage::new(config.clone()))),
    }
}

// Stage implementations
struct PreProcessStage {
    config: StageConfig,
}

impl PreProcessStage {
    fn new(config: StageConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl PipelineStage for PreProcessStage {
    async fn process(&self, input: PipelineData) -> Result<PipelineData> {
        // Implement pre-processing logic
        Ok(input)
    }

    fn stage_type(&self) -> StageType {
        StageType::PreProcess
    }

    fn name(&self) -> String {
        self.config.name.clone()
    }
}

// Similar implementations for other stages...