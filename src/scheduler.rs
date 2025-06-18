use crate::{RedisBroker, Task, TaskId, TaskQueueError};
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::task::JoinHandle;

pub struct TaskScheduler {
    broker: Arc<RedisBroker>,
    tick_interval_seconds: u64,
}

impl TaskScheduler {
    pub fn new(broker: Arc<RedisBroker>) -> Self {
        Self {
            broker,
            tick_interval_seconds: 1, // Default to 1 second for better responsiveness
        }
    }

    pub fn with_tick_interval(broker: Arc<RedisBroker>, interval_seconds: u64) -> Self {
        Self {
            broker,
            tick_interval_seconds: interval_seconds,
        }
    }

    pub fn start(broker: Arc<RedisBroker>) -> Result<JoinHandle<()>, TaskQueueError> {
        let scheduler = Self::new(broker);
        let tick_interval = scheduler.tick_interval_seconds;
        let broker = scheduler.broker;

        let handle = tokio::spawn(async move {
            loop {
                if let Err(e) = Self::process_scheduled_tasks(&broker).await {
                    #[cfg(feature = "tracing")]
                    tracing::error!("Scheduler error: {}", e);
                }

                tokio::time::sleep(tokio::time::Duration::from_secs(tick_interval)).await;
            }
        });

        #[cfg(feature = "tracing")]
        tracing::info!(
            "Task scheduler started with {} second tick interval",
            tick_interval
        );

        Ok(handle)
    }

    pub async fn schedule_task<T: Task>(
        &self,
        task: T,
        queue: &str,
        delay: Duration,
    ) -> Result<TaskId, TaskQueueError> {
        let schedule_start = std::time::Instant::now();
        let execute_at = Utc::now() + delay;
        let task_id = uuid::Uuid::new_v4();
        let task_name = task.name();

        #[cfg(feature = "tracing")]
        tracing::info!(
            task_id = %task_id,
            task_name = task_name,
            queue = queue,
            delay_seconds = delay.num_seconds(),
            execute_at = %execute_at,
            "Scheduling task for delayed execution"
        );

        let scheduled_task = ScheduledTask {
            id: task_id,
            queue: queue.to_string(),
            execute_at,
            task_name: task.name().to_string(),
            max_retries: task.max_retries(),
            timeout_seconds: task.timeout_seconds(),
            payload: rmp_serde::to_vec(&task)?,
        };

        // Store in Redis sorted set with timestamp as score
        let mut conn = self.broker.pool.get().await.map_err(|e| {
            TaskQueueError::Connection(format!("Failed to get Redis connection: {}", e))
        })?;
        let serialized = rmp_serde::to_vec(&scheduled_task)?;

        redis::AsyncCommands::zadd::<_, _, _, ()>(
            &mut *conn,
            "scheduled_tasks",
            serialized.clone(),
            execute_at.timestamp(),
        )
        .await?;

        #[cfg(feature = "tracing")]
        tracing::info!(
            task_id = %task_id,
            task_name = task_name,
            queue = queue,
            execute_at = %execute_at,
            delay_seconds = delay.num_seconds(),
            duration_ms = schedule_start.elapsed().as_millis(),
            payload_size_bytes = serialized.len(),
            redis_score = execute_at.timestamp(),
            "Task scheduled successfully"
        );

        Ok(task_id)
    }

    pub async fn process_scheduled_tasks(broker: &RedisBroker) -> Result<(), TaskQueueError> {
        let process_start = std::time::Instant::now();
        let mut conn = broker.pool.get().await.map_err(|e| {
            TaskQueueError::Connection(format!("Failed to get Redis connection: {}", e))
        })?;
        let now = Utc::now().timestamp();

        #[cfg(feature = "tracing")]
        tracing::debug!(
            timestamp_threshold = now,
            "Processing scheduled tasks ready for execution"
        );

        // Get tasks that should execute now
        let tasks: Vec<Vec<u8>> = redis::AsyncCommands::zrangebyscore_limit(
            &mut *conn,
            "scheduled_tasks",
            0,
            now,
            0,
            100,
        )
        .await?;

        let batch_size = tasks.len();

        if batch_size == 0 {
            #[cfg(feature = "tracing")]
            tracing::trace!(
                duration_ms = process_start.elapsed().as_millis(),
                "No scheduled tasks ready for execution"
            );
            return Ok(());
        }

        #[cfg(feature = "tracing")]
        tracing::info!(
            batch_size = batch_size,
            timestamp_threshold = now,
            "Found scheduled tasks ready for execution"
        );

        let mut processed_count = 0;
        let mut failed_count = 0;
        let mut queue_distribution = HashMap::new();

        for serialized_task in tasks {
            if let Ok(scheduled_task) = rmp_serde::from_slice::<ScheduledTask>(&serialized_task) {
                // Track queue distribution
                *queue_distribution
                    .entry(scheduled_task.queue.clone())
                    .or_insert(0) += 1;

                #[cfg(feature = "tracing")]
                tracing::debug!(
                    task_id = %scheduled_task.id,
                    task_name = %scheduled_task.task_name,
                    queue = %scheduled_task.queue,
                    scheduled_execute_at = %scheduled_task.execute_at,
                    delay_from_scheduled = (now - scheduled_task.execute_at.timestamp()),
                    "Moving scheduled task to regular queue"
                );

                // Move task to regular queue
                let task_wrapper = crate::TaskWrapper {
                    metadata: crate::TaskMetadata {
                        id: scheduled_task.id,
                        name: scheduled_task.task_name.clone(),
                        created_at: Utc::now(),
                        attempts: 0,
                        max_retries: scheduled_task.max_retries,
                        timeout_seconds: scheduled_task.timeout_seconds,
                    },
                    payload: scheduled_task.payload,
                };

                let serialized_wrapper = rmp_serde::to_vec(&task_wrapper)?;

                match redis::AsyncCommands::lpush::<_, _, ()>(
                    &mut *conn,
                    &scheduled_task.queue,
                    &serialized_wrapper,
                )
                .await
                {
                    Ok(_) => {
                        // Remove from scheduled tasks
                        match redis::AsyncCommands::zrem::<_, _, ()>(
                            &mut *conn,
                            "scheduled_tasks",
                            &serialized_task,
                        )
                        .await
                        {
                            Ok(_) => {
                                processed_count += 1;

                                #[cfg(feature = "tracing")]
                                tracing::info!(
                                    task_id = %scheduled_task.id,
                                    task_name = %scheduled_task.task_name,
                                    queue = %scheduled_task.queue,
                                    delay_from_scheduled_seconds = (now - scheduled_task.execute_at.timestamp()),
                                    payload_size_bytes = serialized_wrapper.len(),
                                    "Scheduled task moved to regular queue successfully"
                                );
                            }
                            Err(e) => {
                                failed_count += 1;
                                #[cfg(feature = "tracing")]
                                tracing::error!(
                                    task_id = %scheduled_task.id,
                                    task_name = %scheduled_task.task_name,
                                    error = %e,
                                    "Failed to remove task from scheduled tasks set"
                                );
                            }
                        }
                    }
                    Err(e) => {
                        failed_count += 1;
                        #[cfg(feature = "tracing")]
                        tracing::error!(
                            task_id = %scheduled_task.id,
                            task_name = %scheduled_task.task_name,
                            queue = %scheduled_task.queue,
                            error = %e,
                            "Failed to push scheduled task to regular queue"
                        );
                    }
                }
            } else {
                failed_count += 1;
                #[cfg(feature = "tracing")]
                tracing::error!(
                    payload_size_bytes = serialized_task.len(),
                    "Failed to deserialize scheduled task"
                );
            }
        }

        #[cfg(feature = "tracing")]
        tracing::info!(
            total_processed = processed_count,
            failed_count = failed_count,
            batch_size = batch_size,
            duration_ms = process_start.elapsed().as_millis(),
            queue_distribution = ?queue_distribution,
            success_rate = if batch_size > 0 { processed_count as f64 / batch_size as f64 } else { 0.0 },
            "Scheduled task processing batch completed"
        );

        Ok(())
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ScheduledTask {
    id: TaskId,
    queue: String,
    execute_at: DateTime<Utc>,
    task_name: String,
    max_retries: u32,
    timeout_seconds: u64,
    payload: Vec<u8>,
}
