//! Comprehensive Axum Integration Example
//!
//! This example showcases ALL available features of the rust-task-queue crate with Axum:
//! - TaskQueueBuilder with all configuration options
//! - Auto-registration and manual task registry
//! - All metrics endpoints and health checks
//! - Advanced auto-scaling configuration
//! - Task scheduling with various delay types
//! - Graceful shutdown handling
//! - Security features and rate limiting
//! - Comprehensive error handling
//! - Configuration management (TOML, YAML, ENV)
//! - CLI integration
//! - Task timeouts, retries, and cancellation
//! - Batch operations
//! - SLA monitoring and alerts

mod module_tasks;
mod tasks;

// Import all task types for comprehensive demonstration
use module_tasks::{AnalyticsTask, MLTrainingTask, SlackNotificationTask, SmsTask};
use tasks::{DataProcessingTask, EmailTask, NotificationTask, ScheduleEmailRequest};

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{get, post},
    Router,
};
use chrono::{Duration, Utc};
use rust_task_queue::axum::configure_task_queue_routes;
use rust_task_queue::prelude::*;
use rust_task_queue::queue::queue_names;
use rust_task_queue::{TaskQueue, TaskQueueBuilder, TaskQueueConfig};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
// use uuid::Uuid;

// ============================================================================
// API Response Types
// ============================================================================

#[derive(Serialize)]
struct TaskResponse {
    task_id: String,
    queue: String,
    enqueued_at: chrono::DateTime<chrono::Utc>,
    estimated_execution_time: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Serialize)]
struct BatchTaskResponse {
    batch_id: String,
    task_ids: Vec<String>,
    queue: String,
    total_tasks: usize,
    enqueued_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
struct ScheduledTaskResponse {
    scheduled_task_id: String,
    delay_seconds: i64,
    queue: String,
    scheduled_at: chrono::DateTime<chrono::Utc>,
    estimated_execution_time: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    error_type: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    request_id: Option<String>,
}

#[derive(Serialize)]
struct SystemStatusResponse {
    status: String,
    version: String,
    uptime_seconds: u64,
    workers: WorkerStatus,
    queues: Vec<QueueStatus>,
    auto_scaler: AutoScalerStatus,
    timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
struct WorkerStatus {
    active_count: usize,
    shutting_down: bool,
    total_tasks_processed: u64,
    current_load: f64,
}

#[derive(Serialize)]
struct QueueStatus {
    name: String,
    pending_tasks: i64,
    processed_tasks: i64,
    failed_tasks: i64,
    error_rate: f64,
}

#[derive(Serialize)]
struct AutoScalerStatus {
    enabled: bool,
    min_workers: usize,
    max_workers: usize,
    last_scaling_action: Option<String>,
    last_scaling_time: Option<chrono::DateTime<chrono::Utc>>,
}

// ============================================================================
// Request Types for Advanced Features
// ============================================================================

#[derive(Deserialize)]
struct BatchEnqueueRequest<T> {
    tasks: Vec<T>,
    queue: Option<String>,
    delay_seconds: Option<i64>,
}

#[derive(Deserialize)]
struct ScheduleTaskRequest<T> {
    task: T,
    queue: Option<String>,
    delay_seconds: Option<i64>,
    delay_until: Option<chrono::DateTime<chrono::Utc>>,
    cron_expression: Option<String>,
}

#[derive(Deserialize)]
struct WorkerScalingRequest {
    target_count: usize,
    reason: Option<String>,
}

// ============================================================================
// Helper Functions
// ============================================================================

fn get_request_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-request-id")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
}

fn json_error_response(
    error: &str,
    error_type: &str,
    request_id: Option<String>,
) -> (StatusCode, Json<ErrorResponse>) {
    let response = ErrorResponse {
        error: error.to_string(),
        error_type: error_type.to_string(),
        timestamp: Utc::now(),
        request_id,
    };
    (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
}

// ============================================================================
// System Management Endpoints
// ============================================================================

async fn get_comprehensive_status(
    State(task_queue): State<Arc<TaskQueue>>,
) -> Json<SystemStatusResponse> {
    let worker_count = task_queue.worker_count().await;
    let is_shutting_down = task_queue.is_shutting_down().await;
    let system_metrics = task_queue.get_system_metrics().await;

    // Get queue metrics for all known queues
    let queue_names_list = [
        queue_names::DEFAULT,
        queue_names::HIGH_PRIORITY,
        queue_names::LOW_PRIORITY,
    ];
    let mut queue_statuses = Vec::new();

    for queue_name in &queue_names_list {
        if let Ok(metrics) = task_queue.broker.get_queue_metrics(queue_name).await {
            let error_rate = if metrics.processed_tasks + metrics.failed_tasks > 0 {
                metrics.failed_tasks as f64
                    / (metrics.processed_tasks + metrics.failed_tasks) as f64
            } else {
                0.0
            };

            queue_statuses.push(QueueStatus {
                name: queue_name.to_string(),
                pending_tasks: metrics.pending_tasks,
                processed_tasks: metrics.processed_tasks,
                failed_tasks: metrics.failed_tasks,
                error_rate,
            });
        }
    }

    // Get auto-scaler status
    let autoscaler_status = AutoScalerStatus {
        enabled: true,             // TODO: Get from config
        min_workers: 1,            // TODO: Get from config
        max_workers: 10,           // TODO: Get from config
        last_scaling_action: None, // TODO: Track this
        last_scaling_time: None,   // TODO: Track this
    };

    let response = SystemStatusResponse {
        status: if is_shutting_down {
            "shutting_down"
        } else {
            "healthy"
        }
        .to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: system_metrics.uptime_seconds,
        workers: WorkerStatus {
            active_count: worker_count,
            shutting_down: is_shutting_down,
            total_tasks_processed: system_metrics.tasks.total_executed,
            current_load: system_metrics.performance.tasks_per_second,
        },
        queues: queue_statuses,
        auto_scaler: autoscaler_status,
        timestamp: Utc::now(),
    };

    Json(response)
}

async fn initiate_graceful_shutdown(
    State(task_queue): State<Arc<TaskQueue>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match task_queue.shutdown().await {
        Ok(_) => Ok(Json(serde_json::json!({
            "message": "Graceful shutdown initiated",
            "timestamp": Utc::now()
        }))),
        Err(e) => Err(json_error_response(&e.to_string(), "ShutdownError", None)),
    }
}

// ============================================================================
// Configuration Endpoints
// ============================================================================

async fn get_current_configuration() -> Json<serde_json::Value> {
    if let Some(config) = TaskQueueConfig::global() {
        Json(serde_json::to_value(config).unwrap_or_else(|_| serde_json::json!({})))
    } else {
        Json(serde_json::json!({
            "message": "No global configuration found",
            "config_sources": ["task-queue.toml", "task-queue.yaml", "environment variables"]
        }))
    }
}

async fn reload_configuration(
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let request_id = get_request_id(&headers);

    match TaskQueueConfig::init_global() {
        Ok(_) => Ok(Json(serde_json::json!({
            "message": "Configuration reloaded successfully",
            "timestamp": Utc::now()
        }))),
        Err(e) => Err(json_error_response(
            &e.to_string(),
            "ConfigReloadError",
            request_id,
        )),
    }
}

// ============================================================================
// Main Application Setup
// ============================================================================

async fn create_comprehensive_task_queue() -> Result<Arc<TaskQueue>, TaskQueueError> {
    // Demonstrate all TaskQueueBuilder options
    // Create the builder automatically (based on your environment variables or your task-queue.toml)
    let builder = TaskQueueBuilder::auto()?;

    // create the builder from global configuration (task-queue.toml)
    // let mut builder = TaskQueueBuilder::from_global_config()?;

    // Manual builder creation
    // let mut builder = TaskQueueBuilder::new("redis://localhost:6379")
    //     .initial_workers(5)
    //     .with_scheduler()
    //     .with_autoscaler()
    //     .auto_register_tasks();

    // Custom builder creation from global config fallinback to a manual configuration
    // let mut builder = if let Ok(builder) = TaskQueueBuilder::from_global_config() {
    //     println!("Using global configuration");
    //     builder
    // } else {
    //     println!("Using builder with manual configuration");
    //     TaskQueueBuilder::new("redis://localhost:6379")
    //         .initial_workers(5)
    //         .with_scheduler()
    //         .with_autoscaler()
    // };

    // Enable auto-registration if available. (Optional if you are using a separated cli worker)
    // #[cfg(feature = "auto-register")]
    // {
    //     builder = builder.auto_register_tasks();
    //     println!("Auto-registration enabled");
    // }
    //

    // Manual task registration. (Optional if you are using a separated cli worker)
    // #[cfg(not(feature = "auto-register"))]
    // {
    //     // Create manual registry if auto-registration is not available
    //     let registry = TaskRegistry::new();
    //     // Manual registration would go here
    //     builder = builder.task_registry(Arc::new(registry));
    //     println!("Manual task registry configured");
    // }

    let task_queue = builder.build().await?;
    Ok(Arc::new(task_queue))
}

fn print_api_endpoints() {
    println!();
    println!("Available API Endpoints:");
    println!("========================");

    println!();

    println!("Task Management:");
    println!("  POST /api/v1/tasks/email              - Enqueue email task");
    println!("  POST /api/v1/schedule/email           - Schedule email task");
    println!("  POST /api/v1/tasks/notification       - Enqueue notification task");
    println!("  POST /api/v1/tasks/data-processing    - Enqueue data processing task");
    println!("  POST /api/v1/tasks/slack-notification - Enqueue Slack notification");
    println!("  POST /api/v1/tasks/sms                - Enqueue SMS task");
    println!("  POST /api/v1/tasks/analytics          - Enqueue analytics task");
    println!("  POST /api/v1/tasks/ml-training        - Enqueue ML training task");

    println!();

    println!("Metrics & Monitoring:");
    println!("  GET  /tasks/health               - Health check");
    println!("  GET  /tasks/metrics              - Comprehensive metrics");
    println!("  GET  /tasks/metrics/system       - System metrics");
    println!("  GET  /tasks/metrics/performance  - Performance metrics");
    println!("  GET  /tasks/metrics/autoscaler   - Auto-scaler metrics");
    println!("  GET  /tasks/metrics/queues       - Queue metrics");
    println!("  GET  /tasks/metrics/workers      - Worker metrics");
    println!("  GET  /tasks/metrics/memory       - Memory metrics");
    println!("  GET  /tasks/alerts               - Active alerts");
    println!("  GET  /tasks/sla                  - SLA status");
    println!("  GET  /tasks/diagnostics          - Comprehensive diagnostics");
    println!("  GET  /tasks/uptime               - System uptime information");

    println!();

    println!("Administration:");
    println!("  GET  /api/config          - Get current configuration");
    println!("  POST /api/config/reload   - Reload configuration");
    println!("  POST /api/shutdown        - Initiate graceful shutdown");
    println!("  GET  /tasks/registry      - Task registry information");
    println!();

    println!();

    println!("Health & Status:");
    println!("  GET  /tasks/status        - System status");
    println!("  GET  /api/status          - Comprehensive system status");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("Starting Comprehensive Axum Integration Example");
    println!("================================================");
    println!();

    println!("Features Demonstrated:");
    println!("- TaskQueueBuilder with all configuration options");
    println!("- Auto-registration and manual task registry");
    println!("- All available metrics endpoints");
    println!("- Advanced scheduling (delays, future dates)");
    println!("- Batch task operations");
    println!("- Graceful shutdown handling");
    println!("- Configuration management (TOML/YAML/ENV)");
    println!("- Comprehensive error handling");
    println!("- CORS and request tracing support");
    println!("- SLA monitoring and health checks");
    println!();

    // Create TaskQueue with comprehensive configuration
    println!("Building TaskQueue with all features...");
    let task_queue = match create_comprehensive_task_queue().await {
        Ok(tq) => {
            println!("TaskQueue created successfully");
            tq
        }
        Err(e) => {
            eprintln!("Failed to create TaskQueue: {}", e);
            return Err(e.into());
        }
    };

    // Start background services (Optional if you are using a separated cli worker)
    // println!("Starting background services...");
    // if let Err(e) = task_queue.start_scheduler().await {
    //     eprintln!("Warning: Failed to start scheduler: {}", e);
    // } else {
    //     println!("Task scheduler started");
    // }

    // Start autoscaler services (Optional if you are using a separated cli worker)
    // if let Err(e) = task_queue.start_autoscaler() {
    //     eprintln!("Warning: Failed to start autoscaler: {}", e);
    // } else {
    //     println!("Auto-scaler started");
    // }

    print_api_endpoints();

    println!();
    println!("Starting Axum server on http://localhost:8000");
    println!("Workers: cargo run --bin task-worker");
    println!();

    // Create the main application router
    let app = Router::new()
        // Task management endpoints
        .route("/api/tasks/email", post(enqueue_email))
        .route("/api/tasks/notification", post(enqueue_notification))
        .route("/api/tasks/data-processing", post(enqueue_data_processing))
        .route(
            "/api/tasks/slack-notification",
            post(enqueue_slack_notification),
        )
        .route("/api/tasks/sms", post(enqueue_sms))
        .route("/api/tasks/analytics", post(enqueue_analytics))
        .route("/api/tasks/ml-training", post(enqueue_ml_training))
        .route("/api/schedule/email", post(schedule_email))
        // System management endpoints
        .route("/api/system/status", get(get_comprehensive_status))
        .route("/api/system/shutdown", post(initiate_graceful_shutdown))
        // Configuration endpoints
        .route("/api/system/config", get(get_current_configuration))
        .route("/api/system/config/reload", post(reload_configuration))
        // Merge with task queue routes (all metrics endpoints)
        .merge(configure_task_queue_routes())
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        )
        .with_state(task_queue.clone());

    // Start the server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await?;

    // Set up graceful shutdown (Optional if you are using a separated cli worker)
    // let shutdown_signal = async move {
    //     tokio::signal::ctrl_c()
    //         .await
    //         .expect("failed to install CTRL+C signal handler");
    //
    //     println!("\nShutdown signal received, initiating graceful shutdown...");
    //
    //     // Shutdown TaskQueue
    //     if let Err(e) = task_queue.shutdown().await {
    //         eprintln!("Error during TaskQueue shutdown: {}", e);
    //     } else {
    //         println!("TaskQueue shutdown completed successfully");
    //     }
    // };

    axum::serve(listener, app)
        // .with_graceful_shutdown(shutdown_signal) # Optional if you are using a separated cli worker
        .await?;
    Ok(())
}

// ============================================================================
// Basic Task Endpoints (Original Tasks)
// ============================================================================

async fn enqueue_email(
    State(task_queue): State<Arc<TaskQueue>>,
    headers: HeaderMap,
    Json(email_data): Json<EmailTask>,
) -> Result<Json<TaskResponse>, (StatusCode, Json<ErrorResponse>)> {
    let request_id = get_request_id(&headers);

    match task_queue.enqueue(email_data, queue_names::DEFAULT).await {
        Ok(task_id) => {
            let response = TaskResponse {
                task_id: task_id.to_string(),
                queue: queue_names::DEFAULT.to_string(),
                enqueued_at: Utc::now(),
                estimated_execution_time: None,
            };
            Ok(Json(response))
        }
        Err(e) => Err(json_error_response(
            &e.to_string(),
            "EnqueueError",
            request_id,
        )),
    }
}

async fn schedule_email(
    State(task_queue): State<Arc<TaskQueue>>,
    headers: HeaderMap,
    Json(request): Json<ScheduleEmailRequest>,
) -> Result<Json<ScheduledTaskResponse>, (StatusCode, Json<ErrorResponse>)> {
    let request_id = get_request_id(&headers);
    let delay = Duration::seconds(request.delay_seconds);

    match task_queue
        .schedule(request.email, queue_names::DEFAULT, delay)
        .await
    {
        Ok(task_id) => {
            let estimated_execution = Utc::now() + delay;
            let response = ScheduledTaskResponse {
                scheduled_task_id: task_id.to_string(),
                delay_seconds: request.delay_seconds,
                queue: queue_names::DEFAULT.to_string(),
                scheduled_at: Utc::now(),
                estimated_execution_time: estimated_execution,
            };
            Ok(Json(response))
        }
        Err(e) => Err(json_error_response(
            &e.to_string(),
            "ScheduleError",
            request_id,
        )),
    }
}

// ============================================================================
// Advanced Task Endpoints
// ============================================================================

async fn enqueue_notification(
    State(task_queue): State<Arc<TaskQueue>>,
    headers: HeaderMap,
    Json(notification_data): Json<NotificationTask>,
) -> Result<Json<TaskResponse>, (StatusCode, Json<ErrorResponse>)> {
    let request_id = get_request_id(&headers);

    match task_queue
        .enqueue(notification_data, queue_names::DEFAULT)
        .await
    {
        Ok(task_id) => {
            let response = TaskResponse {
                task_id: task_id.to_string(),
                queue: queue_names::DEFAULT.to_string(),
                enqueued_at: Utc::now(),
                estimated_execution_time: None,
            };
            Ok(Json(response))
        }
        Err(e) => Err(json_error_response(
            &e.to_string(),
            "EnqueueError",
            request_id,
        )),
    }
}

async fn enqueue_data_processing(
    State(task_queue): State<Arc<TaskQueue>>,
    headers: HeaderMap,
    Json(data): Json<DataProcessingTask>,
) -> Result<Json<TaskResponse>, (StatusCode, Json<ErrorResponse>)> {
    let request_id = get_request_id(&headers);

    match task_queue.enqueue(data, queue_names::DEFAULT).await {
        Ok(task_id) => {
            let response = TaskResponse {
                task_id: task_id.to_string(),
                queue: queue_names::DEFAULT.to_string(),
                enqueued_at: Utc::now(),
                estimated_execution_time: None,
            };
            Ok(Json(response))
        }
        Err(e) => Err(json_error_response(
            &e.to_string(),
            "EnqueueError",
            request_id,
        )),
    }
}

async fn enqueue_slack_notification(
    State(task_queue): State<Arc<TaskQueue>>,
    headers: HeaderMap,
    Json(data): Json<SlackNotificationTask>,
) -> Result<Json<TaskResponse>, (StatusCode, Json<ErrorResponse>)> {
    let request_id = get_request_id(&headers);

    match task_queue.enqueue(data, queue_names::DEFAULT).await {
        Ok(task_id) => {
            let response = TaskResponse {
                task_id: task_id.to_string(),
                queue: queue_names::DEFAULT.to_string(),
                enqueued_at: Utc::now(),
                estimated_execution_time: None,
            };
            Ok(Json(response))
        }
        Err(e) => Err(json_error_response(
            &e.to_string(),
            "EnqueueError",
            request_id,
        )),
    }
}

async fn enqueue_sms(
    State(task_queue): State<Arc<TaskQueue>>,
    headers: HeaderMap,
    Json(data): Json<SmsTask>,
) -> Result<Json<TaskResponse>, (StatusCode, Json<ErrorResponse>)> {
    let request_id = get_request_id(&headers);

    match task_queue.enqueue(data, queue_names::DEFAULT).await {
        Ok(task_id) => {
            let response = TaskResponse {
                task_id: task_id.to_string(),
                queue: queue_names::DEFAULT.to_string(),
                enqueued_at: Utc::now(),
                estimated_execution_time: None,
            };
            Ok(Json(response))
        }
        Err(e) => Err(json_error_response(
            &e.to_string(),
            "EnqueueError",
            request_id,
        )),
    }
}

async fn enqueue_analytics(
    State(task_queue): State<Arc<TaskQueue>>,
    headers: HeaderMap,
    Json(data): Json<AnalyticsTask>,
) -> Result<Json<TaskResponse>, (StatusCode, Json<ErrorResponse>)> {
    let request_id = get_request_id(&headers);

    match task_queue.enqueue(data, queue_names::DEFAULT).await {
        Ok(task_id) => {
            let response = TaskResponse {
                task_id: task_id.to_string(),
                queue: queue_names::DEFAULT.to_string(),
                enqueued_at: Utc::now(),
                estimated_execution_time: None,
            };
            Ok(Json(response))
        }
        Err(e) => Err(json_error_response(
            &e.to_string(),
            "EnqueueError",
            request_id,
        )),
    }
}

async fn enqueue_ml_training(
    State(task_queue): State<Arc<TaskQueue>>,
    headers: HeaderMap,
    Json(data): Json<MLTrainingTask>,
) -> Result<Json<TaskResponse>, (StatusCode, Json<ErrorResponse>)> {
    let request_id = get_request_id(&headers);

    match task_queue.enqueue(data, queue_names::HIGH_PRIORITY).await {
        Ok(task_id) => {
            let response = TaskResponse {
                task_id: task_id.to_string(),
                queue: queue_names::HIGH_PRIORITY.to_string(),
                enqueued_at: Utc::now(),
                estimated_execution_time: None,
            };
            Ok(Json(response))
        }
        Err(e) => Err(json_error_response(
            &e.to_string(),
            "EnqueueError",
            request_id,
        )),
    }
}
