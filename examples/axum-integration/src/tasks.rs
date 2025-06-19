//! Basic task types for Axum integration example

use async_trait::async_trait;
use rust_task_queue::prelude::*;
use serde::{Deserialize, Serialize};
use crate::module_tasks::EmailTask;

// ============================================================================
// Notification Tasks
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[cfg_attr(feature = "auto-register", derive(AutoRegisterTask))]
pub struct NotificationTask {
    pub user_id: String,
    pub message: String,
    pub notification_type: String,
    pub channels: Vec<String>,
}

#[async_trait]
impl Task for NotificationTask {
    async fn execute(&self) -> TaskResult {
        println!("Sending {} notification to user: {}", self.notification_type, self.user_id);
        println!("Message: {}", self.message);
        println!("Channels: {:?}", self.channels);

        // Simulate notification processing
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        #[derive(Serialize)]
        struct Response {
            status: String,
            notification_id: String,
            delivered_channels: Vec<String>,
        }

        let response = Response {
            status: "delivered".to_string(),
            notification_id: uuid::Uuid::new_v4().to_string(),
            delivered_channels: self.channels.clone(),
        };

        Ok(rmp_serde::to_vec(&response)?)
    }

    fn name(&self) -> &str {
        "send_notification"
    }

    fn max_retries(&self) -> u32 {
        2
    }

    fn timeout_seconds(&self) -> u64 {
        15
    }
}

// ============================================================================
// Data Processing Tasks
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[cfg_attr(feature = "auto-register", derive(AutoRegisterTask))]
pub struct DataProcessingTask {
    pub dataset_id: String,
    pub operation: String,
    pub parameters: std::collections::HashMap<String, serde_json::Value>,
}

#[async_trait]
impl Task for DataProcessingTask {
    async fn execute(&self) -> TaskResult {
        println!("Processing dataset: {}", self.dataset_id);
        println!("Operation: {}", self.operation);
        println!("Parameters: {:?}", self.parameters);

        // Simulate data processing
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        #[derive(Serialize)]
        struct Response {
            status: String,
            dataset_id: String,
            operation: String,
            records_processed: u64,
            processing_time_ms: u64,
        }

        let response = Response {
            status: "completed".to_string(),
            dataset_id: self.dataset_id.clone(),
            operation: self.operation.clone(),
            records_processed: 1000 + (uuid::Uuid::new_v4().as_u128() % 9000) as u64, // Simulate 1k-10k records
            processing_time_ms: 2000,
        };

        Ok(rmp_serde::to_vec(&response)?)
    }

    fn name(&self) -> &str {
        "process_data"
    }

    fn max_retries(&self) -> u32 {
        5
    }

    fn timeout_seconds(&self) -> u64 {
        300 // 5 minutes for data processing
    }
}

// ============================================================================
// Schedule Request Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScheduleEmailRequest {
    pub email: EmailTask,
    pub delay_seconds: i64,
} 