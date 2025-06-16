//! Consumer Helper Functions
//!
//! This module provides simple utilities for consumer projects to create
//! task workers with minimal boilerplate code.

use crate::config::{ConfigBuilder, TaskQueueConfig};
use crate::prelude::*;
use std::env;

#[cfg(feature = "cli")]
use tracing_subscriber;

/// Start a consumer task worker with the given configuration
///
/// This function automatically discovers tasks from the consumer library
/// and starts the specified number of worker processes.
///
/// # Example
///
/// ```rust,no_run
/// // Import your tasks first (this example shows the pattern)
/// // use my_task_app::*;
/// use rust_task_queue::cli::*;
/// use rust_task_queue::config::TaskQueueConfig;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = TaskQueueConfig::load()?;
///     start_cli_worker(config).await
/// }
/// ```
pub async fn start_cli_worker(config: TaskQueueConfig) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    {
        use tracing_subscriber::{fmt, EnvFilter};
        let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
        fmt()
            .with_env_filter(EnvFilter::new(&log_level))
            .with_target(true)
            .init();
    }

    #[cfg(feature = "tracing")]
    {
        tracing::info!("Starting Consumer Task Worker");
        tracing::info!("Redis URL: {}", config.redis.url);
        tracing::info!("Workers: {}", config.workers.initial_count);
        tracing::info!("Auto-register: {}", config.auto_register.enabled);
        tracing::info!("Scheduler: {}", config.scheduler.enabled);
    }

    // Create task queue with configuration
    let mut task_queue_builder = TaskQueueBuilder::new(&config.redis.url);

    if config.auto_register.enabled {
        task_queue_builder = task_queue_builder.auto_register_tasks();
    }

    let task_queue = task_queue_builder.build().await?;

    // Start workers
    #[cfg(feature = "tracing")]
    tracing::info!("Starting {} workers...", config.workers.initial_count);
    task_queue
        .start_workers(config.workers.initial_count)
        .await?;

    // Show discovered tasks if auto-registration is enabled
    if config.auto_register.enabled {
        let task_registry = TaskRegistry::with_auto_registered()
            .map_err(|e| format!("Failed to create registry: {}", e))?;
        let registered_tasks = task_registry.registered_tasks();
        #[cfg(feature = "tracing")]
        {
            tracing::info!("Auto-discovered {} task types:", registered_tasks.len());
            for task_type in &registered_tasks {
                tracing::info!("   • {}", task_type);
            }
        }
    }

    #[cfg(feature = "tracing")]
    {
        tracing::info!("Workers started successfully!");
        tracing::info!("Listening for tasks on all queues");
        tracing::info!("Press Ctrl+C to shutdown gracefully");
    }

    // Keep running until interrupt
    tokio::signal::ctrl_c().await?;

    #[cfg(feature = "tracing")]
    tracing::info!("Shutting down gracefully...");

    Ok(())
}

/// Start a consumer worker with automatic configuration
///
/// This function loads configuration from:
/// 1. Configuration files (task-queue.toml, task-queue.yaml, etc.)
/// 2. Environment variables
/// 3. Command line arguments
/// 4. Sensible defaults
///
/// # Example
///
/// ```rust,no_run
/// // Import your tasks first (this example shows the pattern)
/// // use my_task_app::*;
/// use rust_task_queue::cli::*;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     start_worker().await
/// }
/// ```
pub async fn start_worker() -> Result<(), Box<dyn std::error::Error>> {
    let config = TaskQueueConfig::load()?;
    start_cli_worker(config).await
}

/// Start a consumer worker with configuration from environment variables only
///
/// This is useful when you want to avoid file-based configuration
/// and only use environment variables.
///
/// # Example
///
/// ```rust,no_run
/// // Import your tasks first (this example shows the pattern)
/// // use my_task_app::*;
/// use rust_task_queue::cli::*;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     start_worker_from_env().await
/// }
/// ```
pub async fn start_worker_from_env() -> Result<(), Box<dyn std::error::Error>> {
    let config = TaskQueueConfig::from_env()?;
    start_cli_worker(config).await
}

/// Start a consumer worker with custom configuration built using the builder pattern
///
/// # Example
///
/// ```rust,no_run
/// // Import your tasks first (this example shows the pattern)
/// // use my_task_app::*;
/// use rust_task_queue::cli::*;
/// use rust_task_queue::config::ConfigBuilder;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = ConfigBuilder::new()
///         .redis_url("redis://localhost:6379")
///         .workers(4)
///         .enable_auto_register(true)
///         .enable_scheduler(true)
///         .build();
///     
///     start_cli_worker(config).await
/// }
/// ```
pub async fn start_worker_with_builder<F>(builder_fn: F) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnOnce(ConfigBuilder) -> ConfigBuilder,
{
    let config = builder_fn(ConfigBuilder::new()).build();
    start_cli_worker(config).await
}

/// Macro to create a complete task worker binary with minimal code
///
/// This macro generates a complete `main.rs` for your task worker binary.
/// It uses the comprehensive configuration system from config.rs.
///
/// # Example
///
/// ```rust,no_run
/// // Import your tasks first (this example shows the pattern)
/// // use my_task_app::*;
/// rust_task_queue::create_worker_main!();
/// ```
#[macro_export]
macro_rules! create_worker_main {
    () => {
        #[tokio::main]
        async fn main() -> Result<(), Box<dyn std::error::Error>> {
            $crate::cli::start_worker().await
        }
    };

    (env) => {
        #[tokio::main]
        async fn main() -> Result<(), Box<dyn std::error::Error>> {
            $crate::cli::start_worker_from_env().await
        }
    };

    ($config:expr) => {
        #[tokio::main]
        async fn main() -> Result<(), Box<dyn std::error::Error>> {
            $crate::cli:start_consumer_workerr($config).await
        }
    };
}

/// Macro to create a task worker with custom configuration using the builder pattern
///
/// # Example
///
/// ```rust,no_run
/// // Import your tasks first (this example shows the pattern)
/// // use my_task_app::*;
/// rust_task_queue::create_worker_with_builder!(|builder| {
///     builder
///         .redis_url("redis://localhost:6379")
///         .workers(4)
///         .enable_auto_register(true)
///         .enable_scheduler(true)
/// });
/// ```
#[macro_export]
macro_rules! create_worker_with_builder {
    ($builder_fn:expr) => {
        #[tokio::main]
        async fn main() -> Result<(), Box<dyn std::error::Error>> {
            $crate::cli::start_worker_with_builder($builder_fn).await
        }
    };
}
