use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::{Result, Context};
use image::{DynamicImage, ImageBuffer, Rgb};
use opencv::{
    prelude::*,
    core::*,
    imgproc,
    videoio,
};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessorConfig {
    pub input_size: (u32, u32),
    pub normalize: bool,
    pub color_space: ColorSpace,
    pub preprocessing: Vec<PreprocessingStep>,
    pub batch_size: usize,
    pub device: ProcessingDevice,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ColorSpace {
    RGB,
    BGR,
    GRAY,
    HSV,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PreprocessingStep {
    Resize(u32, u32),
    Normalize,
    GaussianBlur(f64),
    MedianBlur(i32),
    Threshold(f64),
    Sharpen,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessingDevice {
    CPU,
    GPU(i32), // GPU device ID
}

#[derive(Debug, Clone)]
pub struct Frame {
    pub id: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub data: Arc<Mat>,
    pub metadata: FrameMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameMetadata {
    pub width: u32,
    pub height: u32,
    pub channels: u8,
    pub format: String,
    pub source: String,
}

pub struct Processor {
    config: ProcessorConfig,
    frame_counter: Arc<Mutex<u64>>,
    capture: Option<videoio::VideoCapture>,
    preprocessing_pipeline: Vec<Box<dyn PreprocessingOperation>>,
}

#[async_trait::async_trait]
pub trait PreprocessingOperation: Send + Sync {
    async fn process(&self, frame: &mut Mat) -> Result<()>;
}

impl Processor {
    pub fn new(config: ProcessorConfig) -> Result<Self> {
        let preprocessing_pipeline = Self::build_preprocessing_pipeline(&config.preprocessing)?;

        Ok(Self {
            config,
            frame_counter: Arc::new(Mutex::new(0)),
            capture: None,
            preprocessing_pipeline,
        })
    }

    pub async fn process_frame(&self, mut frame: Mat) -> Result<Frame> {
        // Apply preprocessing steps
        for operation in &self.preprocessing_pipeline {
            operation.process(&mut frame).await?;
        }

        // Convert color space if needed
        match self.config.color_space {
            ColorSpace::RGB => {
                let mut rgb = Mat::default();
                imgproc::cvt_color(&frame, &mut rgb, imgproc::COLOR_BGR2RGB, 0)?;
                frame = rgb;
            }
            ColorSpace::GRAY => {
                let mut gray = Mat::default();
                imgproc::cvt_color(&frame, &mut gray, imgproc::COLOR_BGR2GRAY, 0)?;
                frame = gray;
            }
            ColorSpace::HSV => {
                let mut hsv = Mat::default();
                imgproc::cvt_color(&frame, &mut hsv, imgproc::COLOR_BGR2HSV, 0)?;
                frame = hsv;
            }
            _ => {}
        }

        // Create frame metadata
        let metadata = FrameMetadata {
            width: frame.cols() as u32,
            height: frame.rows() as u32,
            channels: frame.channels() as u8,
            format: self.config.color_space.to_string(),
            source: "processor".to_string(),
        };

        // Increment frame counter
        let mut counter = self.frame_counter.lock().await;
        *counter += 1;

        Ok(Frame {
            id: *counter,
            timestamp: chrono::Utc::now(),
            data: Arc::new(frame),
            metadata,
        })
    }

    pub async fn process_batch(&self, frames: Vec<Mat>) -> Result<Vec<Frame>> {
        let mut processed_frames = Vec::with_capacity(frames.len());

        for frame in frames {
            let processed = self.process_frame(frame).await?;
            processed_frames.push(processed);
        }

        Ok(processed_frames)
    }

    fn build_preprocessing_pipeline(
        steps: &[PreprocessingStep]
    ) -> Result<Vec<Box<dyn PreprocessingOperation>>> {
        let mut pipeline = Vec::new();

        for step in steps {
            let operation: Box<dyn PreprocessingOperation> = match step {
                PreprocessingStep::Resize(width, height) => {
                    Box::new(ResizeOperation { width: *width, height: *height })
                }
                PreprocessingStep::GaussianBlur(sigma) => {
                    Box::new(GaussianBlurOperation { sigma: *sigma })
                }
                PreprocessingStep::MedianBlur(ksize) => {
                    Box::new(MedianBlurOperation { ksize: *ksize })
                }
                PreprocessingStep::Threshold(thresh) => {
                    Box::new(ThresholdOperation { threshold: *thresh })
                }
                PreprocessingStep::Sharpen => Box::new(SharpenOperation {}),
                _ => continue,
            };
            pipeline.push(operation);
        }

        Ok(pipeline)
    }

    pub async fn start_capture(&mut self, source: &str) -> Result<()> {
        let mut cap = videoio::VideoCapture::from_file(source, videoio::CAP_ANY)?;
        if !cap.is_opened()? {
            return Err(anyhow::anyhow!("Failed to open video capture"));
        }
        self.capture = Some(cap);
        Ok(())
    }

    pub async fn read_frame(&mut self) -> Result<Option<Frame>> {
        if let Some(cap) = &mut self.capture {
            let mut frame = Mat::default();
            if cap.read(&mut frame)? {
                Ok(Some(self.process_frame(frame).await?))
            } else {
                Ok(None)
            }
        } else {
            Err(anyhow::anyhow!("No capture device initialized"))
        }
    }
}

// Preprocessing Operations Implementation
struct ResizeOperation {
    width: u32,
    height: u32,
}

#[async_trait::async_trait]
impl PreprocessingOperation for ResizeOperation {
    async fn process(&self, frame: &mut Mat) -> Result<()> {
        let mut resized = Mat::default();
        imgproc::resize(
            frame,
            &mut resized,
            Size::new(self.width as i32, self.height as i32),
            0.0,
            0.0,
            imgproc::INTER_LINEAR,
        )?;
        *frame = resized;
        Ok(())
    }
}

// Similar implementations for other preprocessing operations...

impl ToString for ColorSpace {
    fn to_string(&self) -> String {
        match self {
            ColorSpace::RGB => "RGB".to_string(),
            ColorSpace::BGR => "BGR".to_string(),
            ColorSpace::GRAY => "GRAY".to_string(),
            ColorSpace::HSV => "HSV".to_string(),
        }
    }
}