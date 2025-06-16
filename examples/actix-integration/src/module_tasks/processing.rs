//! Data processing tasks - analytics, transformations, etc.
//!
//! These tasks demonstrate modular task organization with auto-registration

use rust_task_queue::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default, AutoRegisterTask)]
pub struct DataProcessingTask {
    pub data_id: String,
    pub operation: String,
    pub batch_size: Option<usize>,
}

#[async_trait]
impl Task for DataProcessingTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let batch_size = self.batch_size.unwrap_or(100);
        println!(
            "[TASKS/PROCESSING] Processing data: {} with operation: {} (batch: {})",
            self.data_id, self.operation, batch_size
        );

        // Simulate processing time based on batch size
        let processing_time = (batch_size / 10).max(50);
        tokio::time::sleep(tokio::time::Duration::from_millis(processing_time as u64)).await;

        #[derive(Serialize)]
        struct DataProcessingResponse {
            status: String,
            data_id: String,
            operation: String,
            batch_size: usize,
            processing_time_ms: usize,
        }

        let response = DataProcessingResponse {
            status: "processed".to_string(),
            data_id: self.data_id.clone(),
            operation: self.operation.clone(),
            batch_size,
            processing_time_ms: processing_time,
        };

        Ok(rmp_serde::to_vec(&response)?)
    }

    fn name(&self) -> &str {
        "modular_data_processing_task"
    }

    fn timeout_seconds(&self) -> u64 {
        600 // Longer timeout for data processing
    }

    fn max_retries(&self) -> u32 {
        5
    }
}

#[derive(Debug, Serialize, Deserialize, Default, AutoRegisterTask)]
pub struct AnalyticsTask {
    pub event_name: String,
    pub user_id: String,
    pub properties: Vec<u8>,
    pub session_id: Option<String>,
}

#[async_trait]
impl Task for AnalyticsTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        println!(
            "[TASKS/PROCESSING] Recording analytics event: {} for user: {}",
            self.event_name, self.user_id
        );

        if let Some(session_id) = &self.session_id {
            println!("   Session: {}", session_id);
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(75)).await;

        #[derive(Serialize)]
        struct AnalyticsResponse {
            status: String,
            event: String,
            user_id: String,
            session_id: Option<String>,
            properties: Vec<u8>,
            timestamp: chrono::DateTime<chrono::Utc>,
        }

        let response = AnalyticsResponse {
            status: "recorded".to_string(),
            event: self.event_name.clone(),
            user_id: self.user_id.clone(),
            session_id: self.session_id.clone(),
            properties: self.properties.clone(),
            timestamp: chrono::Utc::now(),
        };

        Ok(rmp_serde::to_vec(&response)?)
    }

    fn name(&self) -> &str {
        "analytics_task"
    }

    fn max_retries(&self) -> u32 {
        2 // Analytics events are less critical
    }
}

/// Machine learning model training task
#[register_task("ml_training")]
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct MLTrainingTask {
    pub model_name: String,
    pub dataset_path: String,
    pub epochs: u32,
    pub learning_rate: f64,
}

#[async_trait]
impl Task for MLTrainingTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        println!(
            "[TASKS/PROCESSING] Training ML model: {} with {} epochs (lr: {})",
            self.model_name, self.epochs, self.learning_rate
        );
        println!("   Dataset: {}", self.dataset_path);

        // Simulate training time (very quick for demo)
        let training_time = (self.epochs * 10).min(1000);
        tokio::time::sleep(tokio::time::Duration::from_millis(training_time as u64)).await;

        #[derive(Serialize)]
        struct MLTrainingResponse {
            status: String,
            model_name: String,
            epochs: u32,
            learning_rate: f64,
            training_time_ms: u32,
            accuracy: f64,
        }

        let response = MLTrainingResponse {
            status: "trained".to_string(),
            model_name: self.model_name.clone(),
            epochs: self.epochs,
            learning_rate: self.learning_rate,
            training_time_ms: training_time,
            accuracy: 0.95, // Mock accuracy
        };

        Ok(rmp_serde::to_vec(&response)?)
    }

    fn name(&self) -> &str {
        // This name won't be used because we specified "ml_training" in the register_task attribute
        "ml_training_task"
    }

    fn timeout_seconds(&self) -> u64 {
        3600 // 1 hour for ML training
    }

    fn max_retries(&self) -> u32 {
        1 // Training is expensive, don't retry often
    }
}
