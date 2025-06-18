//! Performance test example to demonstrate the improved Redis connection pooling
//!
//! Run with: `cargo run --example performance_test --features "full"`

use rust_task_queue::prelude::*;
use rust_task_queue::queue::queue_names;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;

#[cfg(feature = "tracing")]
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PerformanceTask {
    id: u32,
    complexity: String,
    payload_size: usize,
}

#[async_trait]
impl Task for PerformanceTask {
    async fn execute(&self) -> TaskResult {
        #[cfg(feature = "tracing")]
        let tracker = PerformanceTracker::new("task_execution")
            .with_context("task_id", &self.id.to_string())
            .with_context("complexity", &self.complexity);

        // Simulate different complexity levels
        let work_duration = match self.complexity.as_str() {
            "light" => Duration::from_millis(10),
            "medium" => Duration::from_millis(100),
            "heavy" => Duration::from_millis(500),
            _ => Duration::from_millis(50),
        };

        #[cfg(feature = "tracing")]
        tracing::info!(
            task_id = self.id,
            complexity = %self.complexity,
            work_duration_ms = work_duration.as_millis(),
            "Starting task work simulation"
        );

        sleep(work_duration).await;

        let result = format!(
            "Processed task {} with {} complexity ({}ms)",
            self.id,
            self.complexity,
            work_duration.as_millis()
        );

        #[cfg(feature = "tracing")]
        tracker.trace_completion();

        Ok(result.into_bytes())
    }

    fn name(&self) -> &str {
        "performance_task"
    }

    fn max_retries(&self) -> u32 {
        2
    }

    fn priority(&self) -> TaskPriority {
        match self.complexity.as_str() {
            "heavy" => TaskPriority::High,
            "medium" => TaskPriority::Normal,
            "light" => TaskPriority::Low,
            _ => TaskPriority::Normal,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize enhanced tracing for comprehensive observability
    #[cfg(feature = "tracing")]
    {
        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("rust_task_queue=info,performance_test=info"));

        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                fmt::layer()
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_file(true)
                    .with_line_number(true)
                    .pretty()
            )
            .init();

        tracing::info!("Enhanced Performance Test with Comprehensive Tracing");
        tracing::info!("Features enabled: structured logging, lifecycle tracking, performance metrics");
    }

    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

    #[cfg(feature = "tracing")]
    tracing::info!(redis_url = %redis_url, "Connecting to Redis");

    let task_queue = TaskQueue::new(&redis_url).await?;

    // Start workers with registry
    let registry = TaskRegistry::new();
    registry.register_with_name::<PerformanceTask>("performance_task")
        .expect("Failed to register performance task");

    task_queue.start_workers_with_registry(4, std::sync::Arc::new(registry)).await?;

    #[cfg(feature = "tracing")]
    tracing::info!("Workers started, beginning performance test with tracing");

    // Create and enqueue different types of tasks
    let task_types = [
        ("light", 20),
        ("medium", 10),
        ("heavy", 5),
    ];

    let mut task_id = 1;

    for (complexity, count) in task_types {
        #[cfg(feature = "tracing")]
        tracing::info!(
            complexity = complexity,
            count = count,
            "Enqueuing task batch"
        );

        for _ in 0..count {
            let task = PerformanceTask {
                id: task_id,
                complexity: complexity.to_string(),
                payload_size: match complexity {
                    "light" => 128,
                    "medium" => 1024,
                    "heavy" => 4096,
                    _ => 512,
                },
            };

            // Use lifecycle event tracing
            #[cfg(feature = "tracing")]
            {
                let lifecycle_event = TaskLifecycleEvent::Enqueued {
                    queue: queue_names::DEFAULT.to_string(),
                    priority: task.priority(),
                    payload_size_bytes: task.payload_size,
                    max_retries: task.max_retries(),
                    timeout_seconds: task.timeout_seconds(),
                };
                
                let task_id = task_queue.enqueue(task.clone(), queue_names::DEFAULT).await?;
                trace_task_lifecycle_event(task_id, task.name(), lifecycle_event);
            }

            #[cfg(not(feature = "tracing"))]
            {
                task_queue.enqueue(task, queue_names::DEFAULT).await?;
            }

            task_id += 1;
        }
    }

    #[cfg(feature = "tracing")]
    tracing::info!("All tasks enqueued, monitoring performance...");

    // Monitor queue metrics with enhanced logging
    for i in 1..=20 {
        sleep(Duration::from_secs(1)).await;

        let metrics = task_queue.broker.get_queue_metrics(queue_names::DEFAULT).await?;
        
        #[cfg(feature = "tracing")]
        {
            let completion_rate = if metrics.processed_tasks + metrics.failed_tasks > 0 {
                metrics.processed_tasks as f64 / (metrics.processed_tasks + metrics.failed_tasks) as f64
            } else {
                0.0
            };

            tracing::info!(
                iteration = i,
                pending_tasks = metrics.pending_tasks,
                processed_tasks = metrics.processed_tasks,
                failed_tasks = metrics.failed_tasks,
                completion_rate = format!("{:.2}%", completion_rate * 100.0),
                "Queue metrics snapshot"
            );

            trace_queue_operation(
                "metrics_check",
                queue_names::DEFAULT,
                Some(metrics.pending_tasks as usize),
                Duration::from_millis(1), // Simulated duration
                true,
                None,
            );
        }

        #[cfg(not(feature = "tracing"))]
        println!(
            "Metrics: {} pending, {} processed, {} failed",
            metrics.pending_tasks, metrics.processed_tasks, metrics.failed_tasks
        );

        if metrics.pending_tasks == 0 {
            #[cfg(feature = "tracing")]
            tracing::info!("All tasks completed!");
            break;
        }
    }

    #[cfg(feature = "tracing")]
    tracing::info!("Performance test completed, shutting down gracefully");

    task_queue.shutdown().await?;

    #[cfg(feature = "tracing")]
    tracing::info!("Shutdown complete - check logs for comprehensive performance metrics");

    Ok(())
}
