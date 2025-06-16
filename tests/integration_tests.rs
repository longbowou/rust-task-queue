use rust_task_queue::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use uuid::Uuid;
use std::sync::atomic::{AtomicU8, Ordering};

// Global counter for unique database numbers
static DB_COUNTER: AtomicU8 = AtomicU8::new(0);

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
        sleep(Duration::from_millis(50)).await;
        
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

fn setup_isolated_redis_url() -> String {
    let db_num = DB_COUNTER.fetch_add(1, Ordering::SeqCst);
    let base_url = std::env::var("REDIS_TEST_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    format!("{}/{}", base_url, db_num)
}

async fn cleanup_test_database(redis_url: &str) {
    if let Ok(client) = redis::Client::open(redis_url) {
        if let Ok(mut conn) = client.get_async_connection().await {
            let _: Result<String, _> = redis::cmd("FLUSHDB").query_async(&mut conn).await;
        }
    }
    sleep(Duration::from_millis(50)).await;
}

// Test helper to wait for condition with timeout
async fn wait_for_condition<F, Fut>(mut condition: F, timeout_secs: u64) -> bool 
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let timeout = Duration::from_secs(timeout_secs);
    let start = std::time::Instant::now();
    
    while start.elapsed() < timeout {
        if condition().await {
            return true;
        }
        sleep(Duration::from_millis(100)).await;
    }
    false
}

async fn ensure_clean_shutdown(task_queue: &TaskQueue) {
    // Stop all components
    task_queue.stop_workers().await;
    task_queue.stop_scheduler().await;
    
    // Wait for graceful shutdown
    sleep(Duration::from_millis(1000)).await;
}

#[tokio::test]
async fn test_basic_task_execution() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    let redis_url = setup_isolated_redis_url();
    println!("Running test_basic_task_execution with ID: {} on {}", test_id, redis_url);
    
    cleanup_test_database(&redis_url).await;
    
    let task_queue = TaskQueue::new(&redis_url).await.expect("Failed to create task queue");
    
    // Create task registry and register our test task
    let registry = TaskRegistry::new();
    registry.register_with_name::<TestTask>("test_task").expect("Failed to register task");
    
    // Start a worker
    task_queue.start_workers_with_registry(1, Arc::new(registry)).await.expect("Failed to start workers");
    
    // Wait for worker to be ready
    sleep(Duration::from_millis(500)).await;
    
    // Enqueue a task
    let task = TestTask {
        data: format!("Hello from test {}", test_id),
        should_fail: false,
    };
    
    let _task_id = task_queue.enqueue(task, "default").await.expect("Failed to enqueue task");
    
    // Wait for task to be processed
    let success = wait_for_condition(|| {
        let broker = task_queue.broker.clone();
        async move {
            if let Ok(metrics) = broker.get_queue_metrics("default").await {
                metrics.processed_tasks > 0
            } else {
                false
            }
        }
    }, 10).await;
    
    assert!(success, "Task was not processed within timeout");
    
    let metrics = task_queue.broker.get_queue_metrics("default").await.expect("Failed to get metrics");
    println!("Basic test metrics: processed={}, failed={}, pending={}", 
             metrics.processed_tasks, metrics.failed_tasks, metrics.pending_tasks);
    
    // Cleanup
    ensure_clean_shutdown(&task_queue).await;
    cleanup_test_database(&redis_url).await;
    
    println!("Completed test_basic_task_execution");
}

#[tokio::test]
async fn test_task_retry_mechanism() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    let redis_url = setup_isolated_redis_url();
    println!("Running test_task_retry_mechanism with ID: {} on {}", test_id, redis_url);
    
    cleanup_test_database(&redis_url).await;
    
    let task_queue = TaskQueue::new(&redis_url).await.expect("Failed to create task queue");
    
    let registry = TaskRegistry::new();
    registry.register_with_name::<TestTask>("test_task").expect("Failed to register task");
    
    task_queue.start_workers_with_registry(1, Arc::new(registry)).await.expect("Failed to start workers");
    
    // Wait for worker to be ready
    sleep(Duration::from_millis(500)).await;
    
    // Enqueue a failing task
    let task = TestTask {
        data: format!("Failing task from test {}", test_id),
        should_fail: true,
    };
    
    let _task_id = task_queue.enqueue(task, "default").await.expect("Failed to enqueue task");
    
    // Wait for retries to complete and task to be marked as failed
    let success = wait_for_condition(|| {
        let broker = task_queue.broker.clone();
        async move {
            if let Ok(metrics) = broker.get_queue_metrics("default").await {
                metrics.failed_tasks > 0
            } else {
                false
            }
        }
    }, 15).await;
    
    assert!(success, "Task was not marked as failed within timeout");
    
    let final_metrics = task_queue.broker.get_queue_metrics("default").await.expect("Failed to get metrics");
    println!("Retry test metrics: processed={}, failed={}, pending={}", 
             final_metrics.processed_tasks, final_metrics.failed_tasks, final_metrics.pending_tasks);
    
    // Verify that the task was indeed marked as failed
    assert!(final_metrics.failed_tasks > 0, "Expected failed tasks but got: {}", final_metrics.failed_tasks);
    
    // Cleanup
    ensure_clean_shutdown(&task_queue).await;
    cleanup_test_database(&redis_url).await;
    
    println!("Completed test_task_retry_mechanism");
}

#[tokio::test]
async fn test_scheduler() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    let redis_url = setup_isolated_redis_url();
    println!("Running test_scheduler with ID: {} on {}", test_id, redis_url);
    
    cleanup_test_database(&redis_url).await;
    
    let task_queue = TaskQueue::new(&redis_url).await.expect("Failed to create task queue");
    
    task_queue.start_scheduler().await.expect("Failed to start scheduler");
    
    // Wait for scheduler to be ready
    sleep(Duration::from_millis(500)).await;
    
    // Schedule a task for the future  
    let task = TestTask {
        data: format!("Scheduled task from test {}", test_id),
        should_fail: false,
    };
    
    let delay = chrono::Duration::seconds(1);
    let _task_id = task_queue.schedule(task, "default", delay).await.expect("Failed to schedule task");
    
    // Wait for task to be moved to the queue
    let success = wait_for_condition(|| {
        let broker = task_queue.broker.clone();
        async move {
            if let Ok(queue_size) = broker.get_queue_size("default").await {
                queue_size > 0
            } else {
                false
            }
        }
    }, 10).await;
    
    assert!(success, "Scheduled task was not moved to queue within timeout");
    
    let queue_size = task_queue.broker.get_queue_size("default").await.expect("Failed to get queue size");
    println!("Scheduler test: queue_size={}", queue_size);
    
    // Cleanup
    ensure_clean_shutdown(&task_queue).await;
    cleanup_test_database(&redis_url).await;
    
    println!("Completed test_scheduler");
}

#[tokio::test]
async fn test_autoscaler_metrics() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    let redis_url = setup_isolated_redis_url();
    println!("Running test_autoscaler_metrics with ID: {} on {}", test_id, redis_url);
    
    cleanup_test_database(&redis_url).await;
    
    let task_queue = TaskQueue::new(&redis_url).await.expect("Failed to create task queue");
    
    // Wait a bit to ensure clean state
    sleep(Duration::from_millis(500)).await;
    
    let metrics = task_queue.autoscaler.collect_metrics().await.expect("Failed to collect metrics");
    
    println!("Autoscaler metrics: active_workers={}, total_pending_tasks={}", 
             metrics.active_workers, metrics.total_pending_tasks);
    
    assert_eq!(metrics.active_workers, 0, "Expected 0 active workers, got {}", metrics.active_workers);
    assert!(metrics.total_pending_tasks >= 0, "Expected non-negative pending tasks, got {}", metrics.total_pending_tasks);
    
    cleanup_test_database(&redis_url).await;
    
    println!("Completed test_autoscaler_metrics");
}

#[tokio::test]
async fn test_queue_priorities() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    let redis_url = setup_isolated_redis_url();
    println!("Running test_queue_priorities with ID: {} on {}", test_id, redis_url);
    
    cleanup_test_database(&redis_url).await;
    
    let task_queue = TaskQueue::new(&redis_url).await.expect("Failed to create task queue");
    
    // Wait for clean state
    sleep(Duration::from_millis(500)).await;
    
    // Test different queue priorities
    let high_priority_task = TestTask {
        data: format!("High priority task from test {}", test_id),
        should_fail: false,
    };
    
    let low_priority_task = TestTask {
        data: format!("Low priority task from test {}", test_id),
        should_fail: false,
    };
    
    // Enqueue to different priority queues
    let _high_id = task_queue.enqueue(high_priority_task, "high_priority").await.expect("Failed to enqueue high priority task");
    let _low_id = task_queue.enqueue(low_priority_task, "low_priority").await.expect("Failed to enqueue low priority task");
    
    // Wait for enqueue operations to complete
    sleep(Duration::from_millis(300)).await;
    
    // Verify queue sizes
    let high_size = task_queue.broker.get_queue_size("high_priority").await.expect("Failed to get high priority queue size");
    let low_size = task_queue.broker.get_queue_size("low_priority").await.expect("Failed to get low priority queue size");
    
    println!("Queue priorities test: high_priority_size={}, low_priority_size={}", high_size, low_size);
    
    assert_eq!(high_size, 1, "Expected high priority queue size 1 but got: {}", high_size);
    assert_eq!(low_size, 1, "Expected low priority queue size 1 but got: {}", low_size);
    
    cleanup_test_database(&redis_url).await;
    
    println!("Completed test_queue_priorities");
}

#[tokio::test]
async fn test_integration_comprehensive() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    let redis_url = setup_isolated_redis_url();
    println!("Running test_integration_comprehensive with ID: {} on {}", test_id, redis_url);
    
    cleanup_test_database(&redis_url).await;
    
    let task_queue = TaskQueue::new(&redis_url).await.expect("Failed to create task queue");
    
    // Set up registry
    let registry = TaskRegistry::new();
    registry.register_with_name::<TestTask>("test_task").expect("Failed to register task");
    
    // Start workers and scheduler
    task_queue.start_workers_with_registry(2, Arc::new(registry)).await.expect("Failed to start workers");
    task_queue.start_scheduler().await.expect("Failed to start scheduler");
    
    // Wait for components to be ready
    sleep(Duration::from_millis(1000)).await;
    
    // Test 1: Enqueue multiple tasks
    for i in 0..3 {
        let task = TestTask {
            data: format!("Batch task {} from test {}", i, test_id),
            should_fail: false,
        };
        task_queue.enqueue(task, "default").await.expect("Failed to enqueue task");
    }
    
    // Test 2: Schedule a task
    let scheduled_task = TestTask {
        data: format!("Scheduled task from comprehensive test {}", test_id),
        should_fail: false,
    };
    let delay = chrono::Duration::seconds(1);
    task_queue.schedule(scheduled_task, "default", delay).await.expect("Failed to schedule task");
    
    // Test 3: Enqueue a failing task
    let failing_task = TestTask {
        data: format!("Failing task from comprehensive test {}", test_id),
        should_fail: true,
    };
    task_queue.enqueue(failing_task, "default").await.expect("Failed to enqueue failing task");
    
    // Wait for all tasks to be processed
    let success = wait_for_condition(|| {
        let broker = task_queue.broker.clone();
        async move {
            if let Ok(metrics) = broker.get_queue_metrics("default").await {
                // Expect: 3 successful tasks + 1 scheduled task + 1 failed task = 4 processed, 1 failed
                metrics.processed_tasks >= 4 && metrics.failed_tasks >= 1
            } else {
                false
            }
        }
    }, 25).await;
    
    let final_metrics = task_queue.broker.get_queue_metrics("default").await.expect("Failed to get metrics");
    println!("Comprehensive test metrics: processed={}, failed={}, pending={}", 
             final_metrics.processed_tasks, final_metrics.failed_tasks, final_metrics.pending_tasks);
    
    assert!(success, "Not all tasks were processed within timeout. Metrics: processed={}, failed={}", 
            final_metrics.processed_tasks, final_metrics.failed_tasks);
    
    // Test autoscaler metrics
    let autoscaler_metrics = task_queue.autoscaler.collect_metrics().await.expect("Failed to collect autoscaler metrics");
    println!("Autoscaler metrics: active_workers={}, total_pending_tasks={}", 
             autoscaler_metrics.active_workers, autoscaler_metrics.total_pending_tasks);
    
    // Cleanup
    ensure_clean_shutdown(&task_queue).await;
    cleanup_test_database(&redis_url).await;
    
    println!("Completed test_integration_comprehensive");
} 