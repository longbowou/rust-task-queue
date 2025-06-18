//! Convenience re-exports for common types and traits
//!
//! This module contains the most commonly used items from the rust-task-queue crate.
//! Users can import everything they need with `use rust_task_queue::prelude::*;`

// Core types and traits
pub use crate::{
    AutoScaler, AutoScalerConfig, QueueManager, QueueMetrics, RedisBroker, ScalingAction, Task,
    TaskId, TaskMetadata, TaskPriority, TaskQueue, TaskQueueBuilder, TaskQueueError, TaskRegistry,
    TaskResult, TaskScheduler, TaskWrapper, Worker,
};

// Configuration types
pub use crate::{AutoRegisterConfig, ConfigBuilder, RedisConfig, SchedulerConfig, TaskQueueConfig};

// Actix Web configuration (when feature is enabled)
#[cfg(feature = "actix-integration")]
pub use crate::ActixConfig;

#[cfg(feature = "actix-integration")]
pub use crate::actix::{
    auto_configure_task_queue, configure_task_queue_routes, configure_task_queue_routes_auto,
    create_auto_registered_task_queue, create_task_queue_from_config,
};

// Async trait for Task implementations
pub use async_trait::async_trait;

// MessagePack serialization (replaces JSON)
pub use rmp_serde;

// Common traits for serialization
pub use serde::{Deserialize, Serialize};

// Automatic task registration (when feature is enabled)
#[cfg(feature = "auto-register")]
pub use crate::{inventory, proc_register_task as register_task, AutoRegisterTask};

// Manual task registration macros (always available)
pub use crate::{manual_register_task, register_task_with_name, register_tasks};

// Date/time handling for scheduling
pub use chrono::{DateTime, Duration, Utc};

// UUID handling for task IDs
pub use uuid::Uuid;

#[cfg(feature = "tracing")]
pub use crate::tracing_utils::{
    trace_queue_operation, trace_task_error, trace_task_lifecycle_event, trace_worker_operation,
    PerformanceTracker, QueuePerformanceMetrics, TaskLifecycleEvent, TaskPerformanceMetrics,
    WorkerPerformanceMetrics,
};

#[cfg(feature = "config-support")]
pub use crate::config::*;
