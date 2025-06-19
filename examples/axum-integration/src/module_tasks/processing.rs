//! Data processing tasks - analytics, transformations, file processing, etc.
//!
//! These tasks demonstrate modular task organization with auto-registration
//! and showcase comprehensive processing features including:
//! - Data processing with batch support and streaming
//! - File processing (CSV, JSON, XML, images, videos)
//! - Analytics and reporting
//! - Machine learning model training and inference
//! - Database maintenance and optimization
//! - Backup and restoration tasks
//! - Cache warming and performance optimization

use rust_task_queue::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Default, AutoRegisterTask)]
pub struct DataProcessingTask {
    pub data_id: String,
    pub operation: String,
    pub batch_size: Option<usize>,
    pub input_format: Option<String>, // "json", "csv", "xml", "parquet"
    pub output_format: Option<String>,
    pub filters: Option<Vec<DataFilter>>,
    pub transformations: Option<Vec<DataTransformation>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DataFilter {
    pub field: String,
    pub operator: String, // "eq", "ne", "gt", "lt", "contains", "regex"
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DataTransformation {
    pub field: String,
    pub operation: String, // "uppercase", "lowercase", "trim", "format", "calculate"
    pub parameters: Option<std::collections::HashMap<String, String>>,
}

#[async_trait]
impl Task for DataProcessingTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let batch_size = self.batch_size.unwrap_or(100);
        println!(
            "[PROCESSING] Processing data: {} with operation: {} (batch: {})",
            self.data_id, self.operation, batch_size
        );

        if let Some(input_format) = &self.input_format {
            println!("  Input format: {}", input_format);
        }

        if let Some(filters) = &self.filters {
            println!("  Filters: {} conditions", filters.len());
        }

        if let Some(transformations) = &self.transformations {
            println!("  Transformations: {} operations", transformations.len());
        }

        // Simulate processing time based on batch size and complexity
        let base_time = (batch_size / 10).max(50);
        let filter_time = self.filters.as_ref().map_or(0, |f| f.len() * 10);
        let transform_time = self.transformations.as_ref().map_or(0, |t| t.len() * 15);
        let processing_time = base_time + filter_time + transform_time;

        tokio::time::sleep(tokio::time::Duration::from_millis(processing_time as u64)).await;

        #[derive(Serialize)]
        struct DataProcessingResponse {
            status: String,
            data_id: String,
            operation: String,
            batch_size: usize,
            records_processed: usize,
            records_filtered: usize,
            records_transformed: usize,
            processing_time_ms: usize,
            output_location: String,
        }

        let response = DataProcessingResponse {
            status: "processed".to_string(),
            data_id: self.data_id.clone(),
            operation: self.operation.clone(),
            batch_size,
            records_processed: batch_size,
            records_filtered: batch_size / 10, // Mock filtered count
            records_transformed: batch_size - (batch_size / 10),
            processing_time_ms: processing_time,
            output_location: format!("s3://processed-data/{}.parquet", self.data_id),
        };

        Ok(rmp_serde::to_vec(&response)?)
    }

    fn name(&self) -> &str {
        "modular_data_processing_task"
    }

    fn timeout_seconds(&self) -> u64 {
        // Dynamic timeout based on batch size
        let base_timeout = 600; // 10 minutes base
        let batch_factor = self.batch_size.unwrap_or(100) / 100;
        base_timeout + (batch_factor as u64 * 300) // +5 minutes per 100 records
    }

    fn max_retries(&self) -> u32 {
        5
    }
}

#[derive(Debug, Serialize, Deserialize, Default, AutoRegisterTask)]
pub struct AnalyticsTask {
    pub event_name: String,
    pub user_id: String,
    pub properties: String, // Base64 encoded JSON properties
    pub session_id: Option<String>,
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
    pub source: Option<String>,
    pub campaign_id: Option<String>,
}

#[async_trait]
impl Task for AnalyticsTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        println!(
            "[PROCESSING] Recording analytics event: {} for user: {}",
            self.event_name, self.user_id
        );

        if let Some(session_id) = &self.session_id {
            println!("  Session: {}", session_id);
        }

        if let Some(source) = &self.source {
            println!("  Source: {}", source);
        }

        if let Some(campaign_id) = &self.campaign_id {
            println!("  Campaign: {}", campaign_id);
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(75)).await;

        #[derive(Serialize)]
        struct AnalyticsResponse {
            status: String,
            event: String,
            user_id: String,
            session_id: Option<String>,
            properties: String,
            processed_at: chrono::DateTime<chrono::Utc>,
            data_warehouse_id: String,
        }

        let response = AnalyticsResponse {
            status: "recorded".to_string(),
            event: self.event_name.clone(),
            user_id: self.user_id.clone(),
            session_id: self.session_id.clone(),
            properties: self.properties.clone(),
            processed_at: chrono::Utc::now(),
            data_warehouse_id: format!("dw_{}", Uuid::new_v4()),
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

/// Machine learning model training task with comprehensive features
#[register_task("ml_training")]
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct MLTrainingTask {
    pub model_name: String,
    pub dataset_path: String,
    pub model_type: String, // "classification", "regression", "clustering", "neural_network"
    pub epochs: u32,
    pub learning_rate: f64,
    pub batch_size: Option<u32>,
    pub validation_split: Option<f64>,
    pub hyperparameters: Option<std::collections::HashMap<String, String>>,
    pub gpu_enabled: Option<bool>,
}

#[async_trait]
impl Task for MLTrainingTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        println!(
            "[PROCESSING] Training {} model: {} with {} epochs (lr: {})",
            self.model_type, self.model_name, self.epochs, self.learning_rate
        );
        println!("  Dataset: {}", self.dataset_path);

        if let Some(batch_size) = self.batch_size {
            println!("  Batch size: {}", batch_size);
        }

        if let Some(gpu_enabled) = self.gpu_enabled {
            println!("  GPU enabled: {}", gpu_enabled);
        }

        if let Some(hyperparams) = &self.hyperparameters {
            println!("  Hyperparameters: {} configured", hyperparams.len());
        }

        // Simulate training time based on epochs and complexity
        let base_training_time = self.epochs * 50; // Base time per epoch
        let gpu_factor = if self.gpu_enabled.unwrap_or(false) {
            0.3
        } else {
            1.0
        };
        let training_time = ((base_training_time as f64 * gpu_factor) as u32).min(5000);

        tokio::time::sleep(tokio::time::Duration::from_millis(training_time as u64)).await;

        #[derive(Serialize)]
        struct MLTrainingResponse {
            status: String,
            model_name: String,
            model_type: String,
            epochs: u32,
            learning_rate: f64,
            training_time_ms: u32,
            final_accuracy: f64,
            validation_accuracy: Option<f64>,
            model_size_mb: f64,
            model_path: String,
        }

        let final_accuracy = 0.85 + (self.learning_rate * 0.1).min(0.1); // Mock accuracy
        let validation_accuracy = self.validation_split.map(|_| final_accuracy - 0.02);

        let response = MLTrainingResponse {
            status: "trained".to_string(),
            model_name: self.model_name.clone(),
            model_type: self.model_type.clone(),
            epochs: self.epochs,
            learning_rate: self.learning_rate,
            training_time_ms: training_time,
            final_accuracy,
            validation_accuracy,
            model_size_mb: 15.7, // Mock model size
            model_path: format!("models/{}.pkl", self.model_name),
        };

        Ok(rmp_serde::to_vec(&response)?)
    }

    fn name(&self) -> &str {
        // This name won't be used because we specified "ml_training" in the register_task attribute
        "ml_training_task"
    }

    fn timeout_seconds(&self) -> u64 {
        // Dynamic timeout based on epochs and complexity
        let base_timeout = 3600; // 1 hour base
        let epoch_factor = (self.epochs / 10).max(1);
        base_timeout + (epoch_factor as u64 * 600) // +10 minutes per 10 epochs
    }

    fn max_retries(&self) -> u32 {
        1 // Training is expensive, don't retry often
    }
}

/// Comprehensive file processing task
#[derive(Debug, Serialize, Deserialize, Default, AutoRegisterTask)]
pub struct FileProcessingTask {
    pub file_path: String,
    pub operation: String, // "convert", "compress", "extract", "validate", "analyze"
    pub input_format: String,
    pub output_format: Option<String>,
    pub compression_level: Option<u8>, // 1-9 for compression tasks
    pub validation_rules: Option<Vec<FileValidationRule>>,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileValidationRule {
    pub rule_type: String, // "size", "format", "schema", "virus_scan"
    pub parameters: std::collections::HashMap<String, String>,
}

#[async_trait]
impl Task for FileProcessingTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        println!(
            "[PROCESSING] Processing file: {} ({} -> {})",
            self.file_path,
            self.input_format,
            self.output_format.as_deref().unwrap_or("same")
        );
        println!("  Operation: {}", self.operation);

        if let Some(compression_level) = self.compression_level {
            println!("  Compression level: {}", compression_level);
        }

        if let Some(validation_rules) = &self.validation_rules {
            println!("  Validation rules: {} checks", validation_rules.len());
        }

        // Simulate processing time based on operation complexity
        let processing_time = match self.operation.as_str() {
            "validate" => 200,
            "compress" => 500,
            "convert" => 800,
            "extract" => 300,
            "analyze" => 1000,
            _ => 400,
        };

        tokio::time::sleep(tokio::time::Duration::from_millis(processing_time)).await;

        #[derive(Serialize)]
        struct FileProcessingResponse {
            status: String,
            input_file: String,
            output_file: String,
            operation: String,
            input_size_bytes: u64,
            output_size_bytes: u64,
            compression_ratio: Option<f64>,
            processing_time_ms: u64,
            validation_passed: bool,
            checksum: String,
        }

        let input_size = 1024 * 1024; // Mock 1MB file
        let output_size = match self.operation.as_str() {
            "compress" => input_size / 3,
            "extract" => input_size * 2,
            _ => input_size,
        };

        let response = FileProcessingResponse {
            status: "processed".to_string(),
            input_file: self.file_path.clone(),
            output_file: format!("{}.processed", self.file_path),
            operation: self.operation.clone(),
            input_size_bytes: input_size,
            output_size_bytes: output_size,
            compression_ratio: if self.operation == "compress" {
                Some(input_size as f64 / output_size as f64)
            } else {
                None
            },
            processing_time_ms: processing_time,
            validation_passed: true, // Mock validation
            checksum: format!("sha256:{}", Uuid::new_v4()),
        };

        Ok(rmp_serde::to_vec(&response)?)
    }

    fn name(&self) -> &str {
        "file_processing_task"
    }

    fn timeout_seconds(&self) -> u64 {
        match self.operation.as_str() {
            "validate" => 60,
            "compress" => 300,
            "convert" => 600,
            "extract" => 180,
            "analyze" => 900,
            _ => 240,
        }
    }

    fn max_retries(&self) -> u32 {
        3
    }
}

/// Image processing task with advanced features
#[derive(Debug, Serialize, Deserialize, Default, AutoRegisterTask)]
pub struct ImageProcessingTask {
    pub image_path: String,
    pub operations: Vec<ImageOperation>,
    pub output_format: Option<String>, // "jpg", "png", "webp", "avif"
    pub quality: Option<u8>,           // 1-100 for lossy formats
    pub progressive: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageOperation {
    pub operation_type: String, // "resize", "crop", "rotate", "filter", "watermark"
    pub parameters: std::collections::HashMap<String, String>,
}

#[async_trait]
impl Task for ImageProcessingTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        println!("[PROCESSING] Processing image: {}", self.image_path);
        println!("  Operations: {} to perform", self.operations.len());

        if let Some(output_format) = &self.output_format {
            println!("  Output format: {}", output_format);
        }

        if let Some(quality) = self.quality {
            println!("  Quality: {}%", quality);
        }

        // Simulate processing time based on number of operations
        let processing_time = self.operations.len() * 150 + 100;
        tokio::time::sleep(tokio::time::Duration::from_millis(processing_time as u64)).await;

        #[derive(Serialize)]
        struct ImageProcessingResponse {
            status: String,
            input_image: String,
            output_image: String,
            operations_performed: usize,
            original_dimensions: (u32, u32),
            final_dimensions: (u32, u32),
            original_size_bytes: u64,
            final_size_bytes: u64,
            processing_time_ms: usize,
        }

        let response = ImageProcessingResponse {
            status: "processed".to_string(),
            input_image: self.image_path.clone(),
            output_image: format!("{}.processed.jpg", self.image_path),
            operations_performed: self.operations.len(),
            original_dimensions: (1920, 1080), // Mock dimensions
            final_dimensions: (800, 600),      // Mock final dimensions
            original_size_bytes: 2048 * 1024,  // Mock 2MB
            final_size_bytes: 512 * 1024,      // Mock 512KB
            processing_time_ms: processing_time,
        };

        Ok(rmp_serde::to_vec(&response)?)
    }

    fn name(&self) -> &str {
        "image_processing_task"
    }

    fn timeout_seconds(&self) -> u64 {
        60 + (self.operations.len() as u64 * 10) // Base + 10s per operation
    }

    fn max_retries(&self) -> u32 {
        3
    }
}

/// Report generation task
#[derive(Debug, Serialize, Deserialize, Default, AutoRegisterTask)]
pub struct ReportGenerationTask {
    pub report_name: String,
    pub report_type: String, // "pdf", "excel", "html", "csv"
    pub data_sources: Vec<String>,
    pub template: Option<String>,
    pub filters: Option<std::collections::HashMap<String, String>>,
    pub date_range: Option<DateRange>,
    pub recipients: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DateRange {
    pub start: chrono::DateTime<chrono::Utc>,
    pub end: chrono::DateTime<chrono::Utc>,
}

#[async_trait]
impl Task for ReportGenerationTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        println!(
            "[PROCESSING] Generating {} report: {}",
            self.report_type, self.report_name
        );
        println!("  Data sources: {} tables/queries", self.data_sources.len());

        if let Some(template) = &self.template {
            println!("  Template: {}", template);
        }

        if let Some(date_range) = &self.date_range {
            println!("  Date range: {} to {}", date_range.start, date_range.end);
        }

        // Simulate report generation time
        let base_time = 1000; // Base 1 second
        let data_factor = self.data_sources.len() * 200; // 200ms per data source
        let processing_time = base_time + data_factor;

        tokio::time::sleep(tokio::time::Duration::from_millis(processing_time as u64)).await;

        #[derive(Serialize)]
        struct ReportGenerationResponse {
            status: String,
            report_name: String,
            report_type: String,
            file_path: String,
            file_size_bytes: u64,
            pages: Option<u32>,
            rows: Option<u32>,
            generation_time_ms: usize,
            recipients_notified: usize,
        }

        let response = ReportGenerationResponse {
            status: "generated".to_string(),
            report_name: self.report_name.clone(),
            report_type: self.report_type.clone(),
            file_path: format!("reports/{}.{}", self.report_name, self.report_type),
            file_size_bytes: 1024 * 1024, // Mock 1MB report
            pages: if self.report_type == "pdf" {
                Some(25)
            } else {
                None
            },
            rows: if self.report_type == "csv" {
                Some(10000)
            } else {
                None
            },
            generation_time_ms: processing_time,
            recipients_notified: self.recipients.as_ref().map_or(0, |r| r.len()),
        };

        Ok(rmp_serde::to_vec(&response)?)
    }

    fn name(&self) -> &str {
        "report_generation_task"
    }

    fn timeout_seconds(&self) -> u64 {
        300 + (self.data_sources.len() as u64 * 60) // 5 minutes + 1 minute per data source
    }

    fn max_retries(&self) -> u32 {
        2
    }
}

/// Backup task for data archival
#[derive(Debug, Serialize, Deserialize, Default, AutoRegisterTask)]
pub struct BackupTask {
    pub backup_name: String,
    pub source_paths: Vec<String>,
    pub destination: String,
    pub backup_type: String, // "full", "incremental", "differential"
    pub compression: Option<bool>,
    pub encryption: Option<bool>,
    pub retention_days: Option<u32>,
}

#[async_trait]
impl Task for BackupTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        println!(
            "[PROCESSING] Starting {} backup: {}",
            self.backup_type, self.backup_name
        );
        println!("  Sources: {} paths", self.source_paths.len());
        println!("  Destination: {}", self.destination);

        if self.compression.unwrap_or(false) {
            println!("  Compression: enabled");
        }

        if self.encryption.unwrap_or(false) {
            println!("  Encryption: enabled");
        }

        // Simulate backup time based on type and options
        let base_time = match self.backup_type.as_str() {
            "full" => 3000,
            "incremental" => 800,
            "differential" => 1500,
            _ => 1000,
        };

        let compression_factor = if self.compression.unwrap_or(false) {
            1.5
        } else {
            1.0
        };
        let encryption_factor = if self.encryption.unwrap_or(false) {
            1.2
        } else {
            1.0
        };

        let processing_time = (base_time as f64 * compression_factor * encryption_factor) as u64;
        tokio::time::sleep(tokio::time::Duration::from_millis(processing_time)).await;

        #[derive(Serialize)]
        struct BackupResponse {
            status: String,
            backup_name: String,
            backup_type: String,
            files_backed_up: u32,
            total_size_bytes: u64,
            compressed_size_bytes: Option<u64>,
            backup_location: String,
            checksum: String,
            processing_time_ms: u64,
        }

        let total_size = 1024 * 1024 * 500; // Mock 500MB
        let compressed_size = if self.compression.unwrap_or(false) {
            Some(total_size / 3)
        } else {
            None
        };

        let response = BackupResponse {
            status: "completed".to_string(),
            backup_name: self.backup_name.clone(),
            backup_type: self.backup_type.clone(),
            files_backed_up: 1250, // Mock file count
            total_size_bytes: total_size,
            compressed_size_bytes: compressed_size,
            backup_location: format!("{}/{}.backup", self.destination, self.backup_name),
            checksum: format!("sha256:{}", Uuid::new_v4()),
            processing_time_ms: processing_time,
        };

        Ok(rmp_serde::to_vec(&response)?)
    }

    fn name(&self) -> &str {
        "backup_task"
    }

    fn timeout_seconds(&self) -> u64 {
        // Dynamic timeout based on backup type
        match self.backup_type.as_str() {
            "full" => 3600,         // 1 hour for full backup
            "incremental" => 900,   // 15 minutes for incremental
            "differential" => 1800, // 30 minutes for differential
            _ => 1200,
        }
    }

    fn max_retries(&self) -> u32 {
        2 // Backups are critical but expensive
    }
}

/// Database maintenance task
#[derive(Debug, Serialize, Deserialize, Default, AutoRegisterTask)]
pub struct DatabaseMaintenanceTask {
    pub database_name: String,
    pub operations: Vec<String>, // "vacuum", "reindex", "analyze", "optimize", "cleanup"
    pub tables: Option<Vec<String>>, // Specific tables or all
    pub maintenance_window: Option<chrono::DateTime<chrono::Utc>>,
    pub max_duration_minutes: Option<u32>,
}

#[async_trait]
impl Task for DatabaseMaintenanceTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        println!(
            "[PROCESSING] Database maintenance for: {}",
            self.database_name
        );
        println!("  Operations: {:?}", self.operations);

        if let Some(tables) = &self.tables {
            println!("  Tables: {} specified", tables.len());
        } else {
            println!("  Tables: all tables");
        }

        if let Some(window) = &self.maintenance_window {
            println!("  Scheduled for: {}", window);
        }

        // Simulate maintenance time based on operations
        let operation_time: u64 = self
            .operations
            .iter()
            .map(|op| match op.as_str() {
                "vacuum" => 2000,
                "reindex" => 1500,
                "analyze" => 800,
                "optimize" => 3000,
                "cleanup" => 1000,
                _ => 1200,
            })
            .sum();

        tokio::time::sleep(tokio::time::Duration::from_millis(operation_time)).await;

        #[derive(Serialize)]
        struct DatabaseMaintenanceResponse {
            status: String,
            database_name: String,
            operations_completed: Vec<String>,
            tables_affected: u32,
            space_reclaimed_bytes: u64,
            performance_improvement: f64,
            maintenance_time_ms: u64,
        }

        let response = DatabaseMaintenanceResponse {
            status: "completed".to_string(),
            database_name: self.database_name.clone(),
            operations_completed: self.operations.clone(),
            tables_affected: self.tables.as_ref().map_or(50, |t| t.len() as u32), // Mock table count
            space_reclaimed_bytes: 1024 * 1024 * 128, // Mock 128MB reclaimed
            performance_improvement: 15.5,            // Mock 15.5% improvement
            maintenance_time_ms: operation_time,
        };

        Ok(rmp_serde::to_vec(&response)?)
    }

    fn name(&self) -> &str {
        "database_maintenance_task"
    }

    fn timeout_seconds(&self) -> u64 {
        self.max_duration_minutes.unwrap_or(60) as u64 * 60
    }

    fn max_retries(&self) -> u32 {
        1 // Database maintenance should not be retried automatically
    }
}

/// Cache warming task for performance optimization
#[derive(Debug, Serialize, Deserialize, Default, AutoRegisterTask)]
pub struct CacheWarmupTask {
    pub cache_name: String,
    pub cache_type: String,      // "redis", "memcached", "application", "cdn"
    pub warmup_strategy: String, // "popular_items", "recent_items", "predictive", "full"
    pub data_sources: Vec<String>,
    pub max_items: Option<u32>,
    pub ttl_seconds: Option<u32>,
}

#[async_trait]
impl Task for CacheWarmupTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        println!(
            "[PROCESSING] Cache warmup for {} ({})",
            self.cache_name, self.cache_type
        );
        println!("  Strategy: {}", self.warmup_strategy);
        println!("  Data sources: {} configured", self.data_sources.len());

        if let Some(max_items) = self.max_items {
            println!("  Max items: {}", max_items);
        }

        // Simulate cache warming time based on strategy and item count
        let items_to_cache = self.max_items.unwrap_or(1000);
        let strategy_factor = match self.warmup_strategy.as_str() {
            "popular_items" => 0.5,
            "recent_items" => 0.3,
            "predictive" => 1.2,
            "full" => 2.0,
            _ => 1.0,
        };

        let processing_time = (items_to_cache as f64 * strategy_factor * 0.5) as u64; // 0.5ms per item
        tokio::time::sleep(tokio::time::Duration::from_millis(processing_time)).await;

        #[derive(Serialize)]
        struct CacheWarmupResponse {
            status: String,
            cache_name: String,
            cache_type: String,
            warmup_strategy: String,
            items_cached: u32,
            cache_hit_rate_before: f64,
            cache_hit_rate_after: f64,
            total_cache_size_bytes: u64,
            warmup_time_ms: u64,
        }

        let response = CacheWarmupResponse {
            status: "completed".to_string(),
            cache_name: self.cache_name.clone(),
            cache_type: self.cache_type.clone(),
            warmup_strategy: self.warmup_strategy.clone(),
            items_cached: items_to_cache,
            cache_hit_rate_before: 0.65, // Mock 65% before
            cache_hit_rate_after: 0.89,  // Mock 89% after
            total_cache_size_bytes: items_to_cache as u64 * 2048, // Mock 2KB per item
            warmup_time_ms: processing_time,
        };

        Ok(rmp_serde::to_vec(&response)?)
    }

    fn name(&self) -> &str {
        "cache_warmup_task"
    }

    fn timeout_seconds(&self) -> u64 {
        let items = self.max_items.unwrap_or(1000) as u64;
        60 + (items / 100) // 1 minute + 1 second per 100 items
    }

    fn max_retries(&self) -> u32 {
        3
    }
}

/// Batch processing task for handling multiple items at once
#[derive(Debug, Serialize, Deserialize, Default, AutoRegisterTask)]
pub struct BatchProcessingTask {
    pub batch_id: String,
    pub items: Vec<BatchItem>,
    pub operation: String,
    pub parallel_workers: Option<u8>,
    pub error_threshold: Option<f64>, // Stop if error rate exceeds this
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatchItem {
    pub id: String,
    pub data: String, // Base64 encoded item-specific data
    pub priority: Option<u8>,
}

#[async_trait]
impl Task for BatchProcessingTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        println!(
            "[PROCESSING] Batch processing: {} ({} items)",
            self.batch_id,
            self.items.len()
        );
        println!("  Operation: {}", self.operation);

        if let Some(workers) = self.parallel_workers {
            println!("  Parallel workers: {}", workers);
        }

        let workers = self.parallel_workers.unwrap_or(1);
        let items_per_worker = (self.items.len() as f64 / workers as f64).ceil() as usize;
        let processing_time = items_per_worker * 10; // 10ms per item

        tokio::time::sleep(tokio::time::Duration::from_millis(processing_time as u64)).await;

        #[derive(Serialize)]
        struct BatchProcessingResponse {
            status: String,
            batch_id: String,
            operation: String,
            total_items: usize,
            successful_items: usize,
            failed_items: usize,
            error_rate: f64,
            processing_time_ms: usize,
            parallel_workers: u8,
        }

        let failed_items = self.items.len() / 20; // Mock 5% failure rate
        let successful_items = self.items.len() - failed_items;
        let error_rate = failed_items as f64 / self.items.len() as f64;

        let response = BatchProcessingResponse {
            status: "completed".to_string(),
            batch_id: self.batch_id.clone(),
            operation: self.operation.clone(),
            total_items: self.items.len(),
            successful_items,
            failed_items,
            error_rate,
            processing_time_ms: processing_time,
            parallel_workers: workers,
        };

        Ok(rmp_serde::to_vec(&response)?)
    }

    fn name(&self) -> &str {
        "batch_processing_task"
    }

    fn timeout_seconds(&self) -> u64 {
        let items = self.items.len() as u64;
        let workers = self.parallel_workers.unwrap_or(1) as u64;
        300 + (items / workers) // 5 minutes + 1 second per item per worker
    }

    fn max_retries(&self) -> u32 {
        2
    }
}
