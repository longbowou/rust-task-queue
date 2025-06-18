//! Comprehensive tracing utilities for the Rust Task Queue system
//!
//! This module provides structured tracing utilities for observability, debugging,
//! and production monitoring of task queue operations.

use crate::{TaskId, TaskPriority};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Task lifecycle events for comprehensive observability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskLifecycleEvent {
    /// Task has been enqueued
    Enqueued {
        queue: String,
        priority: TaskPriority,
        payload_size_bytes: usize,
        max_retries: u32,
        timeout_seconds: u64,
    },
    /// Task has been dequeued by a worker
    Dequeued {
        worker_id: String,
        queue: String,
        wait_time_ms: u64,
        payload_size_bytes: usize,
    },
    /// Task execution has started
    ExecutionStarted {
        worker_id: String,
        attempt: u32,
        max_retries: u32,
    },
    /// Task execution completed successfully
    ExecutionCompleted {
        worker_id: String,
        duration_ms: u64,
        result_size_bytes: usize,
        attempt: u32,
    },
    /// Task execution failed
    ExecutionFailed {
        worker_id: String,
        duration_ms: u64,
        error: String,
        error_source: Option<String>,
        attempt: u32,
    },
    /// Task is being retried
    Retrying {
        attempt: u32,
        max_retries: u32,
        delay_ms: u64,
        reason: String,
    },
    /// Task has permanently failed
    PermanentlyFailed {
        total_attempts: u32,
        final_error: String,
        total_duration_ms: u64,
    },
    /// Task has been scheduled for delayed execution
    Scheduled {
        execute_at: chrono::DateTime<chrono::Utc>,
        delay_ms: i64,
        queue: String,
    },
    /// Scheduled task moved to regular queue
    MovedToRegularQueue {
        from_scheduled: bool,
        delay_from_scheduled_ms: i64,
        queue: String,
    },
}

/// Performance metrics for task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPerformanceMetrics {
    pub task_name: String,
    pub execution_count: u64,
    pub total_duration_ms: u64,
    pub average_duration_ms: f64,
    pub min_duration_ms: u64,
    pub max_duration_ms: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub success_rate: f64,
    pub last_execution: chrono::DateTime<chrono::Utc>,
}

/// Queue performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuePerformanceMetrics {
    pub queue_name: String,
    pub current_depth: i64,
    pub peak_depth: i64,
    pub total_processed: u64,
    pub total_failed: u64,
    pub average_processing_time_ms: f64,
    pub throughput_per_minute: f64,
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

/// Worker performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerPerformanceMetrics {
    pub worker_id: String,
    pub tasks_processed: u64,
    pub tasks_failed: u64,
    pub total_processing_time_ms: u64,
    pub average_processing_time_ms: f64,
    pub current_active_tasks: usize,
    pub max_concurrent_tasks: usize,
    pub uptime_seconds: u64,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
}

/// Trace a task lifecycle event with structured logging
pub fn trace_task_lifecycle_event(task_id: TaskId, task_name: &str, event: TaskLifecycleEvent) {
    match event {
        TaskLifecycleEvent::Enqueued {
            queue,
            priority,
            payload_size_bytes,
            max_retries,
            timeout_seconds,
        } => {
            tracing::info!(
                task_id = %task_id,
                task_name = task_name,
                queue = queue,
                priority = ?priority,
                payload_size_bytes = payload_size_bytes,
                max_retries = max_retries,
                timeout_seconds = timeout_seconds,
                event = "enqueued",
                "Task enqueued"
            );
        }
        TaskLifecycleEvent::Dequeued {
            worker_id,
            queue,
            wait_time_ms,
            payload_size_bytes,
        } => {
            tracing::info!(
                task_id = %task_id,
                task_name = task_name,
                worker_id = worker_id,
                queue = queue,
                wait_time_ms = wait_time_ms,
                payload_size_bytes = payload_size_bytes,
                event = "dequeued",
                "Task dequeued"
            );
        }
        TaskLifecycleEvent::ExecutionStarted {
            worker_id,
            attempt,
            max_retries,
        } => {
            tracing::info!(
                task_id = %task_id,
                task_name = task_name,
                worker_id = worker_id,
                attempt = attempt,
                max_retries = max_retries,
                event = "execution_started",
                "Task execution started"
            );
        }
        TaskLifecycleEvent::ExecutionCompleted {
            worker_id,
            duration_ms,
            result_size_bytes,
            attempt,
        } => {
            tracing::info!(
                task_id = %task_id,
                task_name = task_name,
                worker_id = worker_id,
                duration_ms = duration_ms,
                result_size_bytes = result_size_bytes,
                attempt = attempt,
                success = true,
                event = "execution_completed",
                "Task execution completed successfully"
            );
        }
        TaskLifecycleEvent::ExecutionFailed {
            worker_id,
            duration_ms,
            error,
            error_source,
            attempt,
        } => {
            tracing::error!(
                task_id = %task_id,
                task_name = task_name,
                worker_id = worker_id,
                duration_ms = duration_ms,
                error = error,
                error_source = error_source,
                attempt = attempt,
                success = false,
                event = "execution_failed",
                "Task execution failed"
            );
        }
        TaskLifecycleEvent::Retrying {
            attempt,
            max_retries,
            delay_ms,
            reason,
        } => {
            tracing::warn!(
                task_id = %task_id,
                task_name = task_name,
                attempt = attempt,
                max_retries = max_retries,
                delay_ms = delay_ms,
                reason = reason,
                event = "retrying",
                "Task being retried"
            );
        }
        TaskLifecycleEvent::PermanentlyFailed {
            total_attempts,
            final_error,
            total_duration_ms,
        } => {
            tracing::error!(
                task_id = %task_id,
                task_name = task_name,
                total_attempts = total_attempts,
                final_error = final_error,
                total_duration_ms = total_duration_ms,
                event = "permanently_failed",
                "Task permanently failed"
            );
        }
        TaskLifecycleEvent::Scheduled {
            execute_at,
            delay_ms,
            queue,
        } => {
            tracing::info!(
                task_id = %task_id,
                task_name = task_name,
                execute_at = %execute_at,
                delay_ms = delay_ms,
                queue = queue,
                event = "scheduled",
                "Task scheduled for delayed execution"
            );
        }
        TaskLifecycleEvent::MovedToRegularQueue {
            from_scheduled,
            delay_from_scheduled_ms,
            queue,
        } => {
            tracing::info!(
                task_id = %task_id,
                task_name = task_name,
                from_scheduled = from_scheduled,
                delay_from_scheduled_ms = delay_from_scheduled_ms,
                queue = queue,
                event = "moved_to_regular_queue",
                "Scheduled task moved to regular queue"
            );
        }
    }
}

/// Trace task error with detailed context and error chain
pub fn trace_task_error(
    task_id: TaskId,
    task_name: &str,
    error: &dyn std::error::Error,
    context: &str,
    worker_id: Option<&str>,
    attempt: Option<u32>,
) {
    tracing::error!(
        task_id = %task_id,
        task_name = task_name,
        error = %error,
        error_source = error.source().map(|e| e.to_string()).as_deref(),
        context = context,
        worker_id = worker_id,
        attempt = attempt,
        "Task error occurred"
    );

    // Log error chain for debugging
    let mut source = error.source();
    let mut depth = 1;
    while let Some(err) = source {
        tracing::debug!(
            task_id = %task_id,
            error_depth = depth,
            error = %err,
            "Error chain"
        );
        source = err.source();
        depth += 1;
    }
}

/// Simple performance tracker for measuring operation durations
pub struct PerformanceTracker {
    start_time: Instant,
    operation_name: String,
    context: HashMap<String, String>,
}

impl PerformanceTracker {
    pub fn new(operation_name: &str) -> Self {
        Self {
            start_time: Instant::now(),
            operation_name: operation_name.to_string(),
            context: HashMap::new(),
        }
    }

    pub fn with_context(mut self, key: &str, value: &str) -> Self {
        self.context.insert(key.to_string(), value.to_string());
        self
    }

    pub fn add_context(&mut self, key: &str, value: &str) {
        self.context.insert(key.to_string(), value.to_string());
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn trace_completion(self) {
        let duration = self.elapsed();
        tracing::debug!(
            operation = self.operation_name,
            duration_ms = duration.as_millis(),
            context = ?self.context,
            "Operation completed"
        );
    }

    pub fn trace_completion_with_result<T, E>(self, result: &Result<T, E>)
    where
        E: std::fmt::Display,
    {
        let duration = self.elapsed();
        match result {
            Ok(_) => {
                tracing::info!(
                    operation = self.operation_name,
                    duration_ms = duration.as_millis(),
                    success = true,
                    context = ?self.context,
                    "Operation completed successfully"
                );
            }
            Err(e) => {
                tracing::error!(
                    operation = self.operation_name,
                    duration_ms = duration.as_millis(),
                    success = false,
                    error = %e,
                    context = ?self.context,
                    "Operation failed"
                );
            }
        }
    }
}

/// Trace queue operation with metrics
pub fn trace_queue_operation(
    operation: &str,
    queue: &str,
    item_count: Option<usize>,
    duration: Duration,
    success: bool,
    error: Option<&str>,
) {
    if success {
        tracing::info!(
            operation = operation,
            queue = queue,
            item_count = item_count,
            duration_ms = duration.as_millis(),
            success = true,
            "Queue operation completed"
        );
    } else {
        tracing::error!(
            operation = operation,
            queue = queue,
            item_count = item_count,
            duration_ms = duration.as_millis(),
            success = false,
            error = error,
            "Queue operation failed"
        );
    }
}

/// Trace worker operation with context
pub fn trace_worker_operation(
    worker_id: &str,
    operation: &str,
    active_tasks: Option<usize>,
    duration: Option<Duration>,
    success: bool,
    error: Option<&str>,
) {
    if success {
        tracing::info!(
            worker_id = worker_id,
            operation = operation,
            active_tasks = active_tasks,
            duration_ms = duration.map(|d| d.as_millis()),
            success = true,
            "Worker operation completed"
        );
    } else {
        tracing::error!(
            worker_id = worker_id,
            operation = operation,
            active_tasks = active_tasks,
            duration_ms = duration.map(|d| d.as_millis()),
            success = false,
            error = error,
            "Worker operation failed"
        );
    }
}

/// Helper macro for creating traced spans around async operations
#[macro_export]
macro_rules! traced_operation {
    ($span_name:expr, $($field:ident = $value:expr),* $(,)?) => {
        tracing::info_span!($span_name, $($field = $value),*)
    };
}

/// Helper macro for timing operations with automatic tracing
#[macro_export]
macro_rules! timed_operation {
    ($operation_name:expr, $code:expr) => {{
        let tracker = $crate::tracing_utils::PerformanceTracker::new($operation_name);
        let result = $code;
        tracker.trace_completion_with_result(&result);
        result
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_tracker() {
        let tracker =
            PerformanceTracker::new("test_operation").with_context("test_key", "test_value");

        assert_eq!(tracker.operation_name, "test_operation");
        assert_eq!(
            tracker.context.get("test_key"),
            Some(&"test_value".to_string())
        );
        assert!(tracker.elapsed().as_nanos() > 0);
    }

    #[test]
    fn test_task_lifecycle_event_serialization() {
        let event = TaskLifecycleEvent::Enqueued {
            queue: "test_queue".to_string(),
            priority: TaskPriority::High,
            payload_size_bytes: 1024,
            max_retries: 3,
            timeout_seconds: 300,
        };

        let serialized = serde_json::to_string(&event).expect("Failed to serialize");
        let deserialized: TaskLifecycleEvent =
            serde_json::from_str(&serialized).expect("Failed to deserialize");

        match deserialized {
            TaskLifecycleEvent::Enqueued {
                queue,
                priority,
                payload_size_bytes,
                max_retries,
                timeout_seconds,
            } => {
                assert_eq!(queue, "test_queue");
                assert_eq!(priority, TaskPriority::High);
                assert_eq!(payload_size_bytes, 1024);
                assert_eq!(max_retries, 3);
                assert_eq!(timeout_seconds, 300);
            }
            _ => panic!("Unexpected event type"),
        }
    }
}
