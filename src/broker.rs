use crate::{Task, TaskId, TaskMetadata, TaskQueueError, TaskWrapper};
use deadpool_redis::{Config, Pool, Runtime};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

pub struct RedisBroker {
    pub(crate) pool: Pool,
}

impl RedisBroker {
    pub async fn new(redis_url: &str) -> Result<Self, TaskQueueError> {
        let cfg = Config::from_url(redis_url);
        let pool = cfg.create_pool(Some(Runtime::Tokio1))
            .map_err(|e| TaskQueueError::Connection(format!("Failed to create Redis pool: {}", e)))?;

        // Test the connection
        let mut conn = pool.get().await?;
        let _: String = redis::cmd("PING").query_async::<_, String>(&mut *conn).await?;

        Ok(Self { pool })
    }

    pub async fn new_with_config(redis_url: &str, pool_size: Option<usize>) -> Result<Self, TaskQueueError> {
        let mut cfg = Config::from_url(redis_url);
        if let Some(size) = pool_size {
            cfg.pool = Some(deadpool_redis::PoolConfig::new(size));
        }
        
        let pool = cfg.create_pool(Some(Runtime::Tokio1))
            .map_err(|e| TaskQueueError::Connection(format!("Failed to create Redis pool: {}", e)))?;

        // Test the connection
        let mut conn = pool.get().await?;
        let _: String = redis::cmd("PING").query_async::<_, String>(&mut *conn).await?;

        Ok(Self { pool })
    }

    async fn get_connection(&self) -> Result<deadpool_redis::Connection, TaskQueueError> {
        self.pool.get().await.map_err(TaskQueueError::Pool)
    }

    pub async fn enqueue_task<T: Task>(
        &self,
        task: T,
        queue: &str,
    ) -> Result<TaskId, TaskQueueError> {
        let metadata = TaskMetadata {
            id: uuid::Uuid::new_v4(),
            name: task.name().to_string(),
            created_at: chrono::Utc::now(),
            attempts: 0,
            max_retries: task.max_retries(),
            timeout_seconds: task.timeout_seconds(),
        };

        let task_wrapper = TaskWrapper {
            metadata: metadata.clone(),
            payload: rmp_serde::to_vec(&task)?,
        };

        let serialized = rmp_serde::to_vec(&task_wrapper)?;
        let mut conn = self.pool.get().await
            .map_err(|e| TaskQueueError::Connection(format!("Failed to get Redis connection: {}", e)))?;

        // Push to the queue
        conn.lpush::<_, _, ()>(queue, serialized).await?;

        // Store task metadata for monitoring
        let metadata_key = format!("task:{}:metadata", metadata.id);
        conn.set::<_, _, ()>(&metadata_key, rmp_serde::to_vec(&metadata)?)
            .await?;
        conn.expire::<_, ()>(&metadata_key, 3600).await?; // Expire after 1 hour

        // Update queue metrics
        let queue_size_key = format!("queue:{}:size", queue);
        conn.incr::<_, _, ()>(&queue_size_key, 1).await?;

        #[cfg(feature = "tracing")]
        tracing::info!("Enqueued task {} to queue {}", metadata.id, queue);

        Ok(metadata.id)
    }

    /// Enqueue a pre-constructed task wrapper (used for re-enqueueing)
    pub async fn enqueue_task_wrapper(
        &self,
        task_wrapper: TaskWrapper,
        queue: &str,
    ) -> Result<TaskId, TaskQueueError> {
        let serialized = rmp_serde::to_vec(&task_wrapper)?;
        let mut conn = self.pool.get().await
            .map_err(|e| TaskQueueError::Connection(format!("Failed to get Redis connection: {}", e)))?;

        // Push to the queue
        conn.lpush::<_, _, ()>(queue, serialized).await?;

        // Update queue metrics
        let queue_size_key = format!("queue:{}:size", queue);
        conn.incr::<_, _, ()>(&queue_size_key, 1).await?;

        #[cfg(feature = "tracing")]
        tracing::info!("Re-enqueued task {} to queue {}", task_wrapper.metadata.id, queue);

        Ok(task_wrapper.metadata.id)
    }

    pub async fn dequeue_task(
        &self,
        queues: &[String],
    ) -> Result<Option<TaskWrapper>, TaskQueueError> {
        let mut conn = self.pool.get().await
            .map_err(|e| TaskQueueError::Connection(format!("Failed to get Redis connection: {}", e)))?;

        // Use BRPOP for blocking pop with timeout
        let result: Option<(String, Vec<u8>)> = conn.brpop(queues, 5f64).await?;

        if let Some((queue, serialized)) = result {
            let task_wrapper: TaskWrapper = rmp_serde::from_slice(&serialized)?;

            // Update queue metrics
            let queue_size_key = format!("queue:{}:size", queue);
            conn.decr::<_, _, ()>(&queue_size_key, 1).await?;

            #[cfg(feature = "tracing")]
            tracing::debug!(
                "Dequeued task {} from queue {}",
                task_wrapper.metadata.id,
                queue
            );

            Ok(Some(task_wrapper))
        } else {
            Ok(None)
        }
    }

    pub async fn get_queue_size(&self, queue: &str) -> Result<i64, TaskQueueError> {
        let mut conn = self.pool.get().await
            .map_err(|e| TaskQueueError::Connection(format!("Failed to get Redis connection: {}", e)))?;
        let size: i64 = conn.llen(queue).await?;
        Ok(size)
    }

    pub async fn get_queue_metrics(&self, queue: &str) -> Result<QueueMetrics, TaskQueueError> {
        let mut conn = self.pool.get().await
            .map_err(|e| TaskQueueError::Connection(format!("Failed to get Redis connection: {}", e)))?;

        let size: i64 = conn.llen(queue).await?;
        let processed_key = format!("queue:{}:processed", queue);
        let failed_key = format!("queue:{}:failed", queue);

        let processed: i64 = conn.get(&processed_key).await.unwrap_or(0);
        let failed: i64 = conn.get(&failed_key).await.unwrap_or(0);

        Ok(QueueMetrics {
            queue_name: queue.to_string(),
            pending_tasks: size,
            processed_tasks: processed,
            failed_tasks: failed,
        })
    }

    pub async fn mark_task_completed(
        &self,
        task_id: TaskId,
        queue: &str,
    ) -> Result<(), TaskQueueError> {
        let mut conn = self.pool.get().await
            .map_err(|e| TaskQueueError::Connection(format!("Failed to get Redis connection: {}", e)))?;
        let processed_key = format!("queue:{}:processed", queue);
        conn.incr::<_, _, ()>(&processed_key, 1).await?;

        // Remove task metadata
        let metadata_key = format!("task:{}:metadata", task_id);
        conn.del::<_, ()>(&metadata_key).await?;

        Ok(())
    }

    pub async fn mark_task_failed_with_reason(
        &self,
        task_id: TaskId,
        queue: &str,
        reason: Option<String>,
    ) -> Result<(), TaskQueueError> {
        let mut conn = self.pool.get().await
            .map_err(|e| TaskQueueError::Connection(format!("Failed to get Redis connection: {}", e)))?;

        // Increment the failed counter for queue metrics
        let failed_key = format!("queue:{}:failed", queue);
        conn.incr::<_, _, ()>(&failed_key, 1).await?;

        let default_reason = reason.unwrap_or_else(|| "Unknown error".to_string());

        // Store detailed failure information
        let failure_key = format!("task:{}:failure", task_id);
        let failure_info = TaskFailureInfo {
            task_id,
            queue: queue.to_string(),
            failed_at: chrono::Utc::now().to_rfc3339(),
            reason: default_reason.clone(),
            status: "failed".to_string(),
        };

        // Store failure info with expiration
        conn.set::<_, _, ()>(&failure_key, rmp_serde::to_vec(&failure_info)?)
            .await?;
        conn.expire::<_, ()>(&failure_key, 86400).await?;

        // Add to failed tasks set for monitoring
        let queue_failed_set = format!("queue:{}:failed_tasks", queue);
        conn.sadd::<_, _, ()>(&queue_failed_set, task_id.to_string())
            .await?;
        conn.expire::<_, ()>(&queue_failed_set, 86400).await?;

        // Clean up task metadata
        let metadata_key = format!("task:{}:metadata", task_id);
        conn.del::<_, ()>(&metadata_key).await?;

        #[cfg(feature = "tracing")]
        tracing::warn!(
            "Task {} marked as failed in queue {} - Reason: {}",
            task_id,
            queue,
            default_reason
        );

        Ok(())
    }

    // Keep the original method for backwards compatibility
    pub async fn mark_task_failed(
        &self,
        task_id: TaskId,
        queue: &str,
    ) -> Result<(), TaskQueueError> {
        self.mark_task_failed_with_reason(task_id, queue, None)
            .await
    }

    pub async fn get_active_workers(&self) -> Result<i64, TaskQueueError> {
        let mut conn = self.pool.get().await
            .map_err(|e| TaskQueueError::Connection(format!("Failed to get Redis connection: {}", e)))?;
        let count: i64 = conn.scard("active_workers").await?;
        Ok(count)
    }

    pub async fn register_worker(&self, worker_id: &str) -> Result<(), TaskQueueError> {
        let mut conn = self.pool.get().await
            .map_err(|e| TaskQueueError::Connection(format!("Failed to get Redis connection: {}", e)))?;
        conn.sadd::<_, _, ()>("active_workers", worker_id).await?;
        conn.set::<_, _, ()>(
            &format!("worker:{}:heartbeat", worker_id),
            chrono::Utc::now().timestamp(),
        )
        .await?;
        Ok(())
    }

    pub async fn unregister_worker(&self, worker_id: &str) -> Result<(), TaskQueueError> {
        let mut conn = self.pool.get().await
            .map_err(|e| TaskQueueError::Connection(format!("Failed to get Redis connection: {}", e)))?;
        conn.srem::<_, _, ()>("active_workers", worker_id).await?;
        conn.del::<_, ()>(&format!("worker:{}:heartbeat", worker_id))
            .await?;
        Ok(())
    }

    pub async fn update_worker_heartbeat(&self, worker_id: &str) -> Result<(), TaskQueueError> {
        let mut conn = self.pool.get().await
            .map_err(|e| TaskQueueError::Connection(format!("Failed to get Redis connection: {}", e)))?;
        conn.set::<_, _, ()>(
            &format!("worker:{}:heartbeat", worker_id),
            chrono::Utc::now().timestamp(),
        )
        .await?;
        Ok(())
    }

    /// Get failure information for a specific task
    pub async fn get_task_failure_info(
        &self,
        task_id: TaskId,
    ) -> Result<Option<TaskFailureInfo>, TaskQueueError> {
        let mut conn = self.pool.get().await
            .map_err(|e| TaskQueueError::Connection(format!("Failed to get Redis connection: {}", e)))?;
        let failure_key = format!("task:{}:failure", task_id);

        let failure_info: Option<Vec<u8>> = conn.get(&failure_key).await?;

        if let Some(info) = failure_info {
            let parsed: TaskFailureInfo = rmp_serde::from_slice(&info)?;
            Ok(Some(parsed))
        } else {
            Ok(None)
        }
    }

    /// Get list of failed task IDs for a queue
    pub async fn get_failed_tasks(&self, queue: &str) -> Result<Vec<String>, TaskQueueError> {
        let mut conn = self.pool.get().await
            .map_err(|e| TaskQueueError::Connection(format!("Failed to get Redis connection: {}", e)))?;
        let queue_failed_set = format!("queue:{}:failed_tasks", queue);

        let failed_tasks: Vec<String> = conn.smembers(&queue_failed_set).await?;
        Ok(failed_tasks)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFailureInfo {
    pub task_id: TaskId,
    pub queue: String,
    pub failed_at: String,
    pub reason: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueMetrics {
    pub queue_name: String,
    pub pending_tasks: i64,
    pub processed_tasks: i64,
    pub failed_tasks: i64,
}
