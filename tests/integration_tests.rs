use rust_task_queue::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct TestTask {
    data: String,
    should_fail: bool,
}

#[async_trait::async_trait]
impl Task for TestTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        if self.should_fail {
            return Err("Task intentionally failed".into());
        }
        
        // Simulate some work
        sleep(Duration::from_millis(100)).await;
        
        #[derive(Serialize)]
        struct Response {
            status: String,
            processed_data: String,
        }
        
        let response = Response {
            status: "completed".to_string(),
            processed_data: format!("Processed: {}", self.data),
        };
        
        Ok(rmp_serde::to_vec(&response)?)
    }

    fn name(&self) -> &str {
        "test_task"
    }

    fn max_retries(&self) -> u32 {
        2
    }

    fn timeout_seconds(&self) -> u64 {
        30
    }
}

async fn setup_redis() -> String {
    // Use Redis container or local Redis for testing
    std::env::var("REDIS_TEST_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/15".to_string())
}

#[tokio::test]
async fn test_basic_task_execution() {
    let redis_url = setup_redis().await;
    let task_queue = TaskQueue::new(&redis_url).await.expect("Failed to create task queue");
    
    // Create task registry and register our test task
    let mut registry = TaskRegistry::new();
    registry.register_with_name::<TestTask>("test_task").expect("Failed to register task");
    
    // Start a worker
    task_queue.start_workers_with_registry(1, Arc::new(registry)).await.expect("Failed to start workers");
    
    // Enqueue a task
    let task = TestTask {
        data: "Hello, World!".to_string(),
        should_fail: false,
    };
    
    let task_id = task_queue.enqueue(task, "default").await.expect("Failed to enqueue task");
    
    // Wait for task processing
    sleep(Duration::from_secs(2)).await;
    
    // Check metrics
    let metrics = task_queue.broker.get_queue_metrics("default").await.expect("Failed to get metrics");
    assert!(metrics.processed_tasks > 0);
    
    // Cleanup
    task_queue.stop_workers().await;
}

#[tokio::test]
async fn test_task_retry_mechanism() {
    let redis_url = setup_redis().await;
    let task_queue = TaskQueue::new(&redis_url).await.expect("Failed to create task queue");
    
    let mut registry = TaskRegistry::new();
    registry.register_with_name::<TestTask>("test_task").expect("Failed to register task");
    
    task_queue.start_workers_with_registry(1, Arc::new(registry)).await.expect("Failed to start workers");
    
    // Enqueue a failing task
    let task = TestTask {
        data: "This will fail".to_string(),
        should_fail: true,
    };
    
    let _task_id = task_queue.enqueue(task, "default").await.expect("Failed to enqueue task");
    
    // Wait for retries
    sleep(Duration::from_secs(5)).await;
    
    let metrics = task_queue.broker.get_queue_metrics("default").await.expect("Failed to get metrics");
    assert!(metrics.failed_tasks > 0);
    
    task_queue.stop_workers().await;
}

#[tokio::test]
async fn test_scheduler() {
    let redis_url = setup_redis().await;
    let task_queue = TaskQueue::new(&redis_url).await.expect("Failed to create task queue");
    
    task_queue.start_scheduler().await.expect("Failed to start scheduler");
    
    // Schedule a task for the future
    let task = TestTask {
        data: "Scheduled task".to_string(),
        should_fail: false,
    };
    
    let delay = chrono::Duration::seconds(2);
    let _task_id = task_queue.schedule(task, "default", delay).await.expect("Failed to schedule task");
    
    // Wait and verify the task gets moved to the queue
    sleep(Duration::from_secs(3)).await;
    
    let queue_size = task_queue.broker.get_queue_size("default").await.expect("Failed to get queue size");
    assert!(queue_size > 0);
    
    task_queue.stop_scheduler().await;
}

#[tokio::test]
async fn test_autoscaler_metrics() {
    let redis_url = setup_redis().await;
    let task_queue = TaskQueue::new(&redis_url).await.expect("Failed to create task queue");
    
    let metrics = task_queue.autoscaler.collect_metrics().await.expect("Failed to collect metrics");
    
    assert_eq!(metrics.active_workers, 0);  // No workers started yet
    assert!(metrics.total_pending_tasks >= 0);
}

#[tokio::test]
async fn test_queue_priorities() {
    let redis_url = setup_redis().await;
    let task_queue = TaskQueue::new(&redis_url).await.expect("Failed to create task queue");
    
    // Test different queue priorities
    let high_priority_task = TestTask {
        data: "High priority".to_string(),
        should_fail: false,
    };
    
    let low_priority_task = TestTask {
        data: "Low priority".to_string(),
        should_fail: false,
    };
    
    // Enqueue to different priority queues
    let _high_id = task_queue.enqueue(high_priority_task, "high_priority").await.expect("Failed to enqueue high priority task");
    let _low_id = task_queue.enqueue(low_priority_task, "low_priority").await.expect("Failed to enqueue low priority task");
    
    // Verify queue sizes
    let high_size = task_queue.broker.get_queue_size("high_priority").await.expect("Failed to get high priority queue size");
    let low_size = task_queue.broker.get_queue_size("low_priority").await.expect("Failed to get low priority queue size");
    
    assert_eq!(high_size, 1);
    assert_eq!(low_size, 1);
} 