//! Configuration management for the task queue framework
//!
//! This module provides a comprehensive configuration system that supports:
//! - Environment variables
//! - TOML and YAML configuration files
//! - Global configuration initialization
//! - Integration with Actix Web

use crate::{AutoScalerConfig, TaskQueueError};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Global configuration instance
static GLOBAL_CONFIG: OnceCell<TaskQueueConfig> = OnceCell::new();

/// Comprehensive task queue configuration
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
#[derive(Default)]
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
            initial_count: std::env::var("TASK_QUEUE_WORKERS")
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
            enabled: std::env::var("TASK_QUEUE_AUTO_REGISTER")
                .map(|s| s.to_lowercase() == "true")
                .unwrap_or(true),
        }
    }
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: std::env::var("TASK_QUEUE_SCHEDULER")
                .map(|s| s.to_lowercase() == "true")
                .unwrap_or(true),
            tick_interval: None,
            max_tasks_per_tick: None,
        }
    }
}

#[cfg(feature = "actix-integration")]
impl Default for ActixConfig {
    fn default() -> Self {
        Self {
            auto_configure_routes: std::env::var("TASK_QUEUE_AUTO_ROUTES")
                .map(|s| s.to_lowercase() == "true")
                .unwrap_or(true),
            route_prefix: std::env::var("TASK_QUEUE_ROUTE_PREFIX")
                .unwrap_or_else(|_| "/tasks".to_string()),
            enable_metrics: true,
            enable_health_check: true,
        }
    }
}

impl TaskQueueConfig {
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

        Ok(config)
    }

    /// Load configuration from a file (TOML or YAML based on extension)
    #[cfg(feature = "config-support")]
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, TaskQueueError> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|e| {
            TaskQueueError::Configuration(format!("Failed to read config file: {}", e))
        })?;

        match path.extension().and_then(|s| s.to_str()) {
            Some("toml") => toml::from_str(&content)
                .map_err(|e| TaskQueueError::Configuration(format!("Invalid TOML config: {}", e))),
            Some("yaml") | Some("yml") => serde_yaml::from_str(&content)
                .map_err(|e| TaskQueueError::Configuration(format!("Invalid YAML config: {}", e))),
            _ => Err(TaskQueueError::Configuration(
                "Unsupported config file format. Use .toml, .yaml, or .yml".to_string(),
            )),
        }
    }

    /// Load configuration with priority: file -> environment -> defaults
    #[cfg(feature = "config-support")]
    pub fn load() -> Result<Self, TaskQueueError> {
        use config::{Config, Environment, File};

        let mut builder = Config::builder()
            // Start with defaults
            .add_source(Config::try_from(&Self::default()).unwrap());

        // Check for config file in common locations
        for config_path in &[
            "task-queue.toml",
            "task-queue.yaml",
            "task-queue.yml",
            "config/task-queue.toml",
            "config/task-queue.yaml",
            "config/task-queue.yml",
        ] {
            if Path::new(config_path).exists() {
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

        config.try_deserialize().map_err(|e| {
            TaskQueueError::Configuration(format!("Failed to deserialize config: {}", e))
        })
    }

    /// Load configuration without the config crate (fallback)
    #[cfg(not(feature = "config-support"))]
    pub fn load() -> Result<Self, TaskQueueError> {
        Self::from_env()
    }

    /// Initialize global configuration
    pub fn init_global() -> Result<&'static Self, TaskQueueError> {
        GLOBAL_CONFIG.get_or_try_init(Self::load)
    }

    /// Get global configuration (must be initialized first)
    pub fn global() -> Option<&'static Self> {
        GLOBAL_CONFIG.get()
    }

    /// Get global configuration, initializing if needed
    pub fn get_or_init() -> Result<&'static Self, TaskQueueError> {
        match Self::global() {
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
