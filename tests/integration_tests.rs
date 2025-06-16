use rust_task_queue::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use uuid;

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

fn generate_unique_queue_name(test_name: &str) -> String {
    format!("test_{}_{}", test_name, uuid::Uuid::new_v4().to_string()[..8].to_string())
}

async fn cleanup_redis_for_test(redis_url: &str, queue_name: &str) {
    if let Ok(client) = redis::Client::open(redis_url) {
        if let Ok(mut conn) = client.get_async_connection().await {
            // Clean up test-specific keys
            let _: Result<(), _> = redis::AsyncCommands::del(&mut conn, queue_name).await;
            let _: Result<(), _> = redis::AsyncCommands::del(&mut conn, format!("queue:{}:processed", queue_name)).await;
            let _: Result<(), _> = redis::AsyncCommands::del(&mut conn, format!("queue:{}:failed", queue_name)).await;
            let _: Result<(), _> = redis::AsyncCommands::del(&mut conn, format!("queue:{}:failed_tasks", queue_name)).await;
            
            // Clean up worker-related keys that might interfere between tests
            let _: Result<(), _> = redis::AsyncCommands::del(&mut conn, "active_workers").await;
            let _: Result<(), _> = redis::AsyncCommands::del(&mut conn, "scheduled_tasks").await;
            
            // Clean up any worker heartbeat keys by pattern
            let worker_keys: Vec<String> = redis::AsyncCommands::keys(&mut conn, "worker:*:heartbeat").await.unwrap_or_default();
            for key in worker_keys {
                let _: Result<(), _> = redis::AsyncCommands::del(&mut conn, &key).await;
            }
            
            // Clean up any task metadata keys by pattern  
            let task_keys: Vec<String> = redis::AsyncCommands::keys(&mut conn, "task:*:metadata").await.unwrap_or_default();
            for key in task_keys {
                let _: Result<(), _> = redis::AsyncCommands::del(&mut conn, &key).await;
            }
            
            // Clean up any task failure keys by pattern
            let failure_keys: Vec<String> = redis::AsyncCommands::keys(&mut conn, "task:*:failure").await.unwrap_or_default();
            for key in failure_keys {
                let _: Result<(), _> = redis::AsyncCommands::del(&mut conn, &key).await;
            }
        }
    }
}

// Add a global cleanup function for comprehensive Redis cleanup
async fn cleanup_all_redis_test_data(redis_url: &str) {
    if let Ok(client) = redis::Client::open(redis_url) {
        if let Ok(mut conn) = client.get_async_connection().await {
            // Flush the entire test database to ensure clean state
            let _: Result<String, _> = redis::cmd("FLUSHDB").query_async(&mut conn).await;
        }
    }
}

#[tokio::test]
async fn test_basic_task_execution() {
    let redis_url = setup_redis().await;
    let queue_name = "default"; // Use default queue that workers listen to
    
    let task_queue = TaskQueue::new(&redis_url).await.expect("Failed to create task queue");
    
    // Create task registry and register our test task
    let registry = TaskRegistry::new();
    registry.register_with_name::<TestTask>("test_task").expect("Failed to register task");
    
    // Start a worker
    task_queue.start_workers_with_registry(1, Arc::new(registry)).await.expect("Failed to start workers");
    
    // Small delay to ensure worker is fully started
    sleep(Duration::from_millis(500)).await;
    
    // Enqueue a task
    let task = TestTask {
        data: "Hello, World!".to_string(),
        should_fail: false,
    };
    
    let _task_id = task_queue.enqueue(task, queue_name).await.expect("Failed to enqueue task");
    
    // Wait for task processing
    sleep(Duration::from_secs(5)).await;
    
    // Check metrics
    let metrics = task_queue.broker.get_queue_metrics(queue_name).await.expect("Failed to get metrics");
    println!("Basic test metrics: processed={}, failed={}, pending={}", 
             metrics.processed_tasks, metrics.failed_tasks, metrics.pending_tasks);
    assert!(metrics.processed_tasks > 0, "Expected processed tasks > 0, got {}", metrics.processed_tasks);
    
    // Cleanup
    task_queue.stop_workers().await;
    sleep(Duration::from_millis(500)).await; // Allow workers to fully stop
}

#[tokio::test]
async fn test_task_retry_mechanism() {
    let redis_url = setup_redis().await;
    let queue_name = "default"; // Use default queue that workers listen to
    
    let task_queue = TaskQueue::new(&redis_url).await.expect("Failed to create task queue");
    
    let registry = TaskRegistry::new();
    registry.register_with_name::<TestTask>("test_task").expect("Failed to register task");
    
    task_queue.start_workers_with_registry(1, Arc::new(registry)).await.expect("Failed to start workers");
    
    // Small delay to ensure worker is fully started
    sleep(Duration::from_millis(500)).await;
    
    // Enqueue a failing task
    let task = TestTask {
        data: "This will fail".to_string(),
        should_fail: true,
    };
    
    let _task_id = task_queue.enqueue(task, queue_name).await.expect("Failed to enqueue task");
    
    // Wait for retries to complete (task will fail 3 times: initial + 2 retries)
    println!("Waiting for task retries to complete...");
    sleep(Duration::from_secs(8)).await;
    
    // Check metrics multiple times to debug
    for i in 0..3 {
        let metrics = task_queue.broker.get_queue_metrics(queue_name).await.expect("Failed to get metrics");
        println!("Attempt {}: processed={}, failed={}, pending={}", 
                 i + 1, metrics.processed_tasks, metrics.failed_tasks, metrics.pending_tasks);
        
        if metrics.failed_tasks > 0 {
            break;
        }
        
        sleep(Duration::from_secs(2)).await;
    }
    
    let final_metrics = task_queue.broker.get_queue_metrics(queue_name).await.expect("Failed to get metrics");
    println!("Final metrics: processed={}, failed={}, pending={}", 
             final_metrics.processed_tasks, final_metrics.failed_tasks, final_metrics.pending_tasks);
    
    // Also check failed tasks list
    let failed_tasks = task_queue.broker.get_failed_tasks(queue_name).await.expect("Failed to get failed tasks");
    println!("Failed tasks list: {:?}", failed_tasks);
    
    assert!(final_metrics.failed_tasks > 0, "Expected failed tasks but got: {}", final_metrics.failed_tasks);
    
    task_queue.stop_workers().await;
    sleep(Duration::from_millis(500)).await; // Allow workers to fully stop
}

#[tokio::test]
async fn test_scheduler() {
    let redis_url = setup_redis().await;
    let queue_name = "default"; // Use default queue that workers listen to
    
    let task_queue = TaskQueue::new(&redis_url).await.expect("Failed to create task queue");
    
    task_queue.start_scheduler().await.expect("Failed to start scheduler");
    
    // Schedule a task for the future  
    let task = TestTask {
        data: "Scheduled task".to_string(),
        should_fail: false,
    };
    
    let delay = chrono::Duration::seconds(1); // Shorter delay for faster test
    let _task_id = task_queue.schedule(task, queue_name, delay).await.expect("Failed to schedule task");
    
    // Wait and verify the task gets moved to the queue
    sleep(Duration::from_secs(3)).await;
    
    let queue_size = task_queue.broker.get_queue_size(queue_name).await.expect("Failed to get queue size");
    assert!(queue_size > 0, "Expected queue size > 0 but got: {}", queue_size);
    
    task_queue.stop_scheduler().await;
    sleep(Duration::from_millis(500)).await; // Allow scheduler to fully stop
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
    let high_priority_queue = "high_priority"; // Use actual queue that workers listen to
    let low_priority_queue = "low_priority";   // Use actual queue that workers listen to
    
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
    let _high_id = task_queue.enqueue(high_priority_task, high_priority_queue).await.expect("Failed to enqueue high priority task");
    let _low_id = task_queue.enqueue(low_priority_task, low_priority_queue).await.expect("Failed to enqueue low priority task");
    
    // Add a small delay to ensure the enqueue operations complete
    sleep(Duration::from_millis(100)).await;
    
    // Verify queue sizes
    let high_size = task_queue.broker.get_queue_size(high_priority_queue).await.expect("Failed to get high priority queue size");
    let low_size = task_queue.broker.get_queue_size(low_priority_queue).await.expect("Failed to get low priority queue size");
    
    assert_eq!(high_size, 1, "Expected high priority queue size 1 but got: {}", high_size);
    assert_eq!(low_size, 1, "Expected low priority queue size 1 but got: {}", low_size);
} 