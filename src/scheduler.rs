use crate::{RedisBroker, Task, TaskId, TaskQueueError};
use chrono::{DateTime, Duration, Utc};
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
        let execute_at = Utc::now() + delay;
        let task_id = uuid::Uuid::new_v4();

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
            serialized,
            execute_at.timestamp(),
        )
        .await?;

        #[cfg(feature = "tracing")]
        tracing::info!("Scheduled task {} to execute at {}", task_id, execute_at);

        Ok(task_id)
    }

    pub async fn process_scheduled_tasks(broker: &RedisBroker) -> Result<(), TaskQueueError> {
        let mut conn = broker.pool.get().await.map_err(|e| {
            TaskQueueError::Connection(format!("Failed to get Redis connection: {}", e))
        })?;
        let now = Utc::now().timestamp();

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

        for serialized_task in tasks {
            if let Ok(scheduled_task) = rmp_serde::from_slice::<ScheduledTask>(&serialized_task) {
                // Move task to regular queue
                let task_wrapper = crate::TaskWrapper {
                    metadata: crate::TaskMetadata {
                        id: scheduled_task.id,
                        name: scheduled_task.task_name,
                        created_at: Utc::now(),
                        attempts: 0,
                        max_retries: scheduled_task.max_retries,
                        timeout_seconds: scheduled_task.timeout_seconds,
                    },
                    payload: scheduled_task.payload,
                };

                let serialized_wrapper = rmp_serde::to_vec(&task_wrapper)?;
                redis::AsyncCommands::lpush::<_, _, ()>(
                    &mut *conn,
                    &scheduled_task.queue,
                    &serialized_wrapper,
                )
                .await?;

                // Remove from scheduled tasks
                redis::AsyncCommands::zrem::<_, _, ()>(
                    &mut *conn,
                    "scheduled_tasks",
                    &serialized_task,
                )
                .await?;

                #[cfg(feature = "tracing")]
                tracing::info!(
                    "Moved scheduled task {} to queue {}",
                    scheduled_task.id,
                    scheduled_task.queue
                );
            }
        }

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
