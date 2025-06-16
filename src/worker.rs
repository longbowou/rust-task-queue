use crate::{RedisBroker, TaskQueueError, TaskScheduler, TaskWrapper, queue::queue_names};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
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
}

/// Configuration for worker backpressure and performance tuning
#[derive(Debug, Clone)]
pub struct WorkerBackpressureConfig {
    pub max_concurrent_tasks: usize,
    pub queue_size_threshold: usize,
    pub backpressure_delay_ms: u64,
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

    pub async fn start(mut self) -> Result<Self, TaskQueueError> {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        let worker_id = self.id.clone();
        let broker = self.broker.clone();
        let task_registry = self.task_registry.clone();
        let semaphore = self.task_semaphore.clone();

        // Register worker
        broker.register_worker(&worker_id).await?;

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
                        tracing::info!("Worker {} shutting down", worker_id);

                        if let Err(e) = broker.unregister_worker(&worker_id).await {
                            #[cfg(feature = "tracing")]
                            tracing::error!("Failed to unregister worker {}: {}", worker_id, e);
                        }
                        break;
                    }

                    // Process tasks with backpressure control
                    task_result = broker.dequeue_task(&queues) => {
                        match task_result {
                            Ok(Some(task_wrapper)) => {
                                // Use semaphore for backpressure control
                                if let Some(sem) = semaphore.clone() {
                                    // Try to acquire permit without blocking
                                    if let Ok(_permit) = sem.try_acquire() {
                                        let broker_clone = broker.clone();
                                        let task_registry_clone = task_registry.clone();
                                        let worker_id_clone = worker_id.clone();
                                        let sem_clone = sem.clone();
                                        
                                        // Spawn task processing in background to avoid blocking main loop
                                        tokio::spawn(async move {
                                            // Acquire permit for this task - handle potential errors
                                            match sem_clone.acquire().await {
                                                Ok(_permit) => {
                                                    if let Err(e) = Worker::process_task(
                                                        &broker_clone,
                                                        &task_registry_clone,
                                                        &worker_id_clone,
                                                        task_wrapper,
                                                    ).await {
                                                        #[cfg(feature = "tracing")]
                                                        tracing::error!("Worker {} failed to process task: {}", worker_id_clone, e);
                                                    }
                                                    // Permit is automatically released when dropped
                                                }
                                                Err(e) => {
                                                    #[cfg(feature = "tracing")]
                                                    tracing::error!("Worker {} failed to acquire semaphore permit: {}", worker_id_clone, e);
                                                    
                                                    // Re-enqueue the task since we couldn't process it
                                                    if let Err(enqueue_err) = broker_clone.enqueue_task_wrapper(task_wrapper, queue_names::DEFAULT).await {
                                                        #[cfg(feature = "tracing")]
                                                        tracing::error!("Failed to re-enqueue task after semaphore error: {}", enqueue_err);
                                                    }
                                                }
                                            }
                                        });
                                    } else {
                                        #[cfg(feature = "tracing")]
                                        tracing::warn!("Worker {} at max capacity, delaying task", worker_id);
                                        
                                        // Put task back on queue for later processing
                                        if let Err(e) = broker.enqueue_task_wrapper(task_wrapper, queue_names::DEFAULT).await {
                                            #[cfg(feature = "tracing")]
                                            tracing::error!("Failed to re-enqueue task: {}", e);
                                        }
                                        
                                        // Brief delay to prevent tight loop
                                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                                    }
                                } else {
                                    // Fallback to original behavior if no semaphore
                                    if let Err(e) = Worker::process_task(&broker, &task_registry, &worker_id, task_wrapper).await {
                                        #[cfg(feature = "tracing")]
                                        tracing::error!("Worker {} failed to process task: {}", worker_id, e);
                                    }
                                }
                            }
                            Ok(None) => {
                                // No tasks available, update heartbeat
                                if let Err(e) = broker.update_worker_heartbeat(&worker_id).await {
                                    #[cfg(feature = "tracing")]
                                    tracing::error!("Failed to update heartbeat for worker {}: {}", worker_id, e);
                                }
                            }
                            Err(e) => {
                                #[cfg(feature = "tracing")]
                                tracing::error!("Worker {} error dequeuing task: {}", worker_id, e);

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
