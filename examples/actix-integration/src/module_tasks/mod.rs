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
    DiscordEmbed,
    DiscordEmbedField,
    DiscordEmbedFooter,
    DiscordNotificationTask,
    // Supporting types
    EmailAttachment,
    EmailTask,
    MultiChannelNotificationTask,
    NotificationChannel,
    SlackAttachment,
    SlackField,
    SlackNotificationTask,
    SmsTask,
    WebhookTask,
};

// Re-export all processing tasks
pub use processing::{
    AnalyticsTask,
    BackupTask,
    BatchItem,
    BatchProcessingTask,
    CacheWarmupTask,
    // Supporting types
    DataFilter,
    DataProcessingTask,
    DataTransformation,
    DatabaseMaintenanceTask,
    DateRange,
    FileProcessingTask,
    FileValidationRule,
    ImageOperation,
    ImageProcessingTask,
    MLTrainingTask,
    ReportGenerationTask,
};
