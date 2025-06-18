//! Communication tasks - email, notifications, webhooks, etc.
//!
//! These tasks demonstrate modular task organization with auto-registration
//! and showcase comprehensive communication features including:
//! - Email with templates and attachments
//! - Multi-channel notifications (Slack, SMS, Push, Discord)
//! - Webhook delivery with retry logic
//! - Batch notifications
//! - Real-time chat integrations

use rust_task_queue::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Default, Clone, AutoRegisterTask)]
pub struct EmailTask {
    pub to: String,
    pub subject: String,
    pub body: Option<String>,
    pub template: Option<String>,
    pub template_data: Option<Vec<u8>>, // JSON serialized template data
    pub attachments: Option<Vec<EmailAttachment>>,
    pub priority: Option<u8>, // 1=high, 2=normal, 3=low
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmailAttachment {
    pub filename: String,
    pub content_type: String,
    pub data: Vec<u8>, // Base64 encoded in practice
}

#[async_trait]
impl Task for EmailTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        println!(
            "[COMMUNICATION] Sending email to: {} - {}",
            self.to, self.subject
        );
        
        if let Some(template) = &self.template {
            println!("  Using template: {}", template);
        }
        
        if let Some(body) = &self.body {
            println!("  Body: {}", body);
        }
        
        if let Some(attachments) = &self.attachments {
            println!("  Attachments: {} files", attachments.len());
        }
        
        // Simulate different processing times based on priority
        let delay = match self.priority.unwrap_or(2) {
            1 => 50,  // High priority - fast
            2 => 100, // Normal priority
            3 => 200, // Low priority
            _ => 100,
        };
        
        tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;

        #[derive(Serialize)]
        struct EmailResponse {
            status: String,
            to: String,
            subject: String,
            message_id: String,
            sent_at: chrono::DateTime<chrono::Utc>,
        }

        let response = EmailResponse {
            status: "sent".to_string(),
            to: self.to.clone(),
            subject: self.subject.clone(),
            message_id: format!("msg_{}", Uuid::new_v4()),
            sent_at: chrono::Utc::now(),
        };

        Ok(rmp_serde::to_vec(&response)?)
    }

    fn name(&self) -> &str {
        "communication_email_task"
    }
    
    fn timeout_seconds(&self) -> u64 {
        120 // 2 minutes for email processing
    }
    
    fn max_retries(&self) -> u32 {
        match self.priority.unwrap_or(2) {
            1 => 5, // High priority gets more retries
            2 => 3, // Normal priority
            3 => 1, // Low priority gets fewer retries
            _ => 3,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default, AutoRegisterTask)]
pub struct SlackNotificationTask {
    pub channel: String,
    pub message: String,
    pub username: Option<String>,
    pub icon_emoji: Option<String>,
    pub attachments: Option<Vec<SlackAttachment>>,
    pub thread_ts: Option<String>, // For threading replies
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SlackAttachment {
    pub color: Option<String>,
    pub pretext: Option<String>,
    pub author_name: Option<String>,
    pub title: Option<String>,
    pub text: Option<String>,
    pub fields: Option<Vec<SlackField>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SlackField {
    pub title: String,
    pub value: String,
    pub short: bool,
}

#[async_trait]
impl Task for SlackNotificationTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let sender = self.username.as_deref().unwrap_or("TaskQueue");
        println!(
            "[COMMUNICATION] Sending Slack message to #{} from {}: {}",
            self.channel, sender, self.message
        );
        
        if let Some(attachments) = &self.attachments {
            println!("  Rich attachments: {} items", attachments.len());
        }
        
        if let Some(thread_ts) = &self.thread_ts {
            println!("  Threading reply to: {}", thread_ts);
        }
        
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        #[derive(Serialize)]
        struct SlackResponse {
            status: String,
            channel: String,
            message: String,
            sender: String,
            timestamp: String,
            permalink: String,
        }

        let timestamp = chrono::Utc::now().timestamp().to_string();
        let response = SlackResponse {
            status: "sent".to_string(),
            channel: self.channel.clone(),
            message: self.message.clone(),
            sender: sender.to_string(),
            timestamp: timestamp.clone(),
            permalink: format!("https://example.slack.com/archives/{}/p{}", self.channel, timestamp),
        };

        Ok(rmp_serde::to_vec(&response)?)
    }

    fn name(&self) -> &str {
        "slack_notification_task"
    }

    fn max_retries(&self) -> u32 {
        3
    }
    
    fn timeout_seconds(&self) -> u64 {
        30 // Quick timeout for chat messages
    }
}

/// SMS notification task with custom registration name and advanced features
#[register_task("sms_notification")]
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SmsTask {
    pub phone_number: String,
    pub message: String,
    pub priority: u8,
    pub country_code: Option<String>,
    pub sender_id: Option<String>,
    pub delivery_receipt: Option<bool>,
    pub scheduled_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[async_trait]
impl Task for SmsTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        println!(
            "[COMMUNICATION] Sending SMS to {} (priority: {}): {}",
            self.phone_number, self.priority, self.message
        );
        
        if let Some(sender_id) = &self.sender_id {
            println!("  From: {}", sender_id);
        }
        
        if let Some(scheduled_at) = &self.scheduled_at {
            println!("  Scheduled for: {}", scheduled_at);
        }

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
            message_id: String,
            cost: f64,
            parts: u32,
        }

        let parts = (self.message.len() / 160 + 1) as u32; // SMS parts calculation
        let response = SmsResponse {
            status: "sent".to_string(),
            phone_number: self.phone_number.clone(),
            priority: self.priority,
            message_id: format!("sms_{}", Uuid::new_v4()),
            cost: parts as f64 * 0.04, // Mock cost calculation
            parts,
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
    
    fn timeout_seconds(&self) -> u64 {
        match self.priority {
            1..=3 => 15, // Quick timeout for high priority
            _ => 30,
        }
    }
}

/// Comprehensive webhook delivery task
#[derive(Debug, Serialize, Deserialize, Default, AutoRegisterTask)]
pub struct WebhookTask {
    pub url: String,
    pub method: String, // GET, POST, PUT, PATCH, DELETE
    pub headers: Option<std::collections::HashMap<String, String>>,
    pub payload: Option<Vec<u8>>, // JSON or other payload
    pub timeout_seconds: Option<u64>,
    pub verify_ssl: Option<bool>,
    pub follow_redirects: Option<bool>,
    pub signature_secret: Option<String>, // For webhook signature verification
}

#[async_trait]
impl Task for WebhookTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        println!(
            "[COMMUNICATION] Sending {} webhook to: {}",
            self.method, self.url
        );
        
        if let Some(headers) = &self.headers {
            println!("  Headers: {} custom headers", headers.len());
        }
        
        if let Some(payload) = &self.payload {
            println!("  Payload: {} bytes", payload.len());
        }
        
        // Simulate HTTP request time
        let timeout = self.timeout_seconds.unwrap_or(30);
        let delay = std::cmp::min(timeout * 10, 500); // Simulate up to 500ms
        tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;

        #[derive(Serialize)]
        struct WebhookResponse {
            status: String,
            url: String,
            method: String,
            response_code: u16,
            response_time_ms: u64,
            response_headers: std::collections::HashMap<String, String>,
            response_body: String,
        }

        let mut response_headers = std::collections::HashMap::new();
        response_headers.insert("content-type".to_string(), "application/json".to_string());
        response_headers.insert("x-webhook-id".to_string(), Uuid::new_v4().to_string());

        let response = WebhookResponse {
            status: "delivered".to_string(),
            url: self.url.clone(),
            method: self.method.clone(),
            response_code: 200,
            response_time_ms: delay,
            response_headers,
            response_body: r#"{"success": true, "message": "Webhook received"}"#.to_string(),
        };

        Ok(rmp_serde::to_vec(&response)?)
    }

    fn name(&self) -> &str {
        "webhook_delivery_task"
    }

    fn max_retries(&self) -> u32 {
        5 // Webhooks need good retry logic
    }
    
    fn timeout_seconds(&self) -> u64 {
        self.timeout_seconds.unwrap_or(30)
    }
}

/// Multi-channel notification task (sends to multiple channels)
#[derive(Debug, Serialize, Deserialize, Default, AutoRegisterTask)]
pub struct MultiChannelNotificationTask {
    pub user_id: String,
    pub message: String,
    pub channels: Vec<NotificationChannel>,
    pub fallback_order: Option<Vec<String>>, // Channel priority for fallback
    pub urgent: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NotificationChannel {
    pub channel_type: String, // "email", "sms", "slack", "push", "discord"
    pub address: String,      // email@example.com, +1234567890, #channel, etc.
    pub config: Option<std::collections::HashMap<String, String>>,
}

#[async_trait]
impl Task for MultiChannelNotificationTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        println!(
            "[COMMUNICATION] Multi-channel notification for user {}: {}",
            self.user_id, self.message
        );
        
        let mut results = Vec::new();
        let is_urgent = self.urgent.unwrap_or(false);
        
        println!("  Channels: {} configured", self.channels.len());
        if is_urgent {
            println!("  URGENT notification - parallel delivery");
        }
        
        // Simulate sending to multiple channels
        for (i, channel) in self.channels.iter().enumerate() {
            println!("    [{}/{}] {} -> {}", 
                i + 1, self.channels.len(), 
                channel.channel_type, channel.address
            );
            
            // Simulate different delivery times per channel
            let delay = match channel.channel_type.as_str() {
                "push" => 50,
                "sms" => 100,
                "slack" => 150,
                "discord" => 175,
                "email" => 300,
                _ => 200,
            };
            
            if !is_urgent {
                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
            }
            
            results.push(format!("{}:sent", channel.channel_type));
        }
        
        if is_urgent {
            // For urgent notifications, simulate parallel processing
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        #[derive(Serialize)]
        struct MultiChannelResponse {
            status: String,
            user_id: String,
            channels_attempted: usize,
            channels_succeeded: usize,
            results: Vec<String>,
            is_urgent: bool,
            delivery_time_ms: u64,
        }

        let response = MultiChannelResponse {
            status: "completed".to_string(),
            user_id: self.user_id.clone(),
            channels_attempted: self.channels.len(),
            channels_succeeded: results.len(),
            results,
            is_urgent,
            delivery_time_ms: if is_urgent { 200 } else { 300 * self.channels.len() as u64 },
        };

        Ok(rmp_serde::to_vec(&response)?)
    }

    fn name(&self) -> &str {
        "multi_channel_notification_task"
    }

    fn max_retries(&self) -> u32 {
        if self.urgent.unwrap_or(false) { 5 } else { 3 }
    }
    
    fn timeout_seconds(&self) -> u64 {
        if self.urgent.unwrap_or(false) { 60 } else { 300 }
    }
}

/// Discord notification with rich embeds
#[derive(Debug, Serialize, Deserialize, Default, AutoRegisterTask)]
pub struct DiscordNotificationTask {
    pub channel_id: String,
    pub content: Option<String>,
    pub embeds: Option<Vec<DiscordEmbed>>,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub tts: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiscordEmbed {
    pub title: Option<String>,
    pub description: Option<String>,
    pub color: Option<u32>,
    pub fields: Option<Vec<DiscordEmbedField>>,
    pub footer: Option<DiscordEmbedFooter>,
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiscordEmbedField {
    pub name: String,
    pub value: String,
    pub inline: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiscordEmbedFooter {
    pub text: String,
    pub icon_url: Option<String>,
}

#[async_trait]
impl Task for DiscordNotificationTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        println!(
            "[COMMUNICATION] Sending Discord message to channel: {}",
            self.channel_id
        );
        
        if let Some(content) = &self.content {
            println!("  Content: {}", content);
        }
        
        if let Some(embeds) = &self.embeds {
            println!("  Rich embeds: {} items", embeds.len());
        }
        
        if self.tts.unwrap_or(false) {
            println!("  Text-to-speech enabled");
        }
        
        tokio::time::sleep(tokio::time::Duration::from_millis(175)).await;

        #[derive(Serialize)]
        struct DiscordResponse {
            status: String,
            channel_id: String,
            message_id: String,
            timestamp: chrono::DateTime<chrono::Utc>,
        }

        let response = DiscordResponse {
            status: "sent".to_string(),
            channel_id: self.channel_id.clone(),
            message_id: format!("discord_{}", Uuid::new_v4()),
            timestamp: chrono::Utc::now(),
        };

        Ok(rmp_serde::to_vec(&response)?)
    }

    fn name(&self) -> &str {
        "discord_notification_task"
    }

    fn max_retries(&self) -> u32 {
        3
    }
    
    fn timeout_seconds(&self) -> u64 {
        30
    }
}
