//! Performance test example to demonstrate the improved Redis connection pooling
//! 
//! Run with: `cargo run --example performance_test --features "full"`

use rust_task_queue::prelude::*;
use rust_task_queue::queue::queue_names;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Serialize, Deserialize, Default)]
struct SimpleTask {
    data: String,
}

#[async_trait::async_trait]
impl Task for SimpleTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        // Simulate some work
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        use serde::Serialize;
        #[derive(Serialize)]
        struct Response { processed: String }
        let response = Response { processed: self.data.clone() };
        Ok(rmp_serde::to_vec(&response)?)
    }
    
    fn name(&self) -> &str {
        "simple_task"
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting performance test with improved connection pooling...");
    
    // Create task queue with connection pooling
    let task_queue = TaskQueueBuilder::new("redis://localhost:6379")
        .initial_workers(0) // Start without workers for this test
        .build()
        .await?;

    // Test 1: Sequential task enqueueing (tests connection pooling efficiency)
    println!("📊 Test 1: Sequential task enqueueing");
    let start = Instant::now();
    for i in 0..100 {
        let task = SimpleTask {
            data: format!("test_task_{}", i),
        };
        task_queue.enqueue(task, queue_names::DEFAULT).await?;
    }
    let sequential_duration = start.elapsed();
    println!("   Sequential: {} tasks in {:?}", 100, sequential_duration);

    // Test 2: Concurrent task enqueueing (tests connection pool under load)
    println!("📊 Test 2: Concurrent task enqueueing");
    let start = Instant::now();
    let mut handles = vec![];
    
    for i in 0..100 {
        let queue = task_queue.clone();
        let handle = tokio::spawn(async move {
            let task = SimpleTask {
                data: format!("concurrent_task_{}", i),
            };
            queue.enqueue(task, queue_names::DEFAULT).await
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to be enqueued
    for handle in handles {
        handle.await??;
    }
    let concurrent_duration = start.elapsed();
    println!("   Concurrent: {} tasks in {:?}", 100, concurrent_duration);

    // Show queue metrics
    let queue_size = task_queue.broker.get_queue_size(queue_names::DEFAULT).await?;
    println!("📈 Queue metrics:");
    println!("   Total tasks enqueued: {}", queue_size);
    println!("   Performance improvement: {:.2}x faster with concurrency", 
             sequential_duration.as_millis() as f64 / concurrent_duration.as_millis() as f64);

    println!("✅ Performance test completed!");
    println!("🔧 Connection pooling provides better performance under concurrent load");
    
    Ok(())
} 