use crate::{RedisBroker, TaskQueueError, TaskScheduler, TaskWrapper, queue::queue_names};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
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
        execution_context.broker.register_worker(&execution_context.worker_id).await?;

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

                        if let Err(e) = execution_context.broker.unregister_worker(&execution_context.worker_id).await {
                            #[cfg(feature = "tracing")]
                            tracing::error!("Failed to unregister worker {}: {}", execution_context.worker_id, e);
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
                                    SpawnResult::Rejected(rejected_task) => {
                                        // Task was rejected due to backpressure, will be re-queued
                                        #[cfg(feature = "tracing")]
                                        tracing::debug!("Task {} rejected due to backpressure", rejected_task.metadata.id);
                                    }
                                    SpawnResult::Failed(e) => {
                                        #[cfg(feature = "tracing")]
                                        tracing::error!("Failed to handle task execution: {}", e);
                                    }
                                }
                            }
                            Ok(None) => {
                                // No tasks available, update heartbeat
                                if let Err(e) = execution_context.broker.update_worker_heartbeat(&execution_context.worker_id).await {
                                    #[cfg(feature = "tracing")]
                                    tracing::error!("Failed to update heartbeat for worker {}: {}", execution_context.worker_id, e);
                                }
                            }
                            Err(e) => {
                                #[cfg(feature = "tracing")]
                                tracing::error!("Worker {} error dequeuing task: {}", execution_context.worker_id, e);

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
    async fn handle_task_execution(context: TaskExecutionContext, task_wrapper: TaskWrapper) -> SpawnResult {
        // Extract semaphore to avoid borrowing issues
        let semaphore_opt = context.semaphore.clone();
        
        match semaphore_opt {
            Some(semaphore) => {
                // Clone semaphore to avoid lifetime issues
                let semaphore_clone = semaphore.clone();
                
                // Try to acquire permit without blocking the main worker loop
                let permit_result = semaphore.try_acquire();
                match permit_result {
                    Ok(_permit) => {
                        // Drop the permit immediately to avoid lifetime issues
                        drop(_permit);
                        // Successfully acquired permit, spawn task execution
                        Self::spawn_task_with_permit(context, task_wrapper, semaphore_clone).await;
                        SpawnResult::Spawned
                    }
                    Err(_) => {
                        // At capacity, handle backpressure
                        Self::handle_backpressure(context, task_wrapper).await
                    }
                }
            }
            None => {
                // No semaphore configured, execute directly (blocking)
                Self::execute_task_directly(context, task_wrapper).await;
                SpawnResult::Spawned
            }
        }
    }

    /// Wait for active tasks to complete during graceful shutdown
    async fn graceful_shutdown(context: &TaskExecutionContext) {
        let timeout = tokio::time::Duration::from_secs(30); // Configurable timeout
        let start_time = tokio::time::Instant::now();

        while context.active_tasks.load(Ordering::Relaxed) > 0 && start_time.elapsed() < timeout {
            #[cfg(feature = "tracing")]
            tracing::info!(
                "Waiting for {} active tasks to complete...",
                context.active_tasks.load(Ordering::Relaxed)
            );
            
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        let remaining_tasks = context.active_tasks.load(Ordering::Relaxed);
        if remaining_tasks > 0 {
            #[cfg(feature = "tracing")]
            tracing::warn!(
                "Shutdown timeout reached with {} tasks still active",
                remaining_tasks
            );
        } else {
            #[cfg(feature = "tracing")]
            tracing::info!("All tasks completed during graceful shutdown");
        }
    }

    /// Spawn task execution with proper permit management and tracking
    async fn spawn_task_with_permit(
        context: TaskExecutionContext,
        task_wrapper: TaskWrapper,
        semaphore: Arc<Semaphore>,
    ) {
        // Increment active task counter
        context.active_tasks.fetch_add(1, Ordering::Relaxed);

        tokio::spawn(async move {
            // Acquire permit for this specific task execution
            let _permit = match semaphore.acquire().await {
                Ok(permit) => permit,
                Err(e) => {
                    #[cfg(feature = "tracing")]
                    tracing::error!(
                        "Worker {} failed to acquire semaphore permit: {}", 
                        context.worker_id, e
                    );
                    
                    // Decrement counter and re-enqueue task
                    context.active_tasks.fetch_sub(1, Ordering::Relaxed);
                    
                    if let Err(enqueue_err) = Self::requeue_task(&context.broker, task_wrapper).await {
                        #[cfg(feature = "tracing")]
                        tracing::error!("Failed to re-enqueue task after semaphore error: {}", enqueue_err);
                    }
                    return;
                }
            };

            // Execute the task with proper error handling
            if let Err(e) = Self::process_task(
                &context.broker,
                &context.task_registry,
                &context.worker_id,
                task_wrapper,
            ).await {
                #[cfg(feature = "tracing")]
                tracing::error!("Worker {} failed to process task: {}", context.worker_id, e);
            }

            // Decrement active task counter when done
            context.active_tasks.fetch_sub(1, Ordering::Relaxed);
            // Permit is automatically released when dropped
        });
    }

    /// Handle backpressure when at capacity
    async fn handle_backpressure(context: TaskExecutionContext, task_wrapper: TaskWrapper) -> SpawnResult {
        #[cfg(feature = "tracing")]
        tracing::warn!("Worker {} at max capacity, re-queuing task", context.worker_id);
        
        // Re-enqueue the task for later processing
        match Self::requeue_task(&context.broker, task_wrapper.clone()).await {
            Ok(()) => {
                // Brief delay to prevent tight loops and allow other tasks to complete
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                SpawnResult::Rejected(task_wrapper)
            }
            Err(e) => {
                #[cfg(feature = "tracing")]
                tracing::error!("Failed to re-enqueue task due to backpressure: {}", e);
                SpawnResult::Failed(e)
            }
        }
    }

    /// Execute task directly without spawning (for non-semaphore configurations)
    async fn execute_task_directly(context: TaskExecutionContext, task_wrapper: TaskWrapper) {
        // Still track the task even when executing directly
        context.active_tasks.fetch_add(1, Ordering::Relaxed);

        if let Err(e) = Self::process_task(
            &context.broker,
            &context.task_registry,
            &context.worker_id,
            task_wrapper,
        ).await {
            #[cfg(feature = "tracing")]
            tracing::error!("Worker {} failed to process task: {}", context.worker_id, e);
        }

        context.active_tasks.fetch_sub(1, Ordering::Relaxed);
    }

    /// Helper method to re-queue tasks with proper error handling
    async fn requeue_task(broker: &RedisBroker, task_wrapper: TaskWrapper) -> Result<(), TaskQueueError> {
        broker.enqueue_task_wrapper(task_wrapper, queue_names::DEFAULT).await.map(|_| ())
    }

    pub async fn stop(self) {
        if let Some(shutdown_tx) = self.shutdown_tx {
            let _ = shutdown_tx.send(()).await;
        }

        if let Some(handle) = self.handle {
            let _ = handle.await;
        }
    }

    async fn process_task(
        broker: &RedisBroker,
        task_registry: &crate::TaskRegistry,
        worker_id: &str,
        mut task_wrapper: TaskWrapper,
    ) -> Result<(), TaskQueueError> {
        task_wrapper.metadata.attempts += 1;

        #[cfg(feature = "tracing")]
        tracing::info!(
            "Worker {} processing task {} (attempt {}) - Task: {}",
            worker_id,
            task_wrapper.metadata.id,
            task_wrapper.metadata.attempts,
            task_wrapper.metadata.name
        );

        // Create a timeout for task execution
        let execution_future = Self::execute_task_with_registry(task_registry, &task_wrapper);
        let timeout_duration =
            tokio::time::Duration::from_secs(task_wrapper.metadata.timeout_seconds);

        match tokio::time::timeout(timeout_duration, execution_future).await {
            Ok(Ok(result)) => {
                #[cfg(feature = "tracing")]
                {
                    // Try to make the result more readable for logging
                    let result_display = if result.len() > 1000 {
                        format!("({} bytes) [truncated]", result.len())
                    } else if let Ok(utf8_str) = std::str::from_utf8(&result) {
                        // Try to display as UTF-8 string if possible
                        format!("\"{}\"", utf8_str)
                    } else if let Ok(msgpack_value) = rmp_serde::from_slice::<serde_json::Value>(&result) {
                        // Try to deserialize as MessagePack and display as JSON for readability
                        msgpack_value.to_string()
                    } else {
                        // Fallback to hex representation for better readability
                        format!("({} bytes) 0x{}", result.len(), hex::encode(&result[..std::cmp::min(50, result.len())]))
                    };
                    
                    tracing::info!(
                        "Task {} completed successfully: {}",
                        task_wrapper.metadata.id,
                        result_display
                    );
                }

                broker
                    .mark_task_completed(task_wrapper.metadata.id, queue_names::DEFAULT)
                    .await?;
                Ok(())
            }
            Ok(Err(e)) => {
                #[cfg(feature = "tracing")]
                tracing::error!("Task {} failed: {}", task_wrapper.metadata.id, e);

                if task_wrapper.metadata.attempts < task_wrapper.metadata.max_retries {
                    #[cfg(feature = "tracing")]
                    tracing::info!(
                        "Retrying task {} (attempt {})",
                        task_wrapper.metadata.id,
                        task_wrapper.metadata.attempts + 1
                    );

                    // Re-enqueue the task for retry
                    broker.enqueue_task_wrapper(task_wrapper, queue_names::DEFAULT).await?;
                } else {
                    #[cfg(feature = "tracing")]
                    tracing::error!("Task {} exceeded max retries", task_wrapper.metadata.id);

                    broker
                        .mark_task_failed(task_wrapper.metadata.id, queue_names::DEFAULT)
                        .await?;
                }

                Err(TaskQueueError::TaskExecution(e.to_string()))
            }
            Err(_) => {
                #[cfg(feature = "tracing")]
                tracing::error!(
                    "Task {} timed out after {} seconds",
                    task_wrapper.metadata.id,
                    task_wrapper.metadata.timeout_seconds
                );

                broker
                    .mark_task_failed(task_wrapper.metadata.id, queue_names::DEFAULT)
                    .await?;
                Err(TaskQueueError::TaskExecution("Task timeout".to_string()))
            }
        }
    }

    async fn execute_task_with_registry(
        task_registry: &crate::TaskRegistry,
        task_wrapper: &TaskWrapper,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let task_name = &task_wrapper.metadata.name;

        #[cfg(feature = "tracing")]
        tracing::debug!("Executing task '{}' with registry", task_name);

        // Try to execute using the task registry
        match task_registry
            .execute(task_name, task_wrapper.payload.clone())
            .await
        {
            Ok(result) => {
                #[cfg(feature = "tracing")]
                tracing::debug!("Task '{}' executed successfully via registry", task_name);
                Ok(result)
            }
            Err(e) => {
                let error_msg = e.to_string();
                
                // Check if this is a "task not found" error vs a task execution failure
                if error_msg.contains("Unknown task type") {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(
                        "Task '{}' not found in registry, falling back to default execution",
                        task_name
                    );

                    // Fallback: generic task execution
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    
                    // Create fallback response struct
                    #[derive(Serialize, Deserialize)]
                    struct FallbackResponse {
                        status: String,
                        message: String,
                        timestamp: String,
                    }
                    
                    let response = FallbackResponse {
                        status: "completed_fallback".to_string(),
                        message: format!("Task '{}' executed with fallback handler", task_name),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    };
                    
                    Ok(rmp_serde::to_vec(&response)?)
                } else {
                    // This is an actual task execution failure - propagate it
                    #[cfg(feature = "tracing")]
                    tracing::error!("Task '{}' failed during execution: {}", task_name, error_msg);
                    
                    Err(e)
                }
            }
        }
    }
}
