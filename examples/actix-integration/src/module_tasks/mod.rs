//! Task modules for the actix integration example
//!
//! This module demonstrates how to organize tasks in separate files
//! while still enabling auto-registration functionality.
//! 
//! Features comprehensive task types including:
//! - Communication: Email, Slack, SMS, Discord, Webhooks, Multi-channel notifications
//! - Processing: Data processing, ML training, File processing, Image processing
//! - Operations: Backups, Database maintenance, Cache warming, Batch processing
//! - Analytics: Event tracking, Report generation

pub mod communication;
pub mod processing;

// Re-export all communication tasks
pub use communication::{
    EmailTask, SlackNotificationTask, SmsTask, WebhookTask,
    MultiChannelNotificationTask, DiscordNotificationTask,
    // Supporting types
    EmailAttachment, SlackAttachment, SlackField, NotificationChannel,
    DiscordEmbed, DiscordEmbedField, DiscordEmbedFooter
};

// Re-export all processing tasks
pub use processing::{
    DataProcessingTask, AnalyticsTask, MLTrainingTask, FileProcessingTask,
    ImageProcessingTask, ReportGenerationTask, BackupTask,
    DatabaseMaintenanceTask, CacheWarmupTask, BatchProcessingTask,
    // Supporting types
    DataFilter, DataTransformation, FileValidationRule, ImageOperation,
    DateRange, BatchItem
};
