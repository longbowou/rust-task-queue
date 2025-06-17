use crate::{Task, TaskId, TaskMetadata, TaskQueueError, TaskWrapper};
use deadpool_redis::{Config, Pool, Runtime};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

pub struct RedisBroker {
    pub(crate) pool: Pool,
}

impl RedisBroker {
    pub async fn new(redis_url: &str) -> Result<Self, TaskQueueError> {
        Self::new_with_config(redis_url, None).await
    }

    pub async fn new_with_config(redis_url: &str, pool_size: Option<usize>) -> Result<Self, TaskQueueError> {
        let mut config = Config::from_url(redis_url);
        if let Some(size) = pool_size {
            config.pool = Some(deadpool_redis::PoolConfig::new(size));
        }
        
        let pool = config
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| TaskQueueError::Connection(format!("Failed to create Redis pool: {}", e)))?;

        // Test connection
        let mut conn = pool.get().await.map_err(|e| {
            TaskQueueError::Connection(format!("Failed to connect to Redis: {}", e))
        })?;

        // Verify Redis connection with a simple ping
        redis::cmd("PING").query_async::<_, String>(&mut conn).await.map_err(|e| {
            TaskQueueError::Connection(format!("Redis connection test failed: {}", e))
        })?;

        Ok(Self { pool })
    }

    async fn get_conn(&self) -> Result<deadpool_redis::Connection, TaskQueueError> {
        self.pool.get().await.map_err(|e| TaskQueueError::Connection(e.to_string()))
    }

    pub async fn enqueue_task<T: Task>(
        &self,
        task: T,
        queue: &str,
    ) -> Result<TaskId, TaskQueueError> {
        let task_id = TaskId::new_v4();
        
        // Create task metadata
        let metadata = TaskMetadata {
            id: task_id,
            name: task.name().to_string(),
            created_at: chrono::Utc::now(),
            attempts: 0,
            max_retries: task.max_retries(),
            timeout_seconds: task.timeout_seconds(),
        };

        // Serialize the task
        let payload = rmp_serde::to_vec(&task)?;

        let task_wrapper = TaskWrapper {
            metadata: metadata.clone(),
            payload,
        };

        self.enqueue_task_wrapper(task_wrapper, queue).await?;

        Ok(task_id)
    }

    pub async fn enqueue_task_wrapper(
        &self,
        task_wrapper: TaskWrapper,
        queue: &str,
    ) -> Result<TaskId, TaskQueueError> {
        let mut conn = self.get_conn().await?;

        // Serialize the task wrapper
        let serialized = rmp_serde::to_vec(&task_wrapper)?;

        // Push to the queue (left push for FIFO with right pop)
        conn.lpush::<_, _, ()>(queue, &serialized).await?;

        // Update queue size metric
        let queue_size_key = format!("queue:{}:size", queue);
        conn.incr::<_, _, ()>(&queue_size_key, 1).await?;

        // Store task metadata for tracking
        let metadata_key = format!("task:{}:metadata", task_wrapper.metadata.id);
        conn.set::<_, _, ()>(&metadata_key, rmp_serde::to_vec(&task_wrapper.metadata)?)
            .await?;
        conn.expire::<_, ()>(&metadata_key, 3600).await?; // 1 hour TTL

        #[cfg(feature = "tracing")]
        tracing::debug!(
            "Enqueued task {} to queue {}",
            task_wrapper.metadata.id,
            queue
        );

        Ok(task_wrapper.metadata.id)
    }

    pub async fn dequeue_task(
        &self,
        queues: &[String],
    ) -> Result<Option<TaskWrapper>, TaskQueueError> {
        let mut conn = self.get_conn().await?;

        // Use BRPOP for blocking right pop (FIFO with LPUSH)
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
        let mut conn = self.get_conn().await?;
        let size: i64 = conn.llen(queue).await?;
        Ok(size)
    }

    pub async fn get_queue_metrics(&self, queue: &str) -> Result<QueueMetrics, TaskQueueError> {
        let mut conn = self.get_conn().await?;

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
        let mut conn = self.get_conn().await?;
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
        let mut conn = self.get_conn().await?;

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
        self.mark_task_failed_with_reason(task_id, queue, None).await
    }

    pub async fn get_active_workers(&self) -> Result<i64, TaskQueueError> {
        let mut conn = self.get_conn().await?;
        let count: i64 = conn.scard("active_workers").await?;
        Ok(count)
    }

    pub async fn register_worker(&self, worker_id: &str) -> Result<(), TaskQueueError> {
        let mut conn = self.get_conn().await?;
        conn.sadd::<_, _, ()>("active_workers", worker_id).await?;
        
        // Set heartbeat
        let heartbeat_key = format!("worker:{}:heartbeat", worker_id);
        conn.set::<_, _, ()>(&heartbeat_key, chrono::Utc::now().to_rfc3339()).await?;
        conn.expire::<_, ()>(&heartbeat_key, 60).await?;
        
        Ok(())
    }

    pub async fn unregister_worker(&self, worker_id: &str) -> Result<(), TaskQueueError> {
        let mut conn = self.get_conn().await?;
        conn.srem::<_, _, ()>("active_workers", worker_id).await?;
        
        // Clean up heartbeat
        let heartbeat_key = format!("worker:{}:heartbeat", worker_id);
        conn.del::<_, ()>(&heartbeat_key).await?;
        
        Ok(())
    }

    pub async fn update_worker_heartbeat(&self, worker_id: &str) -> Result<(), TaskQueueError> {
        let mut conn = self.get_conn().await?;
        let heartbeat_key = format!("worker:{}:heartbeat", worker_id);
        conn.set::<_, _, ()>(&heartbeat_key, chrono::Utc::now().to_rfc3339()).await?;
        conn.expire::<_, ()>(&heartbeat_key, 60).await?;
        Ok(())
    }

    pub async fn get_task_failure_info(
        &self,
        task_id: TaskId,
    ) -> Result<Option<TaskFailureInfo>, TaskQueueError> {
        let mut conn = self.get_conn().await?;
        let failure_key = format!("task:{}:failure", task_id);
        
        if let Ok(data) = conn.get::<_, Vec<u8>>(&failure_key).await {
            match rmp_serde::from_slice::<TaskFailureInfo>(&data) {
                Ok(info) => Ok(Some(info)),
                Err(_) => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    pub async fn get_failed_tasks(&self, queue: &str) -> Result<Vec<String>, TaskQueueError> {
        let mut conn = self.get_conn().await?;
        let queue_failed_set = format!("queue:{}:failed_tasks", queue);
        let failed_tasks: Vec<String> = conn.smembers(&queue_failed_set).await.unwrap_or_default();
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    
    #[derive(Debug, Serialize, Deserialize, Clone)]
    struct TestTask {
        data: String,
    }

    #[async_trait::async_trait]
    impl Task for TestTask {
        async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
            Ok(self.data.as_bytes().to_vec())
        }

        fn name(&self) -> &str {
            "test_task"
        }
    }

    fn get_test_redis_url() -> String {
        std::env::var("REDIS_TEST_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/15".to_string())
    }

    async fn create_test_broker() -> RedisBroker {
        let redis_url = get_test_redis_url();
        RedisBroker::new(&redis_url).await.expect("Failed to create test broker")
    }

    async fn cleanup_test_data(broker: &RedisBroker) {
        if let Ok(mut conn) = broker.get_conn().await {
            let _: Result<String, _> = redis::cmd("FLUSHDB").query_async(&mut conn).await;
        }
    }

    #[tokio::test]
    async fn test_broker_creation() {
        let broker = create_test_broker().await;
        cleanup_test_data(&broker).await;
        
        // Broker should be created successfully and connection should work
        assert!(broker.get_conn().await.is_ok());
    }

    #[tokio::test]
    async fn test_broker_creation_with_config() {
        let redis_url = get_test_redis_url();
        let broker = RedisBroker::new_with_config(&redis_url, Some(5)).await.expect("Failed to create broker");
        cleanup_test_data(&broker).await;
        
        assert!(broker.get_conn().await.is_ok());
    }

    #[tokio::test]
    async fn test_broker_invalid_url() {
        let result = RedisBroker::new("redis://invalid-host:6379").await;
        assert!(result.is_err());
        
        if let Err(e) = result {
            assert!(matches!(e, TaskQueueError::Connection(_)));
        }
    }

    #[tokio::test]
    async fn test_enqueue_task() {
        let broker = create_test_broker().await;
        cleanup_test_data(&broker).await;
        
        let task = TestTask {
            data: "test data".to_string(),
        };
        
        let task_id = broker.enqueue_task(task, "test_queue").await.expect("Failed to enqueue task");
        
        // Verify task was enqueued
        let queue_size = broker.get_queue_size("test_queue").await.expect("Failed to get queue size");
        assert_eq!(queue_size, 1);
        
        // Verify task ID was generated
        assert!(!task_id.to_string().is_empty());
        
        cleanup_test_data(&broker).await;
    }

    #[tokio::test]
    async fn test_dequeue_task() {
        let broker = create_test_broker().await;
        cleanup_test_data(&broker).await;
        
        let task = TestTask {
            data: "test data".to_string(),
        };
        
        let task_id = broker.enqueue_task(task, "test_queue").await.expect("Failed to enqueue task");
        
        // Dequeue the task
        let queues = vec!["test_queue".to_string()];
        let dequeued = broker.dequeue_task(&queues).await.expect("Failed to dequeue task");
        
        assert!(dequeued.is_some());
        let task_wrapper = dequeued.unwrap();
        assert_eq!(task_wrapper.metadata.id, task_id);
        assert_eq!(task_wrapper.metadata.name, "test_task");
        
        // Queue should be empty now
        let queue_size = broker.get_queue_size("test_queue").await.expect("Failed to get queue size");
        assert_eq!(queue_size, 0);
        
        cleanup_test_data(&broker).await;
    }

    #[tokio::test]
    async fn test_dequeue_from_empty_queue() {
        let broker = create_test_broker().await;
        cleanup_test_data(&broker).await;
        
        let queues = vec!["empty_queue".to_string()];
        
        // Should timeout and return None
        let start = std::time::Instant::now();
        let result = broker.dequeue_task(&queues).await.expect("Failed to dequeue from empty queue");
        let elapsed = start.elapsed();
        
        assert!(result.is_none());
        // Should have waited approximately 5 seconds (the timeout)
        assert!(elapsed.as_secs() >= 4 && elapsed.as_secs() <= 6);
        
        cleanup_test_data(&broker).await;
    }

    #[tokio::test]
    async fn test_queue_metrics() {
        let broker = create_test_broker().await;
        cleanup_test_data(&broker).await;
        
        // Initial metrics should be zero
        let metrics = broker.get_queue_metrics("test_queue").await.expect("Failed to get metrics");
        assert_eq!(metrics.pending_tasks, 0);
        assert_eq!(metrics.processed_tasks, 0);
        assert_eq!(metrics.failed_tasks, 0);
        
        // Add a task
        let task = TestTask { data: "test".to_string() };
        broker.enqueue_task(task, "test_queue").await.expect("Failed to enqueue task");
        
        let metrics = broker.get_queue_metrics("test_queue").await.expect("Failed to get metrics");
        assert_eq!(metrics.pending_tasks, 1);
        
        cleanup_test_data(&broker).await;
    }

    #[tokio::test]
    async fn test_mark_task_completed() {
        let broker = create_test_broker().await;
        cleanup_test_data(&broker).await;
        
        let task = TestTask { data: "test".to_string() };
        let task_id = broker.enqueue_task(task, "test_queue").await.expect("Failed to enqueue task");
        
        // Mark as completed
        broker.mark_task_completed(task_id, "test_queue").await.expect("Failed to mark completed");
        
        let metrics = broker.get_queue_metrics("test_queue").await.expect("Failed to get metrics");
        assert_eq!(metrics.processed_tasks, 1);
        
        cleanup_test_data(&broker).await;
    }

    #[tokio::test]
    async fn test_mark_task_failed() {
        let broker = create_test_broker().await;
        cleanup_test_data(&broker).await;
        
        let task = TestTask { data: "test".to_string() };
        let task_id = broker.enqueue_task(task, "test_queue").await.expect("Failed to enqueue task");
        
        // Mark as failed
        broker.mark_task_failed(task_id, "test_queue").await.expect("Failed to mark failed");
        
        let metrics = broker.get_queue_metrics("test_queue").await.expect("Failed to get metrics");
        assert_eq!(metrics.failed_tasks, 1);
        
        cleanup_test_data(&broker).await;
    }

    #[tokio::test]
    async fn test_mark_task_failed_with_reason() {
        let broker = create_test_broker().await;
        cleanup_test_data(&broker).await;
        
        let task = TestTask { data: "test".to_string() };
        let task_id = broker.enqueue_task(task, "test_queue").await.expect("Failed to enqueue task");
        
        let reason = "Custom failure reason".to_string();
        broker.mark_task_failed_with_reason(task_id, "test_queue", Some(reason.clone()))
            .await.expect("Failed to mark failed with reason");
        
        // Verify failure info was stored
        let failure_info = broker.get_task_failure_info(task_id).await.expect("Failed to get failure info");
        assert!(failure_info.is_some());
        
        let info = failure_info.unwrap();
        assert_eq!(info.task_id, task_id);
        assert_eq!(info.queue, "test_queue");
        assert_eq!(info.reason, reason);
        assert_eq!(info.status, "failed");
        
        cleanup_test_data(&broker).await;
    }

    #[tokio::test]
    async fn test_worker_registration() {
        let broker = create_test_broker().await;
        cleanup_test_data(&broker).await;
        
        let worker_id = "test_worker_001";
        
        // Register worker
        broker.register_worker(worker_id).await.expect("Failed to register worker");
        
        let active_workers = broker.get_active_workers().await.expect("Failed to get active workers");
        assert_eq!(active_workers, 1);
        
        // Update heartbeat
        broker.update_worker_heartbeat(worker_id).await.expect("Failed to update heartbeat");
        
        // Unregister worker
        broker.unregister_worker(worker_id).await.expect("Failed to unregister worker");
        
        let active_workers = broker.get_active_workers().await.expect("Failed to get active workers");
        assert_eq!(active_workers, 0);
        
        cleanup_test_data(&broker).await;
    }

    #[tokio::test]
    async fn test_multiple_workers() {
        let broker = create_test_broker().await;
        cleanup_test_data(&broker).await;
        
        // Register multiple workers
        for i in 0..5 {
            let worker_id = format!("worker_{}", i);
            broker.register_worker(&worker_id).await.expect("Failed to register worker");
        }
        
        let active_workers = broker.get_active_workers().await.expect("Failed to get active workers");
        assert_eq!(active_workers, 5);
        
        cleanup_test_data(&broker).await;
    }

    #[tokio::test]
    async fn test_failed_tasks_tracking() {
        let broker = create_test_broker().await;
        cleanup_test_data(&broker).await;
        
        // Enqueue and fail multiple tasks
        for i in 0..3 {
            let task = TestTask { data: format!("task_{}", i) };
            let task_id = broker.enqueue_task(task, "test_queue").await.expect("Failed to enqueue task");
            broker.mark_task_failed(task_id, "test_queue").await.expect("Failed to mark failed");
        }
        
        let failed_tasks = broker.get_failed_tasks("test_queue").await.expect("Failed to get failed tasks");
        assert_eq!(failed_tasks.len(), 3);
        
        cleanup_test_data(&broker).await;
    }

    #[tokio::test]
    async fn test_queue_metrics_comprehensive() {
        let broker = create_test_broker().await;
        cleanup_test_data(&broker).await;
        
        // Add pending tasks
        for i in 0..3 {
            let task = TestTask { data: format!("pending_{}", i) };
            broker.enqueue_task(task, "test_queue").await.expect("Failed to enqueue task");
        }
        
        // Add processed tasks
        for i in 0..2 {
            let task = TestTask { data: format!("processed_{}", i) };
            let task_id = broker.enqueue_task(task, "temp_queue").await.expect("Failed to enqueue task");
            broker.mark_task_completed(task_id, "test_queue").await.expect("Failed to mark completed");
        }
        
        // Add failed tasks
        for i in 0..1 {
            let task = TestTask { data: format!("failed_{}", i) };
            let task_id = broker.enqueue_task(task, "temp_queue").await.expect("Failed to enqueue task");
            broker.mark_task_failed(task_id, "test_queue").await.expect("Failed to mark failed");
        }
        
        let metrics = broker.get_queue_metrics("test_queue").await.expect("Failed to get metrics");
        assert_eq!(metrics.pending_tasks, 3);
        assert_eq!(metrics.processed_tasks, 2);
        assert_eq!(metrics.failed_tasks, 1);
        assert_eq!(metrics.queue_name, "test_queue");
        
        cleanup_test_data(&broker).await;
    }

    #[tokio::test]
    async fn test_task_failure_info_serialization() {
        let task_id = TaskId::new_v4();
        let failure_info = TaskFailureInfo {
            task_id,
            queue: "test_queue".to_string(),
            failed_at: chrono::Utc::now().to_rfc3339(),
            reason: "Test failure".to_string(),
            status: "failed".to_string(),
        };
        
        // Test serialization
        let serialized = rmp_serde::to_vec(&failure_info).expect("Failed to serialize");
        let deserialized: TaskFailureInfo = rmp_serde::from_slice(&serialized).expect("Failed to deserialize");
        
        assert_eq!(deserialized.task_id, failure_info.task_id);
        assert_eq!(deserialized.queue, failure_info.queue);
        assert_eq!(deserialized.reason, failure_info.reason);
        assert_eq!(deserialized.status, failure_info.status);
    }

    #[tokio::test]
    async fn test_queue_metrics_serialization() {
        let metrics = QueueMetrics {
            queue_name: "test_queue".to_string(),
            pending_tasks: 10,
            processed_tasks: 100,
            failed_tasks: 5,
        };
        
        // Test serialization
        let serialized = rmp_serde::to_vec(&metrics).expect("Failed to serialize");
        let deserialized: QueueMetrics = rmp_serde::from_slice(&serialized).expect("Failed to deserialize");
        
        assert_eq!(deserialized.queue_name, metrics.queue_name);
        assert_eq!(deserialized.pending_tasks, metrics.pending_tasks);
        assert_eq!(deserialized.processed_tasks, metrics.processed_tasks);
        assert_eq!(deserialized.failed_tasks, metrics.failed_tasks);
    }

    #[tokio::test]
    async fn test_enqueue_task_wrapper() {
        let broker = create_test_broker().await;
        cleanup_test_data(&broker).await;
        
        let task_id = TaskId::new_v4();
        let metadata = TaskMetadata {
            id: task_id,
            name: "custom_task".to_string(),
            created_at: chrono::Utc::now(),
            attempts: 0,
            max_retries: 5,
            timeout_seconds: 600,
        };
        
        let task_wrapper = TaskWrapper {
            metadata,
            payload: b"custom payload".to_vec(),
        };
        
        let returned_id = broker.enqueue_task_wrapper(task_wrapper, "test_queue")
            .await.expect("Failed to enqueue task wrapper");
        
        assert_eq!(returned_id, task_id);
        
        let queue_size = broker.get_queue_size("test_queue").await.expect("Failed to get queue size");
        assert_eq!(queue_size, 1);
        
        cleanup_test_data(&broker).await;
    }
}
