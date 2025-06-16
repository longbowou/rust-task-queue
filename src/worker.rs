use crate::{RedisBroker, TaskQueueError, TaskScheduler, TaskWrapper};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub struct Worker {
    id: String,
    broker: Arc<RedisBroker>,
    #[allow(dead_code)]
    scheduler: Arc<TaskScheduler>,
    task_registry: Arc<crate::TaskRegistry>,
    shutdown_tx: Option<mpsc::Sender<()>>,
    handle: Option<JoinHandle<()>>,
}

impl Worker {
    pub fn new(id: String, broker: Arc<RedisBroker>, scheduler: Arc<TaskScheduler>) -> Self {
        Self {
            id,
            broker,
            scheduler,
            task_registry: Arc::new(crate::TaskRegistry::new()),
            shutdown_tx: None,
            handle: None,
        }
    }

    pub fn with_task_registry(mut self, registry: Arc<crate::TaskRegistry>) -> Self {
        self.task_registry = registry;
        self
    }

    pub async fn start(mut self) -> Result<Self, TaskQueueError> {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        let worker_id = self.id.clone();
        let broker = self.broker.clone();
        let task_registry = self.task_registry.clone();

        // Register worker
        broker.register_worker(&worker_id).await?;

        let handle = tokio::spawn(async move {
            let queues = vec![
                "default".to_string(),
                "high_priority".to_string(),
                "low_priority".to_string(),
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

                    // Process tasks
                    task_result = broker.dequeue_task(&queues) => {
                        match task_result {
                            Ok(Some(task_wrapper)) => {
                                if let Err(e) = Worker::process_task(&broker, &task_registry, &worker_id, task_wrapper).await {
                                    #[cfg(feature = "tracing")]
                                    tracing::error!("Worker {} failed to process task: {}", worker_id, e);
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
                tracing::info!(
                    "Task {} completed successfully: {:?}",
                    task_wrapper.metadata.id,
                    result
                );

                broker
                    .mark_task_completed(task_wrapper.metadata.id, "default")
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
                } else {
                    #[cfg(feature = "tracing")]
                    tracing::error!("Task {} exceeded max retries", task_wrapper.metadata.id);

                    broker
                        .mark_task_failed(task_wrapper.metadata.id, "default")
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
                    .mark_task_failed(task_wrapper.metadata.id, "default")
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
            Err(_) => {
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
            }
        }
    }
}
