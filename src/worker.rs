use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use crate::{error::TaskQueueError, queue::queue_names, RedisBroker, TaskScheduler, TaskWrapper};
use serde::Serialize;
use tokio::sync::{mpsc, Semaphore};
use tokio::task::JoinHandle;

pub struct Worker {
    id: String,
    broker: Arc<RedisBroker>,
    #[allow(dead_code)]
    scheduler: Arc<TaskScheduler>,
    task_registry: Arc<crate::TaskRegistry>,
    shutdown_tx: Option<mpsc::Sender<()>>,
    handle: Option<JoinHandle<()>>,
    // Added for backpressure control
    max_concurrent_tasks: usize,
    task_semaphore: Option<Arc<Semaphore>>,
    // Task tracking for better observability
    active_tasks: Arc<AtomicUsize>,
}

/// Configuration for worker backpressure and performance tuning
#[derive(Debug, Clone)]
pub struct WorkerBackpressureConfig {
    pub max_concurrent_tasks: usize,
    pub queue_size_threshold: usize,
    pub backpressure_delay_ms: u64,
}

/// Context for task execution spawning
#[derive(Clone)]
struct TaskExecutionContext {
    broker: Arc<RedisBroker>,
    task_registry: Arc<crate::TaskRegistry>,
    worker_id: String,
    semaphore: Option<Arc<Semaphore>>,
    active_tasks: Arc<AtomicUsize>,
}

/// Result of task spawning attempt
#[derive(Debug)]
enum SpawnResult {
    Spawned,
    Rejected(TaskWrapper),
    Failed(TaskQueueError),
}

impl Worker {
    pub fn new(id: String, broker: Arc<RedisBroker>, scheduler: Arc<TaskScheduler>) -> Self {
        let max_concurrent_tasks = 10;
        Self {
            id,
            broker,
            scheduler,
            task_registry: Arc::new(crate::TaskRegistry::new()),
            shutdown_tx: None,
            handle: None,
            max_concurrent_tasks,
            task_semaphore: Some(Arc::new(Semaphore::new(max_concurrent_tasks))),
            active_tasks: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn with_task_registry(mut self, registry: Arc<crate::TaskRegistry>) -> Self {
        self.task_registry = registry;
        self
    }

    pub fn with_backpressure_config(mut self, config: WorkerBackpressureConfig) -> Self {
        self.max_concurrent_tasks = config.max_concurrent_tasks;
        self.task_semaphore = Some(Arc::new(Semaphore::new(config.max_concurrent_tasks)));
        self
    }

    pub fn with_max_concurrent_tasks(mut self, max_tasks: usize) -> Self {
        self.max_concurrent_tasks = max_tasks;
        self.task_semaphore = Some(Arc::new(Semaphore::new(max_tasks)));
        self
    }

    /// Get the current number of active tasks
    pub fn active_task_count(&self) -> usize {
        self.active_tasks.load(Ordering::Relaxed)
    }

    pub async fn start(mut self) -> Result<Self, TaskQueueError> {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        let execution_context = TaskExecutionContext {
            broker: self.broker.clone(),
            task_registry: self.task_registry.clone(),
            worker_id: self.id.clone(),
            semaphore: self.task_semaphore.clone(),
            active_tasks: self.active_tasks.clone(),
        };

        // Register worker
        execution_context
            .broker
            .register_worker(&execution_context.worker_id)
            .await?;

        let handle = tokio::spawn(async move {
            let queues = vec![
                queue_names::DEFAULT.to_string(),
                queue_names::HIGH_PRIORITY.to_string(),
                queue_names::LOW_PRIORITY.to_string(),
            ];

            loop {
                tokio::select! {
                    // Check for shutdown signal
                    _ = shutdown_rx.recv() => {
                        #[cfg(feature = "tracing")]
                        tracing::info!(
                            "Worker {} shutting down with {} active tasks",
                            execution_context.worker_id,
                            execution_context.active_tasks.load(Ordering::Relaxed)
                        );

                        // Wait for active tasks to complete before shutting down
                        Self::graceful_shutdown(&execution_context).await;

                        if let Err(_e) = execution_context.broker.unregister_worker(&execution_context.worker_id).await {
                            #[cfg(feature = "tracing")]
                            tracing::error!("Failed to unregister worker {}: {}", execution_context.worker_id, _e);
                        }
                        break;
                    }

                    // Process tasks with improved spawning logic
                    task_result = execution_context.broker.dequeue_task(&queues) => {
                        match task_result {
                            Ok(Some(task_wrapper)) => {
                                match Self::handle_task_execution(execution_context.clone(), task_wrapper).await {
                                    SpawnResult::Spawned => {
                                        // Task successfully spawned
                                        #[cfg(feature = "tracing")]
                                        tracing::debug!("Task spawned, active tasks: {}", execution_context.active_tasks.load(Ordering::Relaxed));
                                    }
                                    SpawnResult::Rejected(_rejected_task) => {
                                        // Task was rejected due to backpressure, will be re-queued
                                        #[cfg(feature = "tracing")]
                                        tracing::debug!("Task {} rejected due to backpressure", _rejected_task.metadata.id);
                                    }
                                    SpawnResult::Failed(_e) => {
                                        #[cfg(feature = "tracing")]
                                        tracing::error!("Failed to handle task execution: {}", _e);
                                    }
                                }
                            }
                            Ok(None) => {
                                // No tasks available, update heartbeat
                                if let Err(_e) = execution_context.broker.update_worker_heartbeat(&execution_context.worker_id).await {
                                    #[cfg(feature = "tracing")]
                                    tracing::error!("Failed to update heartbeat for worker {}: {}", execution_context.worker_id, _e);
                                }
                            }
                            Err(_e) => {
                                #[cfg(feature = "tracing")]
                                tracing::error!("Worker {} error dequeuing task: {}", execution_context.worker_id, _e);

                                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            }
                        }
                    }
                }
            }
        });

        self.shutdown_tx = Some(shutdown_tx);
        self.handle = Some(handle);

        #[cfg(feature = "tracing")]
        tracing::info!("Started worker {}", self.id);

        Ok(self)
    }

    /// Improved async task spawning with proper resource management
    async fn handle_task_execution(
        context: TaskExecutionContext,
        task_wrapper: TaskWrapper,
    ) -> SpawnResult {
        // Extract semaphore to avoid borrowing issues
        let semaphore_opt = context.semaphore.clone();

        match semaphore_opt {
            Some(semaphore) => {
                // Clone semaphore before use to avoid borrow checker issues
                let semaphore_clone = semaphore.clone();

                // Try to acquire permit without blocking the main worker loop
                match semaphore.try_acquire() {
                    Ok(_permit) => {
                        // Drop permit and rely on async acquisition in spawned task
                        // This maintains backpressure while avoiding lifetime issues
                        drop(_permit);
                        Self::spawn_task_with_semaphore(context, task_wrapper, semaphore_clone)
                            .await;
                        SpawnResult::Spawned
                    }
                    Err(_) => {
                        // At capacity, handle backpressure
                        Self::handle_backpressure(context, task_wrapper).await
                    }
                }
            }
            None => {
                // No backpressure control, execute directly
                Self::execute_task_directly(context, task_wrapper).await;
                SpawnResult::Spawned
            }
        }
    }

    /// Wait for active tasks to complete during shutdown
    async fn graceful_shutdown(context: &TaskExecutionContext) {
        let shutdown_timeout = tokio::time::Duration::from_secs(30);
        let check_interval = tokio::time::Duration::from_millis(100);

        let start_time = tokio::time::Instant::now();

        while context.active_tasks.load(Ordering::Relaxed) > 0
            && start_time.elapsed() < shutdown_timeout
        {
            #[cfg(feature = "tracing")]
            tracing::debug!(
                "Worker {} waiting for {} active tasks to complete",
                context.worker_id,
                context.active_tasks.load(Ordering::Relaxed)
            );

            tokio::time::sleep(check_interval).await;
        }

        if context.active_tasks.load(Ordering::Relaxed) > 0 {
            #[cfg(feature = "tracing")]
            tracing::warn!(
                "Worker {} shutdown timeout reached with {} active tasks",
                context.worker_id,
                context.active_tasks.load(Ordering::Relaxed)
            );
        }
    }

    /// Spawn task execution with semaphore backpressure
    async fn spawn_task_with_semaphore(
        context: TaskExecutionContext,
        task_wrapper: TaskWrapper,
        semaphore: Arc<Semaphore>,
    ) {
        // Increment active task counter
        context.active_tasks.fetch_add(1, Ordering::Relaxed);

        tokio::spawn(async move {
            // Acquire permit for the full duration of execution - this is the correct approach
            let _permit = semaphore
                .acquire()
                .await
                .expect("Semaphore should not be closed");

            if let Err(_e) = Self::process_task(
                &context.broker,
                &context.task_registry,
                &context.worker_id,
                task_wrapper,
            )
            .await
            {
                #[cfg(feature = "tracing")]
                tracing::error!("Task processing failed: {}", _e);
            }

            // Decrement active task counter
            context.active_tasks.fetch_sub(1, Ordering::Relaxed);
            // Permit is automatically released when dropped here
        });
    }

    /// Handle backpressure by re-queuing task
    async fn handle_backpressure(
        context: TaskExecutionContext,
        task_wrapper: TaskWrapper,
    ) -> SpawnResult {
        // Attempt to re-queue the task
        match Self::requeue_task(&context.broker, task_wrapper.clone()).await {
            Ok(_) => {
                #[cfg(feature = "tracing")]
                tracing::debug!(
                    "Task {} re-queued due to backpressure",
                    task_wrapper.metadata.id
                );
                SpawnResult::Rejected(task_wrapper)
            }
            Err(e) => SpawnResult::Failed(e),
        }
    }

    /// Execute task directly without semaphore control
    async fn execute_task_directly(context: TaskExecutionContext, task_wrapper: TaskWrapper) {
        // Increment active task counter
        context.active_tasks.fetch_add(1, Ordering::Relaxed);

        tokio::spawn(async move {
            if let Err(_e) = Self::process_task(
                &context.broker,
                &context.task_registry,
                &context.worker_id,
                task_wrapper,
            )
            .await
            {
                #[cfg(feature = "tracing")]
                tracing::error!("Task processing failed: {}", _e);
            }

            // Decrement active task counter
            context.active_tasks.fetch_sub(1, Ordering::Relaxed);
        });
    }

    /// Re-queue a task for later processing
    async fn requeue_task(
        broker: &RedisBroker,
        task_wrapper: TaskWrapper,
    ) -> Result<(), TaskQueueError> {
        broker
            .enqueue_task_wrapper(task_wrapper, queue_names::DEFAULT)
            .await?;
        Ok(())
    }

    pub async fn stop(self) {
        if let Some(tx) = self.shutdown_tx {
            if let Err(_e) = tx.send(()).await {
                #[cfg(feature = "tracing")]
                tracing::error!("Failed to send shutdown signal");
            }
        }

        if let Some(handle) = self.handle {
            let _ = handle.await;
        }
    }

    async fn process_task(
        broker: &RedisBroker,
        task_registry: &crate::TaskRegistry,
        _worker_id: &str,
        mut task_wrapper: TaskWrapper,
    ) -> Result<(), TaskQueueError> {
        let task_id = task_wrapper.metadata.id;

        #[cfg(feature = "tracing")]
        tracing::debug!(
            "Processing task {}: {}",
            task_id,
            task_wrapper.metadata.name
        );

        task_wrapper.metadata.attempts += 1;

        // Execute the task
        match Self::execute_task_with_registry(task_registry, &task_wrapper).await {
            Ok(_result) => {
                #[cfg(feature = "tracing")]
                tracing::debug!("Task {} completed successfully", task_id);

                broker
                    .mark_task_completed(task_id, queue_names::DEFAULT)
                    .await?;
            }
            Err(error) => {
                let error_msg = error.to_string();

                #[cfg(feature = "tracing")]
                tracing::error!("Task {} failed: {}", task_id, error_msg);

                // Check if we should retry
                if task_wrapper.metadata.attempts < task_wrapper.metadata.max_retries {
                    #[cfg(feature = "tracing")]
                    tracing::info!(
                        "Re-queuing task {} for retry (attempt {} of {})",
                        task_id,
                        task_wrapper.metadata.attempts,
                        task_wrapper.metadata.max_retries
                    );

                    // Re-enqueue for retry
                    broker
                        .enqueue_task_wrapper(task_wrapper, queue_names::DEFAULT)
                        .await?;
                } else {
                    #[cfg(feature = "tracing")]
                    tracing::error!(
                        "Task {} failed permanently after {} attempts",
                        task_id,
                        task_wrapper.metadata.attempts
                    );

                    broker
                        .mark_task_failed_with_reason(
                            task_id,
                            queue_names::DEFAULT,
                            Some(error_msg),
                        )
                        .await?;
                }
            }
        }

        Ok(())
    }

    async fn execute_task_with_registry(
        task_registry: &crate::TaskRegistry,
        task_wrapper: &TaskWrapper,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let task_name = &task_wrapper.metadata.name;

        // Try to execute using the task registry
        match task_registry
            .execute(task_name, task_wrapper.payload.clone())
            .await
        {
            Ok(result) => {
                #[cfg(feature = "tracing")]
                tracing::debug!("Executing task {} with registered executor", task_name);

                Ok(result)
            }
            Err(error) => {
                // Check if this is a "task not found" error vs a task execution failure
                let error_msg = error.to_string();
                if error_msg.contains("Unknown task type") {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(
                        "No executor found for task type: {}, using fallback",
                        task_name
                    );

                    // Fallback: serialize task metadata as response
                    #[derive(Serialize)]
                    struct FallbackResponse {
                        status: String,
                        message: String,
                        timestamp: String,
                    }

                    let response = FallbackResponse {
                        status: "completed".to_string(),
                        message: format!("Task {} completed with fallback executor", task_name),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    };

                    let serialized = serde_json::to_vec(&response)
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

                    Ok(serialized)
                } else {
                    // This is an actual task execution failure - propagate it
                    #[cfg(feature = "tracing")]
                    tracing::error!("Task {} failed during execution: {}", task_name, error_msg);

                    Err(error)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TaskId, TaskMetadata};
    use std::time::Duration;
    use tokio::time::timeout;

    fn create_test_broker() -> Arc<RedisBroker> {
        // Create a mock broker for testing - this is a simplified version
        let redis_url = std::env::var("REDIS_TEST_URL")
            .unwrap_or_else(|_| "redis://127.0.0.1:6379/15".to_string());

        // For unit tests, we'll create a minimal broker
        let config = deadpool_redis::Config::from_url(&redis_url);
        let pool = config
            .create_pool(Some(deadpool_redis::Runtime::Tokio1))
            .expect("Failed to create test pool");

        Arc::new(RedisBroker { pool })
    }

    fn create_test_scheduler() -> Arc<TaskScheduler> {
        let broker = create_test_broker();
        Arc::new(TaskScheduler::new(broker))
    }

    fn create_test_task_wrapper() -> TaskWrapper {
        TaskWrapper {
            metadata: TaskMetadata {
                id: TaskId::new_v4(),
                name: "test_task".to_string(),
                created_at: chrono::Utc::now(),
                attempts: 0,
                max_retries: 3,
                timeout_seconds: 60,
            },
            payload: b"test payload".to_vec(),
        }
    }

    // Helper function to get a connection for tests since get_conn is private
    async fn get_test_connection(
        broker: &RedisBroker,
    ) -> Result<deadpool_redis::Connection, deadpool_redis::PoolError> {
        broker.pool.get().await
    }

    #[test]
    fn test_worker_creation() {
        let broker = create_test_broker();
        let scheduler = create_test_scheduler();
        let worker_id = "test_worker_001".to_string();

        let worker = Worker::new(worker_id.clone(), broker, scheduler);

        assert_eq!(worker.id, worker_id);
        assert_eq!(worker.max_concurrent_tasks, 10);
        assert_eq!(worker.active_task_count(), 0);
        assert!(worker.task_semaphore.is_some());
        assert!(worker.shutdown_tx.is_none());
        assert!(worker.handle.is_none());
    }

    #[test]
    fn test_worker_with_task_registry() {
        let broker = create_test_broker();
        let scheduler = create_test_scheduler();
        let worker_id = "test_worker_002".to_string();
        let registry = Arc::new(crate::TaskRegistry::new());

        let _worker =
            Worker::new(worker_id, broker, scheduler).with_task_registry(registry.clone());

        // The registry should be set
        assert_eq!(Arc::strong_count(&registry), 2); // One in worker, one here
    }

    #[test]
    fn test_worker_with_backpressure_config() {
        let broker = create_test_broker();
        let scheduler = create_test_scheduler();
        let worker_id = "test_worker_003".to_string();

        let config = WorkerBackpressureConfig {
            max_concurrent_tasks: 20,
            queue_size_threshold: 100,
            backpressure_delay_ms: 500,
        };

        let worker =
            Worker::new(worker_id, broker, scheduler).with_backpressure_config(config.clone());

        assert_eq!(worker.max_concurrent_tasks, config.max_concurrent_tasks);
        assert!(worker.task_semaphore.is_some());

        if let Some(semaphore) = &worker.task_semaphore {
            assert_eq!(semaphore.available_permits(), config.max_concurrent_tasks);
        }
    }

    #[test]
    fn test_worker_with_max_concurrent_tasks() {
        let broker = create_test_broker();
        let scheduler = create_test_scheduler();
        let worker_id = "test_worker_004".to_string();
        let max_tasks = 15;

        let worker = Worker::new(worker_id, broker, scheduler).with_max_concurrent_tasks(max_tasks);

        assert_eq!(worker.max_concurrent_tasks, max_tasks);
        assert!(worker.task_semaphore.is_some());

        if let Some(semaphore) = &worker.task_semaphore {
            assert_eq!(semaphore.available_permits(), max_tasks);
        }
    }

    #[test]
    fn test_worker_backpressure_config_clone() {
        let original = WorkerBackpressureConfig {
            max_concurrent_tasks: 25,
            queue_size_threshold: 200,
            backpressure_delay_ms: 1000,
        };

        let cloned = original.clone();

        assert_eq!(original.max_concurrent_tasks, cloned.max_concurrent_tasks);
        assert_eq!(original.queue_size_threshold, cloned.queue_size_threshold);
        assert_eq!(original.backpressure_delay_ms, cloned.backpressure_delay_ms);
    }

    #[test]
    fn test_worker_backpressure_config_debug() {
        let config = WorkerBackpressureConfig {
            max_concurrent_tasks: 8,
            queue_size_threshold: 50,
            backpressure_delay_ms: 250,
        };

        let debug_str = format!("{:?}", config);

        assert!(debug_str.contains("WorkerBackpressureConfig"));
        assert!(debug_str.contains("max_concurrent_tasks: 8"));
        assert!(debug_str.contains("queue_size_threshold: 50"));
        assert!(debug_str.contains("backpressure_delay_ms: 250"));
    }

    #[test]
    fn test_spawn_result_debug() {
        let spawned = SpawnResult::Spawned;
        let rejected = SpawnResult::Rejected(create_test_task_wrapper());
        let failed = SpawnResult::Failed(TaskQueueError::Serialization(
            rmp_serde::encode::Error::Syntax("test error".to_string()),
        ));

        let spawned_debug = format!("{:?}", spawned);
        let rejected_debug = format!("{:?}", rejected);
        let failed_debug = format!("{:?}", failed);

        assert!(spawned_debug.contains("Spawned"));
        assert!(rejected_debug.contains("Rejected"));
        assert!(failed_debug.contains("Failed"));
    }

    #[test]
    fn test_task_execution_context_clone() {
        let broker = create_test_broker();
        let task_registry = Arc::new(crate::TaskRegistry::new());
        let worker_id = "test_worker_005".to_string();
        let semaphore = Some(Arc::new(Semaphore::new(10)));
        let active_tasks = Arc::new(AtomicUsize::new(0));

        let context = TaskExecutionContext {
            broker: broker.clone(),
            task_registry: task_registry.clone(),
            worker_id: worker_id.clone(),
            semaphore: semaphore.clone(),
            active_tasks: active_tasks.clone(),
        };

        let cloned = context.clone();

        assert_eq!(cloned.worker_id, worker_id);
        assert_eq!(cloned.active_tasks.load(Ordering::Relaxed), 0);
        assert!(cloned.semaphore.is_some());
    }

    #[tokio::test]
    async fn test_active_task_count_tracking() {
        let broker = create_test_broker();
        let scheduler = create_test_scheduler();
        let worker_id = "test_worker_006".to_string();

        let worker = Worker::new(worker_id, broker, scheduler);

        assert_eq!(worker.active_task_count(), 0);

        // Simulate task processing
        worker.active_tasks.fetch_add(1, Ordering::Relaxed);
        assert_eq!(worker.active_task_count(), 1);

        worker.active_tasks.fetch_add(2, Ordering::Relaxed);
        assert_eq!(worker.active_task_count(), 3);

        worker.active_tasks.fetch_sub(1, Ordering::Relaxed);
        assert_eq!(worker.active_task_count(), 2);
    }

    #[tokio::test]
    async fn test_requeue_task() {
        let broker = create_test_broker();
        let task_wrapper = create_test_task_wrapper();

        // Clean up any existing data
        if let Ok(mut conn) = get_test_connection(&broker).await {
            let _: Result<String, _> = redis::cmd("FLUSHDB").query_async(&mut conn).await;
        }

        let result = Worker::requeue_task(&broker, task_wrapper).await;
        assert!(result.is_ok());

        // Verify task was requeued
        let queue_size = broker
            .get_queue_size(queue_names::DEFAULT)
            .await
            .expect("Failed to get queue size");
        assert!(queue_size > 0);
    }

    #[tokio::test]
    async fn test_execute_task_with_registry_fallback() {
        let task_registry = crate::TaskRegistry::new();
        let task_wrapper = create_test_task_wrapper();

        let result = Worker::execute_task_with_registry(&task_registry, &task_wrapper).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(!output.is_empty());

        // Verify it's valid JSON (fallback response)
        let parsed: serde_json::Value =
            serde_json::from_slice(&output).expect("Should be valid JSON");

        assert_eq!(parsed["status"], "completed");
        assert!(parsed["message"].as_str().unwrap().contains("test_task"));
        assert!(parsed["timestamp"].is_string());
    }

    #[tokio::test]
    async fn test_process_task_success() {
        let broker = create_test_broker();
        let task_registry = crate::TaskRegistry::new();
        let worker_id = "test_worker_007";
        let task_wrapper = create_test_task_wrapper();

        // Clean up any existing data
        if let Ok(mut conn) = get_test_connection(&broker).await {
            let _: Result<String, _> = redis::cmd("FLUSHDB").query_async(&mut conn).await;
        }

        let result = Worker::process_task(&broker, &task_registry, worker_id, task_wrapper).await;
        assert!(result.is_ok());

        // Verify metrics were updated
        let metrics = broker
            .get_queue_metrics(queue_names::DEFAULT)
            .await
            .expect("Failed to get metrics");
        assert_eq!(metrics.processed_tasks, 1);
    }

    #[tokio::test]
    async fn test_execute_task_directly() {
        let broker = create_test_broker();
        let task_registry = Arc::new(crate::TaskRegistry::new());
        let worker_id = "test_worker_012".to_string();
        let active_tasks = Arc::new(AtomicUsize::new(0));
        let task_wrapper = create_test_task_wrapper();

        let context = TaskExecutionContext {
            broker,
            task_registry,
            worker_id,
            semaphore: None,
            active_tasks: active_tasks.clone(),
        };

        assert_eq!(active_tasks.load(Ordering::Relaxed), 0);

        Worker::execute_task_directly(context, task_wrapper).await;

        // Wait longer and poll for the task to start (more robust timing)
        let mut attempts = 0;
        while active_tasks.load(Ordering::Relaxed) == 0 && attempts < 50 {
            tokio::time::sleep(Duration::from_millis(10)).await;
            attempts += 1;
        }

        // The task should have incremented the counter
        assert!(
            active_tasks.load(Ordering::Relaxed) >= 1,
            "Task should have started and incremented active count"
        );

        // Wait for task to complete with longer timeout
        let mut attempts = 0;
        while active_tasks.load(Ordering::Relaxed) > 0 && attempts < 100 {
            tokio::time::sleep(Duration::from_millis(10)).await;
            attempts += 1;
        }

        // The task should have decremented the counter when it completed
        assert_eq!(active_tasks.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn test_graceful_shutdown() {
        let broker = create_test_broker();
        let task_registry = Arc::new(crate::TaskRegistry::new());
        let worker_id = "test_worker_008".to_string();
        let active_tasks = Arc::new(AtomicUsize::new(2));

        let context = TaskExecutionContext {
            broker,
            task_registry,
            worker_id,
            semaphore: None,
            active_tasks: active_tasks.clone(),
        };

        // Simulate tasks completing during shutdown
        let active_tasks_clone = active_tasks.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            active_tasks_clone.fetch_sub(1, Ordering::Relaxed);
            tokio::time::sleep(Duration::from_millis(50)).await;
            active_tasks_clone.fetch_sub(1, Ordering::Relaxed);
        });

        // Test graceful shutdown with more generous timeout for system variations
        let start = std::time::Instant::now();
        Worker::graceful_shutdown(&context).await;
        let elapsed = start.elapsed();

        assert_eq!(context.active_tasks.load(Ordering::Relaxed), 0);
        assert!(
            elapsed < Duration::from_millis(500),
            "Shutdown should complete in reasonable time"
        ); // More generous timeout
    }

    #[test]
    fn test_worker_default_configuration() {
        let broker = create_test_broker();
        let scheduler = create_test_scheduler();
        let worker_id = "test_worker_011".to_string();

        let worker = Worker::new(worker_id.clone(), broker.clone(), scheduler.clone());

        // Test default values
        assert_eq!(worker.id, worker_id);
        assert_eq!(worker.max_concurrent_tasks, 10);
        assert_eq!(worker.active_task_count(), 0);
        assert!(worker.task_semaphore.is_some());
        assert!(worker.shutdown_tx.is_none());
        assert!(worker.handle.is_none());

        // Test that broker and scheduler are properly stored
        assert_eq!(Arc::strong_count(&broker), 2); // One in worker, one here
        assert_eq!(Arc::strong_count(&scheduler), 2); // One in worker, one here
    }

    #[test]
    fn test_worker_backpressure_config_defaults() {
        let config = WorkerBackpressureConfig {
            max_concurrent_tasks: 50,
            queue_size_threshold: 1000,
            backpressure_delay_ms: 100,
        };

        assert_eq!(config.max_concurrent_tasks, 50);
        assert_eq!(config.queue_size_threshold, 1000);
        assert_eq!(config.backpressure_delay_ms, 100);
    }

    #[tokio::test]
    async fn test_worker_method_chaining() {
        let broker = create_test_broker();
        let scheduler = create_test_scheduler();
        let worker_id = "test_worker_013".to_string();
        let registry = Arc::new(crate::TaskRegistry::new());

        let config = WorkerBackpressureConfig {
            max_concurrent_tasks: 25,
            queue_size_threshold: 500,
            backpressure_delay_ms: 200,
        };

        let worker = Worker::new(worker_id.clone(), broker, scheduler)
            .with_task_registry(registry.clone())
            .with_backpressure_config(config.clone())
            .with_max_concurrent_tasks(30); // This should override the config value

        assert_eq!(worker.id, worker_id);
        assert_eq!(worker.max_concurrent_tasks, 30); // Should be overridden
        assert_eq!(Arc::strong_count(&registry), 2); // One in worker, one here

        if let Some(semaphore) = &worker.task_semaphore {
            assert_eq!(semaphore.available_permits(), 30);
        }
    }

    #[tokio::test]
    async fn test_graceful_shutdown_timeout() {
        let broker = create_test_broker();
        let task_registry = Arc::new(crate::TaskRegistry::new());
        let worker_id = "test_worker_009".to_string();
        let active_tasks = Arc::new(AtomicUsize::new(1)); // Task that never completes

        let context = TaskExecutionContext {
            broker,
            task_registry,
            worker_id,
            semaphore: None,
            active_tasks,
        };

        // Test shutdown timeout (this would normally wait 30s, but we'll test with timeout)
        let result = timeout(
            Duration::from_millis(200),
            Worker::graceful_shutdown(&context),
        )
        .await;
        assert!(result.is_err()); // Should timeout
        assert_eq!(context.active_tasks.load(Ordering::Relaxed), 1); // Task still active
    }

    #[tokio::test]
    async fn test_handle_backpressure() {
        let broker = create_test_broker();
        let task_registry = Arc::new(crate::TaskRegistry::new());
        let worker_id = "test_worker_010".to_string();
        let task_wrapper = create_test_task_wrapper();

        // Clean up any existing data
        if let Ok(mut conn) = get_test_connection(&broker).await {
            let _: Result<String, _> = redis::cmd("FLUSHDB").query_async(&mut conn).await;
        }

        let context = TaskExecutionContext {
            broker: broker.clone(),
            task_registry,
            worker_id,
            semaphore: None,
            active_tasks: Arc::new(AtomicUsize::new(0)),
        };

        let result = Worker::handle_backpressure(context, task_wrapper.clone()).await;

        match result {
            SpawnResult::Rejected(rejected_wrapper) => {
                assert_eq!(rejected_wrapper.metadata.id, task_wrapper.metadata.id);
            }
            _ => panic!("Expected rejected result"),
        }

        // Verify task was requeued
        let queue_size = broker
            .get_queue_size(queue_names::DEFAULT)
            .await
            .expect("Failed to get queue size");
        assert!(queue_size > 0);
    }
}
