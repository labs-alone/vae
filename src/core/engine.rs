use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use anyhow::{Result, Context};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use crate::vision::{processor::Frame, detector::Detection, analyzer::Analysis};
use crate::models::inference::InferenceResult;
use crate::runtime::gpu::GPUManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub max_batch_size: usize,
    pub processing_threads: usize,
    pub enable_gpu: bool,
    pub model_precision: String,
    pub detection_threshold: f32,
    pub enable_analytics: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 32,
            processing_threads: 4,
            enable_gpu: true,
            model_precision: String::from("fp16"),
            detection_threshold: 0.5,
            enable_analytics: true,
        }
    }
}

#[derive(Debug)]
pub struct ProcessingResult {
    pub frame_id: u64,
    pub detections: Vec<Detection>,
    pub analysis: Option<Analysis>,
    pub inference: Option<InferenceResult>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[async_trait]
pub trait FrameProcessor: Send + Sync {
    async fn process_frame(&self, frame: Frame) -> Result<ProcessingResult>;
}

pub struct Engine {
    config: EngineConfig,
    gpu_manager: Arc<GPUManager>,
    frame_processor: Arc<dyn FrameProcessor>,
    processing_queue: mpsc::Sender<Frame>,
    result_channel: mpsc::Receiver<ProcessingResult>,
    state: Arc<Mutex<EngineState>>,
}

#[derive(Debug)]
struct EngineState {
    is_running: bool,
    frames_processed: u64,
    error_count: u64,
    last_error: Option<String>,
    start_time: chrono::DateTime<chrono::Utc>,
}

impl Engine {
    pub async fn new(config: EngineConfig) -> Result<Self> {
        let (tx, rx) = mpsc::channel(config.max_batch_size);
        let (result_tx, result_rx) = mpsc::channel(config.max_batch_size);

        let gpu_manager = Arc::new(GPUManager::new(config.enable_gpu)?);
        let frame_processor = Arc::new(DefaultFrameProcessor::new(
            config.clone(),
            gpu_manager.clone(),
            result_tx,
        ));

        let engine = Self {
            config,
            gpu_manager,
            frame_processor,
            processing_queue: tx,
            result_channel: result_rx,
            state: Arc::new(Mutex::new(EngineState {
                is_running: false,
                frames_processed: 0,
                error_count: 0,
                last_error: None,
                start_time: chrono::Utc::now(),
            })),
        };

        Ok(engine)
    }

    pub async fn start(&mut self) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        if state.is_running {
            return Ok(());
        }

        state.is_running = true;
        state.start_time = chrono::Utc::now();
        drop(state);

        self.initialize_workers().await?;
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        if !state.is_running {
            return Ok(());
        }

        state.is_running = false;
        drop(state);

        // Cleanup resources
        self.gpu_manager.cleanup().await?;
        Ok(())
    }

    pub async fn process_frame(&self, frame: Frame) -> Result<()> {
        self.processing_queue.send(frame).await
            .context("Failed to send frame to processing queue")?;
        Ok(())
    }

    pub async fn get_result(&mut self) -> Option<ProcessingResult> {
        self.result_channel.recv().await
    }

    async fn initialize_workers(&self) -> Result<()> {
        let num_workers = self.config.processing_threads;
        let processor = self.frame_processor.clone();
        
        for _ in 0..num_workers {
            let processor = processor.clone();
            tokio::spawn(async move {
                while let Some(frame) = processor.process_frame().await {
                    if let Err(e) = processor.process_frame(frame).await {
                        log::error!("Frame processing error: {}", e);
                    }
                }
            });
        }

        Ok(())
    }

    pub fn get_metrics(&self) -> Result<EngineMetrics> {
        let state = self.state.lock().unwrap();
        Ok(EngineMetrics {
            frames_processed: state.frames_processed,
            error_count: state.error_count,
            uptime: chrono::Utc::now() - state.start_time,
            is_running: state.is_running,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct EngineMetrics {
    pub frames_processed: u64,
    pub error_count: u64,
    pub uptime: chrono::Duration,
    pub is_running: bool,
}

struct DefaultFrameProcessor {
    config: EngineConfig,
    gpu_manager: Arc<GPUManager>,
    result_sender: mpsc::Sender<ProcessingResult>,
}

impl DefaultFrameProcessor {
    fn new(
        config: EngineConfig,
        gpu_manager: Arc<GPUManager>,
        result_sender: mpsc::Sender<ProcessingResult>,
    ) -> Self {
        Self {
            config,
            gpu_manager,
            result_sender,
        }
    }
}

#[async_trait]
impl FrameProcessor for DefaultFrameProcessor {
    async fn process_frame(&self, frame: Frame) -> Result<ProcessingResult> {
        // Process frame using GPU if available
        let detections = if self.config.enable_gpu {
            self.gpu_manager.detect_objects(&frame).await?
        } else {
            vec![] // CPU fallback implementation
        };

        // Perform analysis if enabled
        let analysis = if self.config.enable_analytics {
            Some(self.analyze_frame(&frame, &detections).await?)
        } else {
            None
        };

        let result = ProcessingResult {
            frame_id: frame.id,
            detections,
            analysis,
            inference: None, // Add inference results if needed
            timestamp: chrono::Utc::now(),
        };

        self.result_sender.send(result.clone()).await
            .context("Failed to send processing result")?;

        Ok(result)
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        // Ensure cleanup runs when engine is dropped
        if let Ok(mut state) = self.state.lock() {
            state.is_running = false;
        }
    }
}