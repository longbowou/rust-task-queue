// Import everything needed for the task definitions
use rust_task_queue::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default, Clone, AutoRegisterTask)]
pub struct EmailTask {
    pub to: String,
    pub subject: String,
}

#[async_trait]
impl Task for EmailTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        println!("Sending email to: {} - {}", self.to, self.subject);
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        println!("Email sent to: {} - {}", self.to, self.subject);

        #[derive(Serialize)]
        struct EmailResponse {
            status: String,
            to: String,
        }

        let response = EmailResponse {
            status: "sent".to_string(),
            to: self.to.clone(),
        };

        Ok(rmp_serde::to_vec(&response)?)
    }

    fn name(&self) -> &str {
        "email_task"
    }
}

/// Simple notification task
#[derive(Debug, Serialize, Deserialize, Default, Clone, AutoRegisterTask)]
pub struct NotificationTask {
    pub user_id: String,
    pub message: String,
    pub notification_type: String,
}

#[async_trait]
impl Task for NotificationTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        println!(
            "🔔 [WORKER] Sending {} notification to user {}: {}",
            self.notification_type, self.user_id, self.message
        );

        // Simulate different processing times based on notification type
        let delay = match self.notification_type.as_str() {
            "email" => 300,
            "sms" => 150,
            "push" => 50,
            _ => 100,
        };

        tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;

        #[derive(Serialize)]
        struct NotificationResponse {
            status: String,
            user_id: String,
            notification_type: String,
            message: String,
        }

        let response = NotificationResponse {
            status: "sent".to_string(),
            user_id: self.user_id.clone(),
            notification_type: self.notification_type.clone(),
            message: self.message.clone(),
        };

        Ok(rmp_serde::to_vec(&response)?)
    }

    fn name(&self) -> &str {
        "notification_task"
    }

    fn max_retries(&self) -> u32 {
        match self.notification_type.as_str() {
            "email" => 3,
            "sms" => 5,
            "push" => 2,
            _ => 3,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default, AutoRegisterTask)]
pub struct DataProcessingTask {
    pub data_id: String,
    pub operation: String,
}

#[async_trait]
impl Task for DataProcessingTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        // Simulate data processing
        println!(
            "🔄 [WORKER] Processing data: {} with operation: {}",
            self.data_id, self.operation
        );
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        #[derive(Serialize)]
        struct DataProcessingResponse {
            status: String,
            data_id: String,
        }

        let response = DataProcessingResponse {
            status: "processed".to_string(),
            data_id: self.data_id.clone(),
        };

        Ok(rmp_serde::to_vec(&response)?)
    }

    fn name(&self) -> &str {
        "legacy_data_processing_task"
    }

    fn timeout_seconds(&self) -> u64 {
        600 // Longer timeout for data processing
    }

    fn max_retries(&self) -> u32 {
        5
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScheduleEmailRequest {
    pub email: EmailTask,
    pub delay_seconds: i64,
}

// Re-export everything for convenience
// (Note: this is not used in the main app but kept for completeness)
