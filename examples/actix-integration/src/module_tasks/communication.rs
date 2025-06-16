//! Communication tasks - email, notifications, etc.
//!
//! These tasks demonstrate modular task organization with auto-registration

use rust_task_queue::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default, Clone, AutoRegisterTask)]
pub struct EmailTask {
    pub to: String,
    pub subject: String,
    pub body: Option<String>,
}

#[async_trait]
impl Task for EmailTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        println!(
            "[TASKS/COMMUNICATION] Sending email to: {} - {}",
            self.to, self.subject
        );
        if let Some(body) = &self.body {
            println!("   Body: {}", body);
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        #[derive(Serialize)]
        struct EmailResponse {
            status: String,
            to: String,
            subject: String,
        }
        
        let response = EmailResponse {
            status: "sent".to_string(),
            to: self.to.clone(),
            subject: self.subject.clone(),
        };
        
        Ok(rmp_serde::to_vec(&response)?)
    }

    fn name(&self) -> &str {
        "communication_email_task"
    }
}

#[derive(Debug, Serialize, Deserialize, Default, AutoRegisterTask)]
pub struct SlackNotificationTask {
    pub channel: String,
    pub message: String,
    pub username: Option<String>,
}

#[async_trait]
impl Task for SlackNotificationTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let sender = self.username.as_deref().unwrap_or("System");
        println!(
            "💬 [TASKS/COMMUNICATION] Sending Slack message to #{} from {}: {}",
            self.channel, sender, self.message
        );
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        #[derive(Serialize)]
        struct SlackResponse {
            status: String,
            channel: String,
            message: String,
            sender: String,
        }

        let response = SlackResponse {
            status: "sent".to_string(),
            channel: self.channel.clone(),
            message: self.message.clone(),
            sender: sender.to_string(),
        };

        Ok(rmp_serde::to_vec(&response)?)
    }

    fn name(&self) -> &str {
        "slack_notification_task"
    }

    fn max_retries(&self) -> u32 {
        3
    }
}

/// SMS notification task with custom registration name
#[register_task("sms_notification")]
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SmsTask {
    pub phone_number: String,
    pub message: String,
    pub priority: u8,
}

#[async_trait]
impl Task for SmsTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        println!(
            "📱 [TASKS/COMMUNICATION] Sending SMS to {} (priority: {}): {}",
            self.phone_number, self.priority, self.message
        );

        let delay = match self.priority {
            1..=3 => 50,  // High priority - fast
            4..=7 => 100, // Medium priority
            _ => 200,     // Low priority
        };

        tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;

        #[derive(Serialize)]
        struct SmsResponse {
            status: String,
            phone_number: String,
            priority: u8,
        }

        let response = SmsResponse {
            status: "sent".to_string(),
            phone_number: self.phone_number.clone(),
            priority: self.priority,
        };

        Ok(rmp_serde::to_vec(&response)?)
    }

    fn name(&self) -> &str {
        // This name won't be used because we specified "sms_notification" in the register_task attribute
        "sms_task"
    }

    fn max_retries(&self) -> u32 {
        match self.priority {
            1..=3 => 5, // Retry high priority more
            _ => 3,
        }
    }
}
