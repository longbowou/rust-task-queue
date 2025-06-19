//! Configuration management for the task queue framework
//!
//! This module provides a comprehensive configuration system that supports:
//! - Environment variables
//! - TOML and YAML configuration files
//! - Global configuration initialization
//! - Integration with Actix Web

use crate::autoscaler::AutoScalerConfig;
use crate::TaskQueueError;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Global configuration instance
static GLOBAL_CONFIG: OnceLock<TaskQueueConfig> = OnceLock::new();

/// Main configuration structure for the task queue system
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskQueueConfig {
    /// Redis connection configuration
    pub redis: RedisConfig,

    /// Worker configuration
    pub workers: WorkerConfig,

    /// Auto-scaling configuration
    pub autoscaler: AutoScalerConfig,

    /// Auto-registration settings
    pub auto_register: AutoRegisterConfig,

    /// Scheduler configuration
    pub scheduler: SchedulerConfig,

    /// Actix Web integration settings
    #[cfg(feature = "actix-integration")]
    pub actix: ActixConfig,

    /// Axum integration settings
    #[cfg(feature = "axum-integration")]
    pub axum: AxumConfig,
}

/// Redis connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis connection URL
    pub url: String,

    /// Connection pool size
    pub pool_size: Option<u32>,

    /// Connection timeout in seconds
    pub connection_timeout: Option<u64>,

    /// Command timeout in seconds
    pub command_timeout: Option<u64>,
}

/// Worker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    /// Initial number of workers to start
    pub initial_count: usize,

    /// Maximum number of concurrent tasks per worker
    pub max_concurrent_tasks: Option<usize>,

    /// Worker heartbeat interval in seconds
    pub heartbeat_interval: Option<u64>,

    /// Grace period for shutdown in seconds
    pub shutdown_grace_period: Option<u64>,
}

/// Auto-registration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoRegisterConfig {
    /// Enable automatic task registration
    pub enabled: bool,
}

/// Scheduler configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    /// Enable the scheduler
    pub enabled: bool,

    /// Scheduler tick interval in seconds
    pub tick_interval: Option<u64>,

    /// Maximum number of scheduled tasks to process per tick
    pub max_tasks_per_tick: Option<usize>,
}

/// Actix Web integration configuration
#[cfg(feature = "actix-integration")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActixConfig {
    /// Enable automatic route configuration
    pub auto_configure_routes: bool,

    /// Base path for task queue routes
    pub route_prefix: String,

    /// Enable metrics endpoint
    pub enable_metrics: bool,

    /// Enable health check endpoint
    pub enable_health_check: bool,
}

/// Axum integration configuration
#[cfg(feature = "axum-integration")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxumConfig {
    /// Enable automatic route configuration
    pub auto_configure_routes: bool,

    /// Base path for task queue routes
    pub route_prefix: String,

    /// Enable metrics endpoint
    pub enable_metrics: bool,

    /// Enable health check endpoint
    pub enable_health_check: bool
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
            pool_size: None,
            connection_timeout: None,
            command_timeout: None,
        }
    }
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            initial_count: std::env::var("INITIAL_WORKER_COUNT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(2),
            max_concurrent_tasks: None,
            heartbeat_interval: None,
            shutdown_grace_period: None,
        }
    }
}

impl Default for AutoRegisterConfig {
    fn default() -> Self {
        Self {
            enabled: std::env::var("AUTO_REGISTER_TASKS")
                .map(|s| s.to_lowercase() == "true")
                .unwrap_or(false),
        }
    }
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: std::env::var("ENABLE_SCHEDULER")
                .map(|s| s.to_lowercase() == "true")
                .unwrap_or(false),
            tick_interval: None,
            max_tasks_per_tick: None,
        }
    }
}

#[cfg(feature = "actix-integration")]
impl Default for ActixConfig {
    fn default() -> Self {
        Self {
            auto_configure_routes: std::env::var("AUTO_CONFIGURE_ROUTES")
                .map(|s| s.to_lowercase() == "true")
                .unwrap_or(true),
            route_prefix: std::env::var("ROUTE_PREFIX")
                .unwrap_or_else(|_| "/api/v1/tasks".to_string()),
            enable_metrics: std::env::var("ENABLE_METRICS")
                .map(|s| s.to_lowercase() == "true")
                .unwrap_or(true),
            enable_health_check: std::env::var("ENABLE_HEALTH_CHECK")
                .map(|s| s.to_lowercase() == "true")
                .unwrap_or(true),
        }
    }
}

#[cfg(feature = "axum-integration")]
impl Default for AxumConfig {
    fn default() -> Self {
        Self {
            auto_configure_routes: std::env::var("AXUM_AUTO_CONFIGURE_ROUTES")
                .map(|s| s.to_lowercase() == "true")
                .unwrap_or(true),
            route_prefix: std::env::var("AXUM_ROUTE_PREFIX")
                .unwrap_or_else(|_| "/api/v1/tasks".to_string()),
            enable_metrics: std::env::var("AXUM_ENABLE_METRICS")
                .map(|s| s.to_lowercase() == "true")
                .unwrap_or(true),
            enable_health_check: std::env::var("AXUM_ENABLE_HEALTH_CHECK")
                .map(|s| s.to_lowercase() == "true")
                .unwrap_or(true),
        }
    }
}

impl TaskQueueConfig {
    /// Validate the entire configuration
    pub fn validate(&self) -> Result<(), TaskQueueError> {
        // Validate Redis URL
        if self.redis.url.is_empty() {
            return Err(TaskQueueError::Configuration(
                "Redis URL cannot be empty".to_string(),
            ));
        }

        // Basic URL format validation
        if !self.redis.url.starts_with("redis://") && !self.redis.url.starts_with("rediss://") {
            return Err(TaskQueueError::Configuration(
                "Redis URL must start with redis:// or rediss://".to_string(),
            ));
        }

        // Validate worker configuration
        if self.workers.initial_count == 0 {
            return Err(TaskQueueError::Configuration(
                "Initial worker count must be greater than 0".to_string(),
            ));
        }

        if self.workers.initial_count > 1000 {
            return Err(TaskQueueError::Configuration(
                "Initial worker count cannot exceed 1000".to_string(),
            ));
        }

        if let Some(max_concurrent) = self.workers.max_concurrent_tasks {
            if max_concurrent == 0 || max_concurrent > 1000 {
                return Err(TaskQueueError::Configuration(
                    "Max concurrent tasks per worker must be between 1 and 1000".to_string(),
                ));
            }
        }

        if let Some(heartbeat) = self.workers.heartbeat_interval {
            if heartbeat == 0 || heartbeat > 3600 {
                return Err(TaskQueueError::Configuration(
                    "Heartbeat interval must be between 1 and 3600 seconds".to_string(),
                ));
            }
        }

        if let Some(grace_period) = self.workers.shutdown_grace_period {
            if grace_period > 300 {
                return Err(TaskQueueError::Configuration(
                    "Shutdown grace period cannot exceed 300 seconds".to_string(),
                ));
            }
        }

        // Validate pool size
        if let Some(pool_size) = self.redis.pool_size {
            if pool_size == 0 || pool_size > 1000 {
                return Err(TaskQueueError::Configuration(
                    "Redis pool size must be between 1 and 1000".to_string(),
                ));
            }
        }

        // Validate timeouts
        if let Some(timeout) = self.redis.connection_timeout {
            if timeout == 0 || timeout > 300 {
                return Err(TaskQueueError::Configuration(
                    "Connection timeout must be between 1 and 300 seconds".to_string(),
                ));
            }
        }

        if let Some(timeout) = self.redis.command_timeout {
            if timeout == 0 || timeout > 300 {
                return Err(TaskQueueError::Configuration(
                    "Command timeout must be between 1 and 300 seconds".to_string(),
                ));
            }
        }

        // Validate scheduler configuration
        if let Some(tick_interval) = self.scheduler.tick_interval {
            if tick_interval == 0 || tick_interval > 3600 {
                return Err(TaskQueueError::Configuration(
                    "Scheduler tick interval must be between 1 and 3600 seconds".to_string(),
                ));
            }
        }

        if let Some(max_tasks) = self.scheduler.max_tasks_per_tick {
            if max_tasks == 0 || max_tasks > 10000 {
                return Err(TaskQueueError::Configuration(
                    "Max tasks per tick must be between 1 and 10000".to_string(),
                ));
            }
        }

        // Validate autoscaler configuration
        self.autoscaler.validate()?;

        // Validate Actix configuration
        #[cfg(feature = "actix-integration")]
        {
            if self.actix.route_prefix.is_empty() {
                return Err(TaskQueueError::Configuration(
                    "Actix route prefix cannot be empty".to_string(),
                ));
            }

            if !self.actix.route_prefix.starts_with('/') {
                return Err(TaskQueueError::Configuration(
                    "Actix route prefix must start with '/'".to_string(),
                ));
            }
        }

        // Validate Axum configuration
        #[cfg(feature = "axum-integration")]
        {
            if self.axum.route_prefix.is_empty() {
                return Err(TaskQueueError::Configuration(
                    "Axum route prefix cannot be empty".to_string(),
                ));
            }

            if !self.axum.route_prefix.starts_with('/') {
                return Err(TaskQueueError::Configuration(
                    "Axum route prefix must start with '/'".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Load configuration from environment variables only
    pub fn from_env() -> Result<Self, TaskQueueError> {
        let mut config = Self::default();

        // Load additional environment variables that aren't in defaults
        if let Ok(pool_size) = std::env::var("REDIS_POOL_SIZE") {
            config.redis.pool_size = Some(pool_size.parse().map_err(|_| {
                TaskQueueError::Configuration("Invalid REDIS_POOL_SIZE".to_string())
            })?);
        }

        if let Ok(timeout) = std::env::var("REDIS_CONNECTION_TIMEOUT") {
            config.redis.connection_timeout = Some(timeout.parse().map_err(|_| {
                TaskQueueError::Configuration("Invalid REDIS_CONNECTION_TIMEOUT".to_string())
            })?);
        }

        // Validate the configuration
        config.validate()?;

        Ok(config)
    }

    /// Load configuration from a file (TOML or YAML based on extension)
    #[cfg(feature = "config-support")]
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, TaskQueueError> {
        let path = path.as_ref();
        let contents = std::fs::read_to_string(path).map_err(|e| {
            TaskQueueError::Configuration(format!("Failed to read config file: {}", e))
        })?;

        let config: TaskQueueConfig = if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            toml::from_str(&contents).map_err(|e| {
                TaskQueueError::Configuration(format!("Failed to parse TOML config: {}", e))
            })?
        } else {
            serde_yaml::from_str(&contents).map_err(|e| {
                TaskQueueError::Configuration(format!("Failed to parse YAML config: {}", e))
            })?
        };

        // Validate the configuration
        config.validate()?;

        Ok(config)
    }

    /// Load configuration with automatic source detection and validation
    #[cfg(feature = "config-support")]
    pub fn load() -> Result<Self, TaskQueueError> {
        use config::{Config, Environment, File};

        let mut builder = Config::builder()
            // Start with defaults
            .add_source(Config::try_from(&Self::default()).map_err(|e| {
                TaskQueueError::Configuration(format!("Failed to create default config: {}", e))
            })?);

        // Check for config file in common locations
        for config_path in &[
            "task-queue.toml",
            "task-queue.yaml",
            "task-queue.yml",
            "config/task-queue.toml",
            "config/task-queue.yaml",
            "config/task-queue.yml",
        ] {
            if std::path::Path::new(config_path).exists() {
                builder = builder.add_source(File::with_name(config_path));
                break;
            }
        }

        // Override with environment variables (prefixed with TASK_QUEUE_)
        builder = builder.add_source(
            Environment::with_prefix("TASK_QUEUE")
                .separator("_")
                .try_parsing(true),
        );

        // Also support unprefixed common variables like REDIS_URL
        if let Ok(redis_url) = std::env::var("REDIS_URL") {
            builder = builder.set_override("redis.url", redis_url).map_err(|e| {
                TaskQueueError::Configuration(format!("Failed to set REDIS_URL override: {}", e))
            })?;
        }

        let config = builder
            .build()
            .map_err(|e| TaskQueueError::Configuration(format!("Failed to build config: {}", e)))?;

        let config: TaskQueueConfig = config.try_deserialize().map_err(|e| {
            TaskQueueError::Configuration(format!("Failed to deserialize config: {}", e))
        })?;

        // Validate the configuration
        config.validate()?;

        Ok(config)
    }

    /// Load configuration without the config crate (fallback)
    #[cfg(not(feature = "config-support"))]
    pub fn load() -> Result<Self, TaskQueueError> {
        Self::from_env()
    }

    /// Initialize global configuration if not already done
    pub fn init_global() -> Result<&'static Self, TaskQueueError> {
        match GLOBAL_CONFIG.get() {
            Some(config) => Ok(config),
            None => match GLOBAL_CONFIG.set(Self::load()?) {
                Ok(()) => Ok(GLOBAL_CONFIG.get().unwrap()),
                Err(_) => Ok(GLOBAL_CONFIG.get().unwrap()),
            },
        }
    }

    /// Get global configuration (if initialized)
    pub fn global() -> Option<&'static Self> {
        GLOBAL_CONFIG.get()
    }

    /// Get global configuration or initialize it
    pub fn get_or_init() -> Result<&'static Self, TaskQueueError> {
        match GLOBAL_CONFIG.get() {
            Some(config) => Ok(config),
            None => Self::init_global(),
        }
    }
}

/// Configuration builder for fluent API
#[derive(Default)]
pub struct ConfigBuilder {
    config: TaskQueueConfig,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn redis_url(mut self, url: impl Into<String>) -> Self {
        self.config.redis.url = url.into();
        self
    }

    pub fn workers(mut self, count: usize) -> Self {
        self.config.workers.initial_count = count;
        self
    }

    pub fn enable_auto_register(mut self, enabled: bool) -> Self {
        self.config.auto_register.enabled = enabled;
        self
    }

    pub fn enable_scheduler(mut self, enabled: bool) -> Self {
        self.config.scheduler.enabled = enabled;
        self
    }

    pub fn autoscaler_config(mut self, config: AutoScalerConfig) -> Self {
        self.config.autoscaler = config;
        self
    }

    pub fn build(self) -> TaskQueueConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_redis_config_default() {
        let config = RedisConfig::default();

        // Should use environment variable or default
        assert!(!config.url.is_empty());
        assert!(config.url.starts_with("redis://"));
        assert!(config.pool_size.is_none());
        assert!(config.connection_timeout.is_none());
        assert!(config.command_timeout.is_none());
    }

    #[test]
    fn test_worker_config_default() {
        let config = WorkerConfig::default();

        assert!(config.initial_count > 0);
        assert!(config.max_concurrent_tasks.is_none());
        assert!(config.heartbeat_interval.is_none());
        assert!(config.shutdown_grace_period.is_none());
    }

    #[test]
    fn test_auto_register_config_default() {
        let config = AutoRegisterConfig::default();
        // Default should be false unless environment variable is set
        assert!(!config.enabled || env::var("AUTO_REGISTER_TASKS").is_ok());
    }

    #[test]
    fn test_scheduler_config_default() {
        let config = SchedulerConfig::default();
        // Default should be false unless environment variable is set
        assert!(!config.enabled || env::var("ENABLE_SCHEDULER").is_ok());
        assert!(config.tick_interval.is_none());
        assert!(config.max_tasks_per_tick.is_none());
    }

    #[cfg(feature = "actix-integration")]
    #[test]
    fn test_actix_config_default() {
        let config = ActixConfig::default();

        assert!(!config.route_prefix.is_empty());
        assert!(config.route_prefix.starts_with('/'));
        // Other defaults may vary based on environment variables
    }

    #[test]
    fn test_task_queue_config_default() {
        let config = TaskQueueConfig::default();

        assert!(!config.redis.url.is_empty());
        assert!(config.workers.initial_count > 0);
        // Other fields should have sensible defaults
    }

    #[test]
    fn test_config_validation_valid() {
        let config = TaskQueueConfig {
            redis: RedisConfig {
                url: "redis://localhost:6379".to_string(),
                pool_size: Some(10),
                connection_timeout: Some(30),
                command_timeout: Some(60),
            },
            workers: WorkerConfig {
                initial_count: 4,
                max_concurrent_tasks: Some(10),
                heartbeat_interval: Some(30),
                shutdown_grace_period: Some(60),
            },
            autoscaler: AutoScalerConfig::default(),
            auto_register: AutoRegisterConfig { enabled: true },
            scheduler: SchedulerConfig {
                enabled: true,
                tick_interval: Some(60),
                max_tasks_per_tick: Some(100),
            },
            #[cfg(feature = "actix-integration")]
            actix: ActixConfig {
                auto_configure_routes: true,
                route_prefix: "/api/tasks".to_string(),
                enable_metrics: true,
                enable_health_check: true,
            },
            #[cfg(feature = "axum-integration")]
            axum: AxumConfig {
                auto_configure_routes: true,
                route_prefix: "/api/tasks".to_string(),
                enable_metrics: true,
                enable_health_check: true
            },
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_empty_redis_url() {
        let mut config = TaskQueueConfig::default();
        config.redis.url = "".to_string();

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Redis URL cannot be empty"));
    }

    #[test]
    fn test_config_validation_invalid_redis_url() {
        let mut config = TaskQueueConfig::default();
        config.redis.url = "http://localhost:6379".to_string();

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Redis URL must start with redis://"));
    }

    #[test]
    fn test_config_validation_zero_workers() {
        let mut config = TaskQueueConfig::default();
        config.workers.initial_count = 0;

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Initial worker count must be greater than 0"));
    }

    #[test]
    fn test_config_validation_too_many_workers() {
        let mut config = TaskQueueConfig::default();
        config.workers.initial_count = 1001;

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Initial worker count cannot exceed 1000"));
    }

    #[test]
    fn test_config_validation_invalid_pool_size() {
        let mut config = TaskQueueConfig::default();
        config.redis.pool_size = Some(0);

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Redis pool size must be between 1 and 1000"));
    }

    #[test]
    fn test_config_validation_invalid_timeouts() {
        let mut config = TaskQueueConfig::default();
        config.redis.connection_timeout = Some(0);

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Connection timeout must be between 1 and 300"));

        config.redis.connection_timeout = Some(30);
        config.redis.command_timeout = Some(301);

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Command timeout must be between 1 and 300"));
    }

    #[test]
    fn test_config_validation_invalid_worker_settings() {
        let mut config = TaskQueueConfig::default();
        config.workers.max_concurrent_tasks = Some(0);

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Max concurrent tasks per worker must be between 1 and 1000"));

        config.workers.max_concurrent_tasks = Some(10);
        config.workers.heartbeat_interval = Some(0);

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Heartbeat interval must be between 1 and 3600"));

        config.workers.heartbeat_interval = Some(30);
        config.workers.shutdown_grace_period = Some(301);

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Shutdown grace period cannot exceed 300"));
    }

    #[test]
    fn test_config_validation_invalid_scheduler_settings() {
        let mut config = TaskQueueConfig::default();
        config.scheduler.tick_interval = Some(0);

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Scheduler tick interval must be between 1 and 3600"));

        config.scheduler.tick_interval = Some(60);
        config.scheduler.max_tasks_per_tick = Some(0);

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Max tasks per tick must be between 1 and 10000"));
    }

    #[cfg(feature = "actix-integration")]
    #[test]
    fn test_config_validation_invalid_actix_settings() {
        let mut config = TaskQueueConfig::default();
        config.actix.route_prefix = "".to_string();

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Actix route prefix cannot be empty"));

        config.actix.route_prefix = "api/tasks".to_string(); // Missing leading slash

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Actix route prefix must start with '/'"));
    }

    #[cfg(feature = "axum-integration")]
    #[test]
    fn test_config_validation_invalid_axum_settings() {
        let mut config = TaskQueueConfig::default();
        config.axum.route_prefix = "".to_string();

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Axum route prefix cannot be empty"));

        config.axum.route_prefix = "api/tasks".to_string(); // Missing leading slash

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Axum route prefix must start with '/'"));
    }

    #[cfg(feature = "axum-integration")]
    #[test]
    fn test_axum_config_default() {
        let config = AxumConfig::default();

        assert!(!config.route_prefix.is_empty());
        assert!(config.route_prefix.starts_with('/'));
        // Other defaults may vary based on environment variables
    }

    #[test]
    fn test_config_builder() {
        let config = ConfigBuilder::new()
            .redis_url("redis://test:6379")
            .workers(8)
            .enable_auto_register(true)
            .enable_scheduler(true)
            .autoscaler_config(AutoScalerConfig {
                min_workers: 2,
                max_workers: 16,
                scale_up_count: 4,
                scale_down_count: 2,
                scaling_triggers: crate::autoscaler::ScalingTriggers {
                    queue_pressure_threshold: 1.0,
                    worker_utilization_threshold: 0.85,
                    task_complexity_threshold: 1.5,
                    error_rate_threshold: 0.05,
                    memory_pressure_threshold: 512.0,
                },
                enable_adaptive_thresholds: true,
                learning_rate: 0.1,
                adaptation_window_minutes: 30,
                scale_up_cooldown_seconds: 60,
                scale_down_cooldown_seconds: 300,
                consecutive_signals_required: 2,
                target_sla: crate::autoscaler::SLATargets {
                    max_p95_latency_ms: 5000.0,
                    min_success_rate: 0.95,
                    max_queue_wait_time_ms: 10000.0,
                    target_worker_utilization: 0.70,
                },
            })
            .build();

        assert_eq!(config.redis.url, "redis://test:6379");
        assert_eq!(config.workers.initial_count, 8);
        assert!(config.auto_register.enabled);
        assert!(config.scheduler.enabled);
        assert_eq!(config.autoscaler.min_workers, 2);
        assert_eq!(config.autoscaler.max_workers, 16);
    }

    #[test]
    fn test_config_serialization() {
        let config = TaskQueueConfig::default();

        // Test JSON serialization
        let json = serde_json::to_string(&config).expect("Failed to serialize to JSON");
        let deserialized: TaskQueueConfig =
            serde_json::from_str(&json).expect("Failed to deserialize from JSON");

        assert_eq!(config.redis.url, deserialized.redis.url);
        assert_eq!(
            config.workers.initial_count,
            deserialized.workers.initial_count
        );
        assert_eq!(
            config.auto_register.enabled,
            deserialized.auto_register.enabled
        );
        assert_eq!(config.scheduler.enabled, deserialized.scheduler.enabled);
    }

    #[test]
    fn test_config_from_env() {
        // Set environment variables
        env::set_var("REDIS_POOL_SIZE", "15");
        env::set_var("REDIS_CONNECTION_TIMEOUT", "45");

        let config = TaskQueueConfig::from_env().expect("Failed to load config from env");

        assert_eq!(config.redis.pool_size, Some(15));
        assert_eq!(config.redis.connection_timeout, Some(45));

        // Clean up
        env::remove_var("REDIS_POOL_SIZE");
        env::remove_var("REDIS_CONNECTION_TIMEOUT");
    }

    #[test]
    fn test_config_from_env_invalid_values() {
        // Set invalid environment variable
        env::set_var("REDIS_POOL_SIZE", "invalid");

        let result = TaskQueueConfig::from_env();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid REDIS_POOL_SIZE"));

        // Clean up
        env::remove_var("REDIS_POOL_SIZE");
    }

    #[test]
    fn test_config_clone() {
        let original = TaskQueueConfig::default();
        let cloned = original.clone();

        assert_eq!(original.redis.url, cloned.redis.url);
        assert_eq!(original.workers.initial_count, cloned.workers.initial_count);
        assert_eq!(original.auto_register.enabled, cloned.auto_register.enabled);
        assert_eq!(original.scheduler.enabled, cloned.scheduler.enabled);
    }

    #[test]
    fn test_config_debug() {
        let config = TaskQueueConfig::default();
        let debug_str = format!("{:?}", config);

        assert!(debug_str.contains("TaskQueueConfig"));
        assert!(debug_str.contains("redis"));
        assert!(debug_str.contains("workers"));
        assert!(debug_str.contains("autoscaler"));
    }

    #[test]
    fn test_individual_config_structs_clone() {
        let redis_config = RedisConfig::default();
        let cloned_redis = redis_config.clone();
        assert_eq!(redis_config.url, cloned_redis.url);

        let worker_config = WorkerConfig::default();
        let cloned_worker = worker_config.clone();
        assert_eq!(worker_config.initial_count, cloned_worker.initial_count);

        let auto_register_config = AutoRegisterConfig::default();
        let cloned_auto_register = auto_register_config.clone();
        assert_eq!(auto_register_config.enabled, cloned_auto_register.enabled);

        let scheduler_config = SchedulerConfig::default();
        let cloned_scheduler = scheduler_config.clone();
        assert_eq!(scheduler_config.enabled, cloned_scheduler.enabled);
    }

    #[test]
    fn test_config_builder_default() {
        let builder = ConfigBuilder::new();
        let config = builder.build();

        // Should have the same values as TaskQueueConfig::default()
        let default_config = TaskQueueConfig::default();
        assert_eq!(config.redis.url, default_config.redis.url);
        assert_eq!(
            config.workers.initial_count,
            default_config.workers.initial_count
        );
    }

    #[test]
    fn test_config_builder_method_chaining() {
        let config = ConfigBuilder::default()
            .redis_url("redis://chained:6379")
            .workers(5)
            .enable_auto_register(false)
            .enable_scheduler(false)
            .build();

        assert_eq!(config.redis.url, "redis://chained:6379");
        assert_eq!(config.workers.initial_count, 5);
        assert!(!config.auto_register.enabled);
        assert!(!config.scheduler.enabled);
    }

    #[test]
    fn test_redis_url_validation() {
        let test_cases = vec![
            ("redis://localhost:6379", true),
            ("rediss://localhost:6379", true),
            ("redis://user:pass@localhost:6379/0", true),
            ("redis://localhost", true),
            ("http://localhost:6379", false),
            ("localhost:6379", false),
            ("", false),
        ];

        for (url, should_be_valid) in test_cases {
            let mut config = TaskQueueConfig::default();
            config.redis.url = url.to_string();

            let result = config.validate();
            if should_be_valid {
                assert!(result.is_ok(), "URL '{}' should be valid", url);
            } else {
                assert!(result.is_err(), "URL '{}' should be invalid", url);
            }
        }
    }
}
