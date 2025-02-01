use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemState {
    pub engine_state: EngineState,
    pub pipeline_state: PipelineState,
    pub resource_state: ResourceState,
    pub error_state: ErrorState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineState {
    pub status: EngineStatus,
    pub frames_processed: u64,
    pub fps: f32,
    pub uptime: i64,
    pub last_active: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineState {
    pub active_stages: Vec<String>,
    pub stage_metrics: HashMap<String, StageMetrics>,
    pub queue_size: usize,
    pub processing_latency: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceState {
    pub gpu_usage: f32,
    pub memory_usage: f32,
    pub cpu_usage: f32,
    pub disk_usage: f32,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorState {
    pub error_count: u64,
    pub last_error: Option<ErrorInfo>,
    pub error_history: Vec<ErrorInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageMetrics {
    pub processed_items: u64,
    pub errors: u64,
    pub average_time: f32,
    pub last_processed: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub timestamp: DateTime<Utc>,
    pub error_type: String,
    pub message: String,
    pub context: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum EngineStatus {
    Starting,
    Running,
    Paused,
    Stopping,
    Stopped,
    Error,
}

pub struct StateManager {
    state: Arc<RwLock<SystemState>>,
    history: Arc<RwLock<Vec<StateSnapshot>>>,
    config: StateConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateConfig {
    pub history_size: usize,
    pub snapshot_interval: i64,
    pub persist_state: bool,
    pub state_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StateSnapshot {
    timestamp: DateTime<Utc>,
    state: SystemState,
}

impl StateManager {
    pub async fn new(config: StateConfig) -> Result<Self> {
        let initial_state = SystemState {
            engine_state: EngineState {
                status: EngineStatus::Stopped,
                frames_processed: 0,
                fps: 0.0,
                uptime: 0,
                last_active: Utc::now(),
            },
            pipeline_state: PipelineState {
                active_stages: Vec::new(),
                stage_metrics: HashMap::new(),
                queue_size: 0,
                processing_latency: 0.0,
            },
            resource_state: ResourceState {
                gpu_usage: 0.0,
                memory_usage: 0.0,
                cpu_usage: 0.0,
                disk_usage: 0.0,
                temperature: 0.0,
            },
            error_state: ErrorState {
                error_count: 0,
                last_error: None,
                error_history: Vec::new(),
            },
        };

        let manager = Self {
            state: Arc::new(RwLock::new(initial_state)),
            history: Arc::new(RwLock::new(Vec::new())),
            config,
        };

        // Start state monitoring
        manager.start_monitoring();

        Ok(manager)
    }

    pub async fn update_engine_state(&self, state: EngineState) -> Result<()> {
        let mut system_state = self.state.write().await;
        system_state.engine_state = state;
        self.take_snapshot().await?;
        Ok(())
    }

    pub async fn update_pipeline_state(&self, state: PipelineState) -> Result<()> {
        let mut system_state = self.state.write().await;
        system_state.pipeline_state = state;
        Ok(())
    }

    pub async fn update_resource_state(&self, state: ResourceState) -> Result<()> {
        let mut system_state = self.state.write().await;
        system_state.resource_state = state;
        Ok(())
    }

    pub async fn record_error(&self, error: ErrorInfo) -> Result<()> {
        let mut system_state = self.state.write().await;
        system_state.error_state.error_count += 1;
        system_state.error_state.last_error = Some(error.clone());
        system_state.error_state.error_history.push(error);

        // Trim error history if needed
        if system_state.error_state.error_history.len() > 100 {
            system_state.error_state.error_history.remove(0);
        }

        Ok(())
    }

    pub async fn get_current_state(&self) -> Result<SystemState> {
        Ok(self.state.read().await.clone())
    }

    pub async fn get_state_history(&self) -> Result<Vec<StateSnapshot>> {
        Ok(self.history.read().await.clone())
    }

    async fn take_snapshot(&self) -> Result<()> {
        let current_state = self.state.read().await.clone();
        let snapshot = StateSnapshot {
            timestamp: Utc::now(),
            state: current_state,
        };

        let mut history = self.history.write().await;
        history.push(snapshot);

        // Trim history if needed
        while history.len() > self.config.history_size {
            history.remove(0);
        }

        if self.config.persist_state {
            self.persist_state().await?;
        }

        Ok(())
    }

    async fn persist_state(&self) -> Result<()> {
        let state = self.state.read().await;
        let serialized = serde_json::to_string_pretty(&*state)?;
        tokio::fs::write(&self.config.state_file, serialized).await?;
        Ok(())
    }

    fn start_monitoring(&self) {
        let state = self.state.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                tokio::time::Duration::from_secs(config.snapshot_interval as u64)
            );

            loop {
                interval.tick().await;
                let mut system_state = state.write().await;
                
                // Update resource metrics
                system_state.resource_state = ResourceState {
                    gpu_usage: get_gpu_usage(),
                    memory_usage: get_memory_usage(),
                    cpu_usage: get_cpu_usage(),
                    disk_usage: get_disk_usage(),
                    temperature: get_temperature(),
                };

                // Update engine metrics
                if system_state.engine_state.status == EngineStatus::Running {
                    system_state.engine_state.uptime += config.snapshot_interval;
                }
            }
        });
    }
}

// Helper functions for resource monitoring
fn get_gpu_usage() -> f32 {
    // Implement GPU usage monitoring
    0.0
}

fn get_memory_usage() -> f32 {
    // Implement memory usage monitoring
    0.0
}

fn get_cpu_usage() -> f32 {
    // Implement CPU usage monitoring
    0.0
}

fn get_disk_usage() -> f32 {
    // Implement disk usage monitoring
    0.0
}

fn get_temperature() -> f32 {
    // Implement temperature monitoring
    0.0
}