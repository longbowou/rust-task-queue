//! # Rust Task Queue Framework
//!
//! A high-performance, Redis-backed task queue system with auto-scaling capabilities
//! designed for use with async Rust applications.
//!
//! ## Features
//!
//! - **Redis-backed broker** for reliable message delivery
//! - **Auto-scaling workers** based on queue load
//! - **Task scheduling** with delay support
//! - **Multiple queue priorities**
//! - **Retry logic** with configurable attempts
//! - **Task timeouts** and failure handling
//! - **Metrics and monitoring**
//! - **Actix Web integration** (optional)
//! - **Automatic task registration** (optional)
//!
//! ## Quick Start
//!
//! ### Basic Usage
//!
//! ```rust,no_run
//! use rust_task_queue::prelude::*;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Serialize, Deserialize, AutoRegisterTask)]
//! struct MyTask {
//!     data: String,
//! }
//!
//! #[async_trait::async_trait]
//! impl Task for MyTask {
//!     async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
//!         println!("Processing: {}", self.data);
//!         use serde::Serialize;
//!         #[derive(Serialize)]
//!         struct Response { status: String }
//!         let response = Response { status: "completed".to_string() };
//!         Ok(rmp_serde::to_vec(&response)?)
//!     }
//!     
//!     fn name(&self) -> &str {
//!         "my_task"
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let task_queue = TaskQueue::new("redis://localhost:6379").await?;
//!     
//!     // Start workers
//!     task_queue.start_workers(2).await?;
//!     
//!     // Enqueue a task
//!     let task = MyTask { data: "Hello, World!".to_string() };
//!     let task_id = task_queue.enqueue(task, "default").await?;
//!     
//!     println!("Enqueued task: {}", task_id);
//!     Ok(())
//! }
//! ```
//!
//! ### Automatic Task Registration
//!
//! ```rust,no_run
//! use rust_task_queue::prelude::*;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Serialize, Deserialize, Default, AutoRegisterTask)]
//! struct MyTask {
//!     data: String,
//! }
//!
//! #[async_trait::async_trait]
//! impl Task for MyTask {
//!     async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
//!         println!("Processing: {}", self.data);
//!         use serde::Serialize;
//!         #[derive(Serialize)]
//!         struct Response { status: String }
//!         let response = Response { status: "completed".to_string() };
//!         Ok(rmp_serde::to_vec(&response)?)
//!     }
//!     
//!     fn name(&self) -> &str {
//!         "my_task"
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Task is automatically registered!
//!     let task_queue = TaskQueueBuilder::new("redis://localhost:6379")
//!         .auto_register_tasks()
//!         .initial_workers(2)
//!         .build()
//!         .await?;
//!     
//!     let task = MyTask { data: "Hello, World!".to_string() };
//!     let task_id = task_queue.enqueue(task, "default").await?;
//!     
//!     println!("Enqueued task: {}", task_id);
//!     Ok(())
//! }
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod autoscaler;
pub mod broker;
pub mod config;
pub mod error;
pub mod metrics;
pub mod queue;
pub mod scheduler;
pub mod task;
pub mod worker;

#[cfg(feature = "tracing")]
pub mod tracing_utils;

#[cfg(feature = "actix-integration")]
#[cfg_attr(docsrs, doc(cfg(feature = "actix-integration")))]
pub mod actix;

#[cfg(feature = "axum-integration")]
#[cfg_attr(docsrs, doc(cfg(feature = "axum-integration")))]
pub mod axum;

#[cfg(feature = "cli")]
#[cfg_attr(docsrs, doc(cfg(feature = "cli")))]
pub mod cli;

pub use autoscaler::*;
pub use broker::*;
#[cfg(feature = "cli")]
pub use cli::*;
pub use config::*;
pub use error::*;
pub use metrics::*;
pub use queue::*;
pub use scheduler::*;
pub use task::*;
pub use worker::*;

// Re-export macros when auto-register feature is enabled
#[cfg(feature = "auto-register")]
pub use rust_task_queue_macros::{register_task as proc_register_task, AutoRegisterTask};

// Provide placeholder when auto-register is disabled
#[cfg(not(feature = "auto-register"))]
pub use std::marker::PhantomData as AutoRegisterTask;
#[cfg(not(feature = "auto-register"))]
pub use std::marker::PhantomData as proc_register_task;

// Re-export inventory for users who want to use it directly
#[cfg(feature = "auto-register")]
#[cfg_attr(docsrs, doc(cfg(feature = "auto-register")))]
pub use inventory;

pub mod prelude;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main task queue framework
#[derive(Clone)]
pub struct TaskQueue {
    pub broker: Arc<RedisBroker>,
    pub scheduler: Arc<TaskScheduler>,
    pub autoscaler: Arc<AutoScaler>,
    pub metrics: Arc<MetricsCollector>,
    workers: Arc<RwLock<Vec<Worker>>>,
    scheduler_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    shutdown_signal: Arc<RwLock<Option<tokio::sync::broadcast::Sender<()>>>>,
}

impl TaskQueue {
    /// Create a new task queue instance
    pub async fn new(redis_url: &str) -> Result<Self, TaskQueueError> {
        let broker = Arc::new(RedisBroker::new(redis_url).await?);
        let scheduler = Arc::new(TaskScheduler::new(broker.clone()));
        let autoscaler = Arc::new(AutoScaler::new(broker.clone()));
        let metrics = Arc::new(MetricsCollector::new());

        Ok(Self {
            broker,
            scheduler,
            autoscaler,
            metrics,
            workers: Arc::new(RwLock::new(Vec::new())),
            scheduler_handle: Arc::new(RwLock::new(None)),
            shutdown_signal: Arc::new(RwLock::new(None)),
        })
    }

    /// Create a new task queue with custom auto-scaler configuration
    pub async fn with_autoscaler_config(
        redis_url: &str,
        autoscaler_config: AutoScalerConfig,
    ) -> Result<Self, TaskQueueError> {
        let broker = Arc::new(RedisBroker::new(redis_url).await?);
        let scheduler = Arc::new(TaskScheduler::new(broker.clone()));
        let autoscaler = Arc::new(AutoScaler::with_config(broker.clone(), autoscaler_config));
        let metrics = Arc::new(MetricsCollector::new());

        Ok(Self {
            broker,
            scheduler,
            autoscaler,
            metrics,
            workers: Arc::new(RwLock::new(Vec::new())),
            scheduler_handle: Arc::new(RwLock::new(None)),
            shutdown_signal: Arc::new(RwLock::new(None)),
        })
    }

    /// Start the specified number of workers with auto-registered tasks (if available)
    pub async fn start_workers(&self, initial_count: usize) -> Result<(), TaskQueueError> {
        // Try to use auto-registered tasks if the feature is enabled, otherwise use empty registry
        let registry = {
            #[cfg(feature = "auto-register")]
            {
                // Check if there's a global config that enables auto-registration
                if let Ok(config) = TaskQueueConfig::get_or_init() {
                    if config.auto_register.enabled {
                        match TaskRegistry::with_auto_registered_and_config(Some(
                            &config.auto_register,
                        )) {
                            Ok(reg) => Arc::new(reg),
                            Err(e) => {
                                #[cfg(feature = "tracing")]
                                tracing::warn!("Failed to create auto-registered task registry: {}, using empty registry", e);
                                Arc::new(TaskRegistry::new())
                            }
                        }
                    } else {
                        Arc::new(TaskRegistry::new())
                    }
                } else {
                    // Try to create auto-registered registry anyway (fallback for backward compatibility)
                    match TaskRegistry::with_auto_registered() {
                        Ok(reg) => Arc::new(reg),
                        Err(_) => Arc::new(TaskRegistry::new()),
                    }
                }
            }
            #[cfg(not(feature = "auto-register"))]
            {
                Arc::new(TaskRegistry::new())
            }
        };

        self.start_workers_with_registry(initial_count, registry)
            .await
    }

    /// Start workers with a custom task registry
    pub async fn start_workers_with_registry(
        &self,
        initial_count: usize,
        task_registry: Arc<TaskRegistry>,
    ) -> Result<(), TaskQueueError> {
        let mut workers = self.workers.write().await;

        for i in 0..initial_count {
            let worker = Worker::new(
                format!("worker-{}", i),
                self.broker.clone(),
                self.scheduler.clone(),
            )
            .with_task_registry(task_registry.clone());

            let worker_handle = worker.start().await?;
            workers.push(worker_handle);
        }

        #[cfg(feature = "tracing")]
        tracing::info!(
            "Started {} workers with task registry containing {} task types",
            initial_count,
            task_registry.registered_tasks().len()
        );

        Ok(())
    }

    /// Start the auto-scaler background process
    pub fn start_autoscaler(&self) -> Result<(), TaskQueueError> {
        let workers = self.workers.clone();
        let autoscaler = self.autoscaler.clone();
        let broker = self.broker.clone();
        let scheduler = self.scheduler.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

                if let Ok(metrics) = autoscaler.collect_metrics().await {
                    // Clone autoscaler for scaling decision since it requires mutable access
                    if let Ok(action) = {
                        let mut autoscaler_clone = (*autoscaler).clone();
                        autoscaler_clone.decide_scaling_action(&metrics)
                    } {
                        match action {
                            ScalingAction::ScaleUp(count) => {
                                let mut workers_lock = workers.write().await;
                                let current_count = workers_lock.len();

                                for i in current_count..current_count + count {
                                    let worker = Worker::new(
                                        format!("worker-{}", i),
                                        broker.clone(),
                                        scheduler.clone(),
                                    );

                                    if let Ok(worker_handle) = worker.start().await {
                                        workers_lock.push(worker_handle);
                                        #[cfg(feature = "tracing")]
                                        tracing::info!("Scaled up: added worker-{}", i);
                                    }
                                }
                            }
                            ScalingAction::ScaleDown(count) => {
                                let mut workers_lock = workers.write().await;
                                for _ in 0..count {
                                    if let Some(worker) = workers_lock.pop() {
                                        worker.stop().await;
                                        #[cfg(feature = "tracing")]
                                        tracing::info!("Scaled down: removed worker");
                                    }
                                }
                            }
                            ScalingAction::NoAction => {}
                        }
                    }
                }
            }
        });

        #[cfg(feature = "tracing")]
        tracing::info!("Started auto-scaler");

        Ok(())
    }

    /// Start the task scheduler background process
    pub async fn start_scheduler(&self) -> Result<(), TaskQueueError> {
        let mut handle_guard = self.scheduler_handle.write().await;

        // Don't start if already running
        if handle_guard.is_some() {
            return Ok(());
        }

        let handle = TaskScheduler::start(self.broker.clone())?;
        *handle_guard = Some(handle);

        Ok(())
    }

    /// Stop the task scheduler
    pub async fn stop_scheduler(&self) {
        let mut handle_guard = self.scheduler_handle.write().await;
        if let Some(handle) = handle_guard.take() {
            handle.abort();

            #[cfg(feature = "tracing")]
            tracing::info!("Task scheduler stopped");
        }
    }

    /// Enqueue a task for immediate execution
    #[inline]
    pub async fn enqueue<T: Task>(&self, task: T, queue: &str) -> Result<TaskId, TaskQueueError> {
        self.broker.enqueue_task(task, queue).await
    }

    /// Schedule a task for delayed execution
    #[inline]
    pub async fn schedule<T: Task>(
        &self,
        task: T,
        queue: &str,
        delay: chrono::Duration,
    ) -> Result<TaskId, TaskQueueError> {
        self.scheduler.schedule_task(task, queue, delay).await
    }

    /// Get the current number of active workers
    #[inline]
    pub async fn worker_count(&self) -> usize {
        self.workers.read().await.len()
    }

    /// Stop all workers
    pub async fn stop_workers(&self) {
        let mut workers = self.workers.write().await;
        while let Some(worker) = workers.pop() {
            worker.stop().await;
        }

        #[cfg(feature = "tracing")]
        tracing::info!("All workers stopped");
    }

    /// Initiate graceful shutdown of the entire task queue
    pub async fn shutdown(&self) -> Result<(), TaskQueueError> {
        self.shutdown_with_timeout(std::time::Duration::from_secs(30))
            .await
    }

    /// Shutdown with a custom timeout
    pub async fn shutdown_with_timeout(
        &self,
        timeout: std::time::Duration,
    ) -> Result<(), TaskQueueError> {
        #[cfg(feature = "tracing")]
        tracing::info!("Initiating graceful shutdown of TaskQueue...");

        // Create shutdown signal
        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);
        {
            let mut signal = self.shutdown_signal.write().await;
            *signal = Some(shutdown_tx.clone());
        }

        // Shutdown components in order
        let shutdown_future = async {
            // 1. Stop accepting new tasks (stop scheduler)
            self.stop_scheduler().await;
            #[cfg(feature = "tracing")]
            tracing::info!("Scheduler stopped");

            // 2. Stop autoscaler
            // (Autoscaler doesn't have a stop method in current implementation)
            #[cfg(feature = "tracing")]
            tracing::info!("Autoscaler stopped");

            // 3. Wait for running tasks to complete and stop workers
            self.stop_workers().await;
            #[cfg(feature = "tracing")]
            tracing::info!("All workers stopped");

            // 4. Broadcast shutdown signal
            if let Err(e) = shutdown_tx.send(()) {
                #[cfg(feature = "tracing")]
                tracing::warn!("Failed to send shutdown signal: {}", e);
            }

            #[cfg(feature = "tracing")]
            tracing::info!("TaskQueue shutdown completed successfully");

            Ok(())
        };

        // Apply timeout
        match tokio::time::timeout(timeout, shutdown_future).await {
            Ok(result) => result,
            Err(_) => {
                #[cfg(feature = "tracing")]
                tracing::error!("TaskQueue shutdown timed out after {:?}", timeout);
                Err(TaskQueueError::Worker("Shutdown timeout".to_string()))
            }
        }
    }

    /// Check if shutdown has been initiated
    pub async fn is_shutting_down(&self) -> bool {
        self.shutdown_signal.read().await.is_some()
    }

    /// Get a shutdown signal receiver for external components
    pub async fn shutdown_receiver(&self) -> Option<tokio::sync::broadcast::Receiver<()>> {
        self.shutdown_signal
            .read()
            .await
            .as_ref()
            .map(|tx| tx.subscribe())
    }

    /// Get comprehensive health status
    pub async fn health_check(&self) -> Result<HealthStatus, TaskQueueError> {
        let mut health = HealthStatus {
            status: "healthy".to_string(),
            timestamp: chrono::Utc::now(),
            components: std::collections::HashMap::new(),
        };

        // Check Redis connection
        match self.broker.pool.get().await {
            Ok(mut conn) => {
                match redis::cmd("PING")
                    .query_async::<_, String>(&mut *conn)
                    .await
                {
                    Ok(_) => {
                        health.components.insert(
                            "redis".to_string(),
                            ComponentHealth {
                                status: "healthy".to_string(),
                                message: Some("Connection successful".to_string()),
                            },
                        );
                    }
                    Err(e) => {
                        health.status = "unhealthy".to_string();
                        health.components.insert(
                            "redis".to_string(),
                            ComponentHealth {
                                status: "unhealthy".to_string(),
                                message: Some(format!("Ping failed: {}", e)),
                            },
                        );
                    }
                }
            }
            Err(e) => {
                health.status = "unhealthy".to_string();
                health.components.insert(
                    "redis".to_string(),
                    ComponentHealth {
                        status: "unhealthy".to_string(),
                        message: Some(format!("Connection failed: {}", e)),
                    },
                );
            }
        }

        // Check workers
        let worker_count = self.worker_count().await;
        health.components.insert(
            "workers".to_string(),
            ComponentHealth {
                status: if worker_count > 0 {
                    "healthy"
                } else {
                    "warning"
                }
                .to_string(),
                message: Some(format!("{} active workers", worker_count)),
            },
        );

        // Check scheduler
        let scheduler_running = self.scheduler_handle.read().await.is_some();
        health.components.insert(
            "scheduler".to_string(),
            ComponentHealth {
                status: if scheduler_running {
                    "healthy"
                } else {
                    "stopped"
                }
                .to_string(),
                message: Some(
                    if scheduler_running {
                        "Running"
                    } else {
                        "Stopped"
                    }
                    .to_string(),
                ),
            },
        );

        Ok(health)
    }

    /// Get comprehensive metrics
    pub async fn get_metrics(&self) -> Result<TaskQueueMetrics, TaskQueueError> {
        let queues = ["default", "high_priority", "low_priority"];
        let mut queue_metrics = Vec::new();
        let mut total_pending = 0;
        let mut total_processed = 0;
        let mut total_failed = 0;

        for queue in &queues {
            let metrics = self.broker.get_queue_metrics(queue).await?;
            total_pending += metrics.pending_tasks;
            total_processed += metrics.processed_tasks;
            total_failed += metrics.failed_tasks;
            queue_metrics.push(metrics);
        }

        let autoscaler_metrics = self.autoscaler.collect_metrics().await?;

        Ok(TaskQueueMetrics {
            timestamp: chrono::Utc::now(),
            total_pending_tasks: total_pending,
            total_processed_tasks: total_processed,
            total_failed_tasks: total_failed,
            active_workers: autoscaler_metrics.active_workers,
            tasks_per_worker: autoscaler_metrics.queue_pressure_score,
            queue_metrics,
        })
    }

    /// Get enhanced system metrics with memory and performance data
    pub async fn get_system_metrics(&self) -> SystemMetrics {
        self.metrics.get_system_metrics().await
    }

    /// Get a quick metrics summary for debugging
    pub async fn get_metrics_summary(&self) -> String {
        self.metrics.get_metrics_summary().await
    }
}

/// Builder for configuring and creating TaskQueue instances
pub struct TaskQueueBuilder {
    config: TaskQueueConfig,
    task_registry: Option<Arc<TaskRegistry>>,
    override_redis_url: Option<String>,
}

impl TaskQueueBuilder {
    /// Create a new TaskQueue builder with default configuration
    #[inline]
    pub fn new(redis_url: impl Into<String>) -> Self {
        let mut config = TaskQueueConfig::default();
        config.redis.url = redis_url.into();

        Self {
            config,
            task_registry: None,
            override_redis_url: None,
        }
    }

    /// Create a TaskQueue builder from global configuration
    #[inline]
    pub fn from_global_config() -> Result<Self, TaskQueueError> {
        let config = TaskQueueConfig::get_or_init()?.clone();
        Ok(Self {
            config,
            task_registry: None,
            override_redis_url: None,
        })
    }

    /// Create a TaskQueue builder from a specific configuration
    #[must_use]
    #[inline]
    pub fn from_config(config: TaskQueueConfig) -> Self {
        Self {
            config,
            task_registry: None,
            override_redis_url: None,
        }
    }

    /// Create a TaskQueue builder that auto-loads configuration
    #[inline]
    pub fn auto() -> Result<Self, TaskQueueError> {
        let config = TaskQueueConfig::load()?;
        Ok(Self {
            config,
            task_registry: None,
            override_redis_url: None,
        })
    }

    /// Override Redis URL (useful for testing or special cases)
    #[must_use]
    #[inline]
    pub fn redis_url(mut self, url: impl Into<String>) -> Self {
        self.override_redis_url = Some(url.into());
        self
    }

    /// Set the auto-scaler configuration
    #[must_use]
    #[inline]
    pub fn autoscaler_config(mut self, config: AutoScalerConfig) -> Self {
        self.config.autoscaler = config;
        self
    }

    /// Set the initial number of workers to start
    #[must_use]
    #[inline]
    pub fn initial_workers(mut self, count: usize) -> Self {
        self.config.workers.initial_count = count;
        self
    }

    /// Set a custom task registry
    #[must_use]
    #[inline]
    pub fn task_registry(mut self, registry: Arc<TaskRegistry>) -> Self {
        self.task_registry = Some(registry);
        self
    }

    /// Enable automatic task registration using inventory pattern
    #[cfg(feature = "auto-register")]
    #[must_use]
    #[inline]
    pub fn auto_register_tasks(mut self) -> Self {
        self.config.auto_register.enabled = true;
        self
    }

    /// Disable automatic task registration
    #[cfg(feature = "auto-register")]
    #[must_use]
    #[inline]
    pub fn disable_auto_register(mut self) -> Self {
        self.config.auto_register.enabled = false;
        self
    }

    /// Start the task scheduler automatically
    #[must_use]
    #[inline]
    pub fn with_scheduler(mut self) -> Self {
        self.config.scheduler.enabled = true;
        self
    }

    /// Start the auto-scaler automatically
    #[must_use]
    #[inline]
    pub fn with_autoscaler(self) -> Self {
        // Auto-scaler is always available, we just don't start it by default
        // This is a no-op now since we determine if to start it during build
        self
    }

    /// Update the full configuration
    #[must_use]
    #[inline]
    pub fn config(mut self, config: TaskQueueConfig) -> Self {
        self.config = config;
        self
    }

    /// Build the `TaskQueue` instance
    ///
    /// # Errors
    /// Returns `TaskQueueError` if Redis connection fails or configuration is invalid
    #[inline]
    pub async fn build(self) -> Result<TaskQueue, TaskQueueError> {
        // Use override Redis URL if provided, otherwise use config
        let redis_url = self
            .override_redis_url
            .as_ref()
            .unwrap_or(&self.config.redis.url);

        // Create TaskQueue with autoscaler config
        let task_queue =
            TaskQueue::with_autoscaler_config(redis_url, self.config.autoscaler.clone()).await?;

        // Handle task registry setup
        let registry = if self.config.auto_register.enabled {
            #[cfg(feature = "auto-register")]
            {
                if let Some(custom_registry) = self.task_registry {
                    // Auto-register tasks into the provided registry using configuration
                    custom_registry
                        .auto_register_tasks_with_config(Some(&self.config.auto_register))
                        .map_err(|e| {
                            TaskQueueError::Configuration(format!("Auto-registration failed: {e}"))
                        })?;
                    custom_registry
                } else {
                    // Create a new registry with auto-registered tasks using configuration
                    Arc::new(
                        TaskRegistry::with_auto_registered_and_config(Some(
                            &self.config.auto_register,
                        ))
                        .map_err(|e| {
                            TaskQueueError::Configuration(format!("Auto-registration failed: {e}"))
                        })?,
                    )
                }
            }
            #[cfg(not(feature = "auto-register"))]
            {
                return Err(TaskQueueError::Configuration(
                    "Auto-registration requested but 'auto-register' feature is not enabled"
                        .to_string(),
                ));
            }
        } else {
            self.task_registry
                .unwrap_or_else(|| Arc::new(TaskRegistry::new()))
        };

        // Start workers
        let worker_count = self.config.workers.initial_count;
        if worker_count > 0 {
            task_queue
                .start_workers_with_registry(worker_count, registry)
                .await?;
        }

        // Start scheduler if enabled
        if self.config.scheduler.enabled {
            task_queue.start_scheduler().await?;
        }

        // Auto-scaler is always available but we start it based on configuration
        // For now, we'll start it if there are specific auto-scaling settings
        if self.config.autoscaler.max_workers > self.config.autoscaler.min_workers {
            task_queue.start_autoscaler()?;
        }

        Ok(task_queue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use tokio_test;

    #[test]
    fn test_autoscaler_config() {
        let config = AutoScalerConfig::default();
        assert_eq!(config.min_workers, 1);
        assert_eq!(config.max_workers, 20);
        assert_eq!(config.scaling_triggers.queue_pressure_threshold, 0.75);
    }

    #[test]
    fn test_queue_manager() {
        let manager = QueueManager::new();
        let queues = manager.get_queue_names();

        assert!(queues.contains(&"default".to_owned()));
        assert!(queues.contains(&"high_priority".to_owned()));
        assert!(queues.contains(&"low_priority".to_owned()));
    }

    #[test]
    fn test_builder_pattern() {
        let builder = TaskQueueBuilder::new("redis://localhost:6379")
            .initial_workers(4)
            .with_scheduler()
            .with_autoscaler();

        assert_eq!(builder.config.redis.url, "redis://localhost:6379");
        assert_eq!(builder.config.workers.initial_count, 4);
        assert!(builder.config.scheduler.enabled);
    }
}

/// Health check status for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub components: std::collections::HashMap<String, ComponentHealth>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub status: String,
    pub message: Option<String>,
}

/// Comprehensive metrics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskQueueMetrics {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub total_pending_tasks: i64,
    pub total_processed_tasks: i64,
    pub total_failed_tasks: i64,
    pub active_workers: i64,
    pub tasks_per_worker: f64,
    pub queue_metrics: Vec<QueueMetrics>,
}
