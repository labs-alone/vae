use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use opencv::{
    prelude::*,
    core::*,
    objdetect,
    dnn,
    types,
};

use crate::vision::processor::Frame;
use crate::models::inference::Model;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectorConfig {
    pub confidence_threshold: f32,
    pub nms_threshold: f32,
    pub device: DetectionDevice,
    pub batch_size: usize,
    pub enabled_detectors: Vec<DetectorType>,
    pub model_configs: Vec<ModelConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DetectorType {
    Object,
    Face,
    Person,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DetectionDevice {
    CPU,
    CUDA,
    OpenCL,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub name: String,
    pub path: String,
    pub framework: ModelFramework,
    pub input_size: (i32, i32),
    pub class_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelFramework {
    ONNX,
    TensorRT,
    OpenVINO,
    Custom(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct Detection {
    pub bbox: BBox,
    pub class_id: usize,
    pub class_name: String,
    pub confidence: f32,
    pub frame_id: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub struct Detector {
    config: DetectorConfig,
    models: Vec<Arc<dyn Model>>,
    detection_count: Arc<Mutex<u64>>,
}

impl Detector {
    pub async fn new(config: DetectorConfig) -> Result<Self> {
        let mut models = Vec::new();
        
        for model_config in &config.model_configs {
            let model = Self::load_model(model_config).await?;
            models.push(Arc::new(model));
        }

        Ok(Self {
            config,
            models,
            detection_count: Arc::new(Mutex::new(0)),
        })
    }

    pub async fn detect(&self, frame: &Frame) -> Result<Vec<Detection>> {
        let mut all_detections = Vec::new();

        for model in &self.models {
            let detections = self.process_frame_with_model(frame, model).await?;
            all_detections.extend(detections);
        }

        // Apply non-maximum suppression
        let filtered_detections = self.apply_nms(all_detections)?;

        // Update detection counter
        let mut counter = self.detection_count.lock().await;
        *counter += filtered_detections.len() as u64;

        Ok(filtered_detections)
    }

    pub async fn detect_batch(&self, frames: &[Frame]) -> Result<Vec<Vec<Detection>>> {
        let mut all_batch_detections = Vec::with_capacity(frames.len());

        for frame in frames {
            let detections = self.detect(frame).await?;
            all_batch_detections.push(detections);
        }

        Ok(all_batch_detections)
    }

    async fn process_frame_with_model(
        &self,
        frame: &Frame,
        model: &Arc<dyn Model>
    ) -> Result<Vec<Detection>> {
        // Prepare input blob
        let blob = self.prepare_input(frame, model)?;

        // Run inference
        let outputs = model.infer(&blob).await?;

        // Process outputs
        let detections = self.process_outputs(outputs, frame)?;

        Ok(detections)
    }

    fn prepare_input(&self, frame: &Frame, model: &Arc<dyn Model>) -> Result<Mat> {
        let mut blob = Mat::default();
        
        dnn::blob_from_image(
            frame.data.as_ref(),
            1.0/255.0,
            Size::new(416, 416),
            Scalar::new(0.0, 0.0, 0.0, 0.0),
            true,
            false,
            CV_32F,
        )?;

        Ok(blob)
    }

    fn process_outputs(&self, outputs: Mat, frame: &Frame) -> Result<Vec<Detection>> {
        let mut detections = Vec::new();
        let rows = outputs.rows();

        for i in 0..rows {
            let confidence = outputs.at_row::<f32>(i)?[4];
            
            if confidence > self.config.confidence_threshold {
                let x = outputs.at_row::<f32>(i)?[0];
                let y = outputs.at_row::<f32>(i)?[1];
                let w = outputs.at_row::<f32>(i)?[2];
                let h = outputs.at_row::<f32>(i)?[3];
                let class_id = outputs.at_row::<f32>(i)?[5] as usize;

                let detection = Detection {
                    bbox: BBox { x, y, width: w, height: h },
                    class_id,
                    class_name: self.get_class_name(class_id)?,
                    confidence,
                    frame_id: frame.id,
                    timestamp: frame.timestamp,
                };

                detections.push(detection);
            }
        }

        Ok(detections)
    }

    fn apply_nms(&self, detections: Vec<Detection>) -> Result<Vec<Detection>> {
        if detections.is_empty() {
            return Ok(detections);
        }

        let mut boxes = Mat::default();
        let mut scores = Mat::default();
        let mut indices = Mat::default();

        // Prepare boxes and scores for NMS
        for detection in &detections {
            boxes.push_row(&Mat::from_slice(&[
                detection.bbox.x,
                detection.bbox.y,
                detection.bbox.width,
                detection.bbox.height,
            ])?)?;
            scores.push_row(&Mat::from_slice(&[detection.confidence])?)?;
        }

        // Apply NMS
        dnn::nms_boxes(
            &boxes,
            &scores,
            self.config.confidence_threshold,
            self.config.nms_threshold,
            &mut indices,
            1.0,
            0,
        )?;

        // Filter detections based on NMS results
        let filtered_detections: Vec<Detection> = indices
            .iter::<i32>()?
            .map(|i| detections[i as usize].clone())
            .collect();

        Ok(filtered_detections)
    }

    fn get_class_name(&self, class_id: usize) -> Result<String> {
        self.config.model_configs
            .first()
            .and_then(|config| config.class_names.get(class_id))
            .map(|name| name.to_string())
            .ok_or_else(|| anyhow::anyhow!("Class name not found for id: {}", class_id))
    }

    async fn load_model(config: &ModelConfig) -> Result<impl Model> {
        // Model loading implementation based on framework
        todo!("Implement model loading")
    }
}