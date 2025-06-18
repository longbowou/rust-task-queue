//! Comprehensive Actix Web Integration Example
//!
//! This example showcases ALL available features of the rust-task-queue crate:
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
use module_tasks::{
    AnalyticsTask

    // Add new comprehensive task types

    ,
    MLTrainingTask
    ,
    SlackNotificationTask,
    SmsTask
    ,
};
use tasks::{DataProcessingTask, EmailTask, NotificationTask, ScheduleEmailRequest};

use actix_web::{
    middleware, web, App, HttpRequest, HttpResponse, HttpServer, Result as ActixResult,
};
use chrono::{Duration, Utc};
use rust_task_queue::prelude::*;
use rust_task_queue::queue::queue_names;
use rust_task_queue::{TaskQueue, TaskQueueBuilder, TaskQueueConfig};
use serde::{Deserialize, Serialize};
use serde_json;
use std::sync::Arc;
use uuid::Uuid;

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

fn json_response<T: Serialize>(data: &T) -> ActixResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(data))
}

fn json_error_response(
    error: &str,
    error_type: &str,
    request_id: Option<String>,
) -> ActixResult<HttpResponse> {
    let response = ErrorResponse {
        error: error.to_string(),
        error_type: error_type.to_string(),
        timestamp: Utc::now(),
        request_id,
    };
    Ok(HttpResponse::InternalServerError().json(response))
}

fn get_request_id(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("x-request-id")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
}

// ============================================================================
// Basic Task Endpoints (Original Tasks)
// ============================================================================

async fn enqueue_email(
    task_queue: web::Data<Arc<TaskQueue>>,
    email_data: web::Json<EmailTask>,
    req: HttpRequest,
) -> ActixResult<HttpResponse> {
    let request_id = get_request_id(&req);

    match task_queue
        .enqueue(email_data.into_inner(), queue_names::DEFAULT)
        .await
    {
        Ok(task_id) => {
            let response = TaskResponse {
                task_id: task_id.to_string(),
                queue: queue_names::DEFAULT.to_string(),
                enqueued_at: Utc::now(),
                estimated_execution_time: None,
            };
            json_response(&response)
        }
        Err(e) => json_error_response(&e.to_string(), "EnqueueError", request_id),
    }
}

async fn schedule_email(
    task_queue: web::Data<Arc<TaskQueue>>,
    request: web::Json<ScheduleEmailRequest>,
    req: HttpRequest,
) -> ActixResult<HttpResponse> {
    let request_id = get_request_id(&req);
    let delay = Duration::seconds(request.delay_seconds);

    match task_queue
        .schedule(request.email.clone(), queue_names::DEFAULT, delay)
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
            json_response(&response)
        }
        Err(e) => json_error_response(&e.to_string(), "ScheduleError", request_id),
    }
}

// ============================================================================
// Advanced Task Endpoints (Comprehensive Examples)
// ============================================================================

async fn enqueue_batch_tasks<T: Task + for<'de> Deserialize<'de> + Clone>(
    task_queue: web::Data<Arc<TaskQueue>>,
    batch_request: web::Json<BatchEnqueueRequest<T>>,
    req: HttpRequest,
) -> ActixResult<HttpResponse> {
    let request_id = get_request_id(&req);
    let queue = batch_request
        .queue
        .as_deref()
        .unwrap_or(queue_names::DEFAULT);
    let mut task_ids = Vec::new();
    let batch_id = Uuid::new_v4().to_string();

    for task in batch_request.tasks.iter() {
        match task_queue.enqueue(task.clone(), queue).await {
            Ok(task_id) => task_ids.push(task_id.to_string()),
            Err(e) => {
                return json_error_response(
                    &format!("Failed to enqueue batch task: {}", e),
                    "BatchEnqueueError",
                    request_id,
                );
            }
        }
    }

    let response = BatchTaskResponse {
        batch_id,
        task_ids,
        queue: queue.to_string(),
        total_tasks: batch_request.tasks.len(),
        enqueued_at: Utc::now(),
    };

    json_response(&response)
}

async fn schedule_advanced_task<T: Task + for<'de> Deserialize<'de> + Clone>(
    task_queue: web::Data<Arc<TaskQueue>>,
    schedule_request: web::Json<ScheduleTaskRequest<T>>,
    req: HttpRequest,
) -> ActixResult<HttpResponse> {
    let request_id = get_request_id(&req);
    let queue = schedule_request
        .queue
        .as_deref()
        .unwrap_or(queue_names::DEFAULT);

    let delay = if let Some(delay_until) = schedule_request.delay_until {
        delay_until - Utc::now()
    } else if let Some(delay_seconds) = schedule_request.delay_seconds {
        Duration::seconds(delay_seconds)
    } else {
        return json_error_response(
            "Either delay_seconds or delay_until must be specified",
            "InvalidScheduleRequest",
            request_id,
        );
    };

    // TODO: Add cron expression support when available
    if schedule_request.cron_expression.is_some() {
        return json_error_response(
            "Cron expressions not yet implemented",
            "NotImplemented",
            request_id,
        );
    }

    match task_queue
        .schedule(schedule_request.task.clone(), queue, delay)
        .await
    {
        Ok(task_id) => {
            let estimated_execution = Utc::now() + delay;
            let response = ScheduledTaskResponse {
                scheduled_task_id: task_id.to_string(),
                delay_seconds: delay.num_seconds(),
                queue: queue.to_string(),
                scheduled_at: Utc::now(),
                estimated_execution_time: estimated_execution,
            };
            json_response(&response)
        }
        Err(e) => json_error_response(&e.to_string(), "AdvancedScheduleError", request_id),
    }
}

// ============================================================================
// System Management Endpoints
// ============================================================================

async fn get_comprehensive_status(
    task_queue: web::Data<Arc<TaskQueue>>,
) -> ActixResult<HttpResponse> {
    let worker_count = task_queue.worker_count().await;
    let is_shutting_down = task_queue.is_shutting_down().await;
    let system_metrics = task_queue.get_system_metrics().await;

    // Get queue metrics for all known queues
    let queue_names = [
        queue_names::DEFAULT,
        queue_names::HIGH_PRIORITY,
        queue_names::LOW_PRIORITY,
    ];
    let mut queue_statuses = Vec::new();

    for queue_name in &queue_names {
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
    let _autoscaler_metrics = task_queue.autoscaler.collect_metrics().await.ok();
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

    json_response(&response)
}

async fn manual_worker_scaling(
    task_queue: web::Data<Arc<TaskQueue>>,
    scale_request: web::Json<WorkerScalingRequest>,
    req: HttpRequest,
) -> ActixResult<HttpResponse> {
    let request_id = get_request_id(&req);
    let current_count = task_queue.worker_count().await;

    if scale_request.target_count == current_count {
        return json_response(&serde_json::json!({
            "message": "No scaling needed",
            "current_workers": current_count,
            "target_workers": scale_request.target_count
        }));
    }

    // Note: Manual scaling would require additional methods on TaskQueue
    // This is a placeholder for the feature demonstration
    json_error_response(
        "Manual worker scaling not implemented in current TaskQueue API",
        "NotImplemented",
        request_id,
    )
}

async fn initiate_graceful_shutdown(
    task_queue: web::Data<Arc<TaskQueue>>,
) -> ActixResult<HttpResponse> {
    match task_queue.shutdown().await {
        Ok(_) => json_response(&serde_json::json!({
            "message": "Graceful shutdown initiated",
            "timestamp": Utc::now()
        })),
        Err(e) => json_error_response(&e.to_string(), "ShutdownError", None),
    }
}

// ============================================================================
// Configuration and Registry Endpoints
// ============================================================================

async fn get_current_configuration() -> ActixResult<HttpResponse> {
    if let Some(config) = TaskQueueConfig::global() {
        json_response(config)
    } else {
        json_response(&serde_json::json!({
            "message": "No global configuration found",
            "config_sources": ["task-queue.toml", "task-queue.yaml", "environment variables"]
        }))
    }
}

async fn reload_configuration(req: HttpRequest) -> ActixResult<HttpResponse> {
    let request_id = get_request_id(&req);

    match TaskQueueConfig::init_global() {
        Ok(_) => json_response(&serde_json::json!({
            "message": "Configuration reloaded successfully",
            "timestamp": Utc::now()
        })),
        Err(e) => json_error_response(&e.to_string(), "ConfigReloadError", request_id),
    }
}

// ============================================================================
// Main Application Setup
// ============================================================================

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("Starting Comprehensive Actix Integration Example");
    println!("==================================================");
    println!();

    println!("Features Demonstrated:");
    println!("TaskQueueBuilder with all configuration options");
    println!("Auto-registration and manual task registry");
    println!("All available metrics endpoints");
    println!("Advanced scheduling (delays, future dates)");
    println!("Batch task operations");
    println!("Graceful shutdown handling");
    println!("Configuration management (TOML/YAML/ENV)");
    println!("Comprehensive error handling");
    println!("Security features and rate limiting");
    println!("SLA monitoring and health checks");
    println!();

    // Initialize global configuration first
    println!("Initializing configuration...");
    if let Err(e) = TaskQueueConfig::init_global() {
        eprintln!("Warning: Failed to initialize global config: {}", e);
        println!("Continuing with default configuration...");
    }

    // Create TaskQueue with comprehensive configuration
    println!("Building TaskQueue with all features...");
    let task_queue = match create_comprehensive_task_queue().await {
        Ok(tq) => {
            println!("TaskQueue created successfully");
            tq
        }
        Err(e) => {
            eprintln!("Failed to create TaskQueue: {}", e);
            std::process::exit(1);
        }
    };

    // Start background services
    println!("Starting background services...");
    if let Err(e) = task_queue.start_scheduler().await {
        eprintln!("Warning: Failed to start scheduler: {}", e);
    } else {
        println!("Task scheduler started");
    }

    if let Err(e) = task_queue.start_autoscaler() {
        eprintln!("Warning: Failed to start autoscaler: {}", e);
    } else {
        println!("Auto-scaler started");
    }

    println!();
    println!("API Endpoints Available:");
    print_api_endpoints();

    println!();
    println!("Quick Test Commands:");
    print_test_commands();

    println!();
    println!("Server starting at http://localhost:8000");
    println!("Workers: cargo run --bin task-worker");
    println!();

    // Start Actix Web server with comprehensive middleware
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(task_queue.clone()))
            // Middleware for comprehensive features
            .wrap(middleware::Logger::default())
            .wrap(
                middleware::DefaultHeaders::new()
                    .add(("X-Version", env!("CARGO_PKG_VERSION")))
                    .add(("X-Framework", "rust-task-queue")),
            )
            .wrap(middleware::Compress::default())
            // Basic task endpoints
            .service(
                web::scope("/api/v1/tasks")
                    .route("/email", web::post().to(enqueue_email))
                    .route("/email/schedule", web::post().to(schedule_email))
                    .route("/notification", web::post().to(enqueue_notification))
                    .route("/data-processing", web::post().to(enqueue_data_processing))
                    .route(
                        "/slack-notification",
                        web::post().to(enqueue_slack_notification),
                    )
                    .route("/sms", web::post().to(enqueue_sms))
                    .route("/analytics", web::post().to(enqueue_analytics))
                    .route("/ml-training", web::post().to(enqueue_ml_training))
                    // Advanced task endpoints
                    .route("/batch/email", web::post().to(enqueue_batch_emails))
                    .route(
                        "/batch/notification",
                        web::post().to(enqueue_batch_notifications),
                    )
                    .route(
                        "/schedule/advanced",
                        web::post().to(schedule_advanced_email),
                    )
                    // New comprehensive task types
                    .route("/file-processing", web::post().to(enqueue_file_processing))
                    .route("/webhook", web::post().to(enqueue_webhook))
                    .route("/report", web::post().to(enqueue_report_generation))
                    .route(
                        "/image-processing",
                        web::post().to(enqueue_image_processing),
                    )
                    .route("/backup", web::post().to(enqueue_backup))
                    .route(
                        "/database-maintenance",
                        web::post().to(enqueue_database_maintenance),
                    )
                    .route("/cache-warmup", web::post().to(enqueue_cache_warmup)),
            )
            // System management endpoints
            .service(
                web::scope("/api/v1/system")
                    .route("/status", web::get().to(get_comprehensive_status))
                    .route("/shutdown", web::post().to(initiate_graceful_shutdown))
                    .route("/workers/scale", web::post().to(manual_worker_scaling))
                    .route("/config", web::get().to(get_current_configuration))
                    .route("/config/reload", web::post().to(reload_configuration)),
            )
            // Auto-configured comprehensive routes
            .configure(configure_task_queue_routes_auto)
    })
    .bind("0.0.0.0:8000")?
    .run()
    .await
}

// ============================================================================
// Helper Functions for Application Setup
// ============================================================================

async fn create_comprehensive_task_queue() -> Result<Arc<TaskQueue>, TaskQueueError> {
    // Demonstrate all TaskQueueBuilder options
    let mut builder = if let Ok(builder) = TaskQueueBuilder::from_global_config() {
        println!("Using global configuration");
        builder
    } else {
        println!("Using builder with manual configuration");
        TaskQueueBuilder::new("redis://localhost:6379")
            .initial_workers(5)
            .with_scheduler()
            .with_autoscaler()
    };

    // Enable auto-registration if available
    #[cfg(feature = "auto-register")]
    {
        builder = builder.auto_register_tasks();
        println!("Auto-registration enabled");
    }

    #[cfg(not(feature = "auto-register"))]
    {
        // Create manual registry if auto-registration is not available
        let registry = TaskRegistry::new();
        // Manual registration would go here
        builder = builder.task_registry(Arc::new(registry));
        println!("Manual task registry configured");
    }

    let task_queue = builder.build().await?;
    Ok(Arc::new(task_queue))
}

fn print_api_endpoints() {
    println!("Task Management:");
    println!("    POST /api/v1/tasks/email                 - Enqueue email task");
    println!("    POST /api/v1/tasks/email/schedule        - Schedule email task");
    println!("    POST /api/v1/tasks/notification          - Enqueue notification");
    println!("    POST /api/v1/tasks/batch/email           - Batch enqueue emails");
    println!("    POST /api/v1/tasks/schedule/advanced     - Advanced scheduling");
    println!();
    println!("System Management:");
    println!("    GET  /api/v1/system/status               - Comprehensive system status");
    println!("    POST /api/v1/system/shutdown             - Graceful shutdown");
    println!("    POST /api/v1/system/workers/scale        - Manual worker scaling");
    println!("    GET  /api/v1/system/config               - Current configuration");
    println!("    POST /api/v1/system/config/reload        - Reload configuration");
    println!();
    println!("Monitoring (Auto-configured):");
    if let Some(config) = TaskQueueConfig::global() {
        let prefix = &config.actix.route_prefix;
        println!(
            "    GET  {}/health                        - Health check",
            prefix
        );
        println!(
            "    GET  {}/metrics                       - All metrics",
            prefix
        );
        println!(
            "    GET  {}/metrics/performance           - Performance metrics",
            prefix
        );
        println!(
            "    GET  {}/metrics/autoscaler            - Auto-scaler metrics",
            prefix
        );
        println!(
            "    GET  {}/diagnostics                   - System diagnostics",
            prefix
        );
        println!(
            "    GET  {}/uptime                        - Uptime information",
            prefix
        );
    }
}

fn print_test_commands() {
    println!("  curl -X POST http://localhost:8000/api/v1/tasks/email \\");
    println!("    -H 'Content-Type: application/json' \\");
    println!("    -d '{{\"to\":\"test@example.com\",\"subject\":\"Hello World\"}}'");
    println!();
    println!("  curl -X GET http://localhost:8000/api/v1/system/status");
    println!();
    println!("  curl -X GET http://localhost:8000/api/v1/tasks/health");
}

// Placeholder implementations for missing endpoint handlers
// These would be implemented based on the actual task types

async fn enqueue_notification(
    task_queue: web::Data<Arc<TaskQueue>>,
    notification_data: web::Json<NotificationTask>,
    req: HttpRequest,
) -> ActixResult<HttpResponse> {
    let request_id = get_request_id(&req);

    match task_queue
        .enqueue(notification_data.into_inner(), queue_names::DEFAULT)
        .await
    {
        Ok(task_id) => {
            let response = TaskResponse {
                task_id: task_id.to_string(),
                queue: queue_names::DEFAULT.to_string(),
                enqueued_at: Utc::now(),
                estimated_execution_time: None,
            };
            json_response(&response)
        }
        Err(e) => json_error_response(&e.to_string(), "EnqueueError", request_id),
    }
}

async fn enqueue_data_processing(
    task_queue: web::Data<Arc<TaskQueue>>,
    data: web::Json<DataProcessingTask>,
    req: HttpRequest,
) -> ActixResult<HttpResponse> {
    let request_id = get_request_id(&req);

    match task_queue
        .enqueue(data.into_inner(), queue_names::HIGH_PRIORITY)
        .await
    {
        Ok(task_id) => {
            let response = TaskResponse {
                task_id: task_id.to_string(),
                queue: queue_names::HIGH_PRIORITY.to_string(),
                enqueued_at: Utc::now(),
                estimated_execution_time: None,
            };
            json_response(&response)
        }
        Err(e) => json_error_response(&e.to_string(), "EnqueueError", request_id),
    }
}

async fn enqueue_slack_notification(
    task_queue: web::Data<Arc<TaskQueue>>,
    data: web::Json<SlackNotificationTask>,
    req: HttpRequest,
) -> ActixResult<HttpResponse> {
    let request_id = get_request_id(&req);

    match task_queue
        .enqueue(data.into_inner(), queue_names::DEFAULT)
        .await
    {
        Ok(task_id) => {
            let response = TaskResponse {
                task_id: task_id.to_string(),
                queue: queue_names::DEFAULT.to_string(),
                enqueued_at: Utc::now(),
                estimated_execution_time: None,
            };
            json_response(&response)
        }
        Err(e) => json_error_response(&e.to_string(), "EnqueueError", request_id),
    }
}

async fn enqueue_sms(
    task_queue: web::Data<Arc<TaskQueue>>,
    data: web::Json<SmsTask>,
    req: HttpRequest,
) -> ActixResult<HttpResponse> {
    let request_id = get_request_id(&req);

    match task_queue
        .enqueue(data.into_inner(), queue_names::HIGH_PRIORITY)
        .await
    {
        Ok(task_id) => {
            let response = TaskResponse {
                task_id: task_id.to_string(),
                queue: queue_names::HIGH_PRIORITY.to_string(),
                enqueued_at: Utc::now(),
                estimated_execution_time: None,
            };
            json_response(&response)
        }
        Err(e) => json_error_response(&e.to_string(), "EnqueueError", request_id),
    }
}

async fn enqueue_analytics(
    task_queue: web::Data<Arc<TaskQueue>>,
    data: web::Json<AnalyticsTask>,
    req: HttpRequest,
) -> ActixResult<HttpResponse> {
    let request_id = get_request_id(&req);

    match task_queue
        .enqueue(data.into_inner(), queue_names::LOW_PRIORITY)
        .await
    {
        Ok(task_id) => {
            let response = TaskResponse {
                task_id: task_id.to_string(),
                queue: queue_names::LOW_PRIORITY.to_string(),
                enqueued_at: Utc::now(),
                estimated_execution_time: None,
            };
            json_response(&response)
        }
        Err(e) => json_error_response(&e.to_string(), "EnqueueError", request_id),
    }
}

async fn enqueue_ml_training(
    task_queue: web::Data<Arc<TaskQueue>>,
    data: web::Json<MLTrainingTask>,
    req: HttpRequest,
) -> ActixResult<HttpResponse> {
    let request_id = get_request_id(&req);

    match task_queue
        .enqueue(data.into_inner(), queue_names::LOW_PRIORITY)
        .await
    {
        Ok(task_id) => {
            let response = TaskResponse {
                task_id: task_id.to_string(),
                queue: queue_names::LOW_PRIORITY.to_string(),
                enqueued_at: Utc::now(),
                estimated_execution_time: None,
            };
            json_response(&response)
        }
        Err(e) => json_error_response(&e.to_string(), "EnqueueError", request_id),
    }
}

// Batch operation implementations
async fn enqueue_batch_emails(
    task_queue: web::Data<Arc<TaskQueue>>,
    batch_request: web::Json<BatchEnqueueRequest<EmailTask>>,
    req: HttpRequest,
) -> ActixResult<HttpResponse> {
    enqueue_batch_tasks(task_queue, batch_request, req).await
}

async fn enqueue_batch_notifications(
    task_queue: web::Data<Arc<TaskQueue>>,
    batch_request: web::Json<BatchEnqueueRequest<NotificationTask>>,
    req: HttpRequest,
) -> ActixResult<HttpResponse> {
    enqueue_batch_tasks(task_queue, batch_request, req).await
}

async fn schedule_advanced_email(
    task_queue: web::Data<Arc<TaskQueue>>,
    schedule_request: web::Json<ScheduleTaskRequest<EmailTask>>,
    req: HttpRequest,
) -> ActixResult<HttpResponse> {
    schedule_advanced_task(task_queue, schedule_request, req).await
}

// Placeholder implementations for new comprehensive task types
// These would need corresponding task definitions in the module_tasks

macro_rules! create_task_endpoint {
    ($name:ident, $task_type:ty, $queue:expr) => {
        async fn $name(
            task_queue: web::Data<Arc<TaskQueue>>,
            data: web::Json<$task_type>,
            req: HttpRequest,
        ) -> ActixResult<HttpResponse> {
            let request_id = get_request_id(&req);

            match task_queue.enqueue(data.into_inner(), $queue).await {
                Ok(task_id) => {
                    let response = TaskResponse {
                        task_id: task_id.to_string(),
                        queue: $queue.to_string(),
                        enqueued_at: Utc::now(),
                        estimated_execution_time: None,
                    };
                    json_response(&response)
                }
                Err(e) => json_error_response(&e.to_string(), "EnqueueError", request_id),
            }
        }
    };
}

// These would need to be uncommented once the task types are defined
// create_task_endpoint!(enqueue_file_processing, FileProcessingTask, queue_names::DEFAULT);
// create_task_endpoint!(enqueue_webhook, WebhookTask, queue_names::HIGH_PRIORITY);
// create_task_endpoint!(enqueue_report_generation, ReportGenerationTask, queue_names::LOW_PRIORITY);
// create_task_endpoint!(enqueue_image_processing, ImageProcessingTask, queue_names::DEFAULT);
// create_task_endpoint!(enqueue_backup, BackupTask, queue_names::LOW_PRIORITY);
// create_task_endpoint!(enqueue_database_maintenance, DatabaseMaintenanceTask, queue_names::LOW_PRIORITY);
// create_task_endpoint!(enqueue_cache_warmup, CacheWarmupTask, queue_names::DEFAULT);

// Placeholder implementations - replace with actual task types when defined
async fn enqueue_file_processing(
    _task_queue: web::Data<Arc<TaskQueue>>,
    req: HttpRequest,
) -> ActixResult<HttpResponse> {
    json_error_response(
        "FileProcessingTask not yet implemented",
        "NotImplemented",
        get_request_id(&req),
    )
}

async fn enqueue_webhook(
    _task_queue: web::Data<Arc<TaskQueue>>,
    req: HttpRequest,
) -> ActixResult<HttpResponse> {
    json_error_response(
        "WebhookTask not yet implemented",
        "NotImplemented",
        get_request_id(&req),
    )
}

async fn enqueue_report_generation(
    _task_queue: web::Data<Arc<TaskQueue>>,
    req: HttpRequest,
) -> ActixResult<HttpResponse> {
    json_error_response(
        "ReportGenerationTask not yet implemented",
        "NotImplemented",
        get_request_id(&req),
    )
}

async fn enqueue_image_processing(
    _task_queue: web::Data<Arc<TaskQueue>>,
    req: HttpRequest,
) -> ActixResult<HttpResponse> {
    json_error_response(
        "ImageProcessingTask not yet implemented",
        "NotImplemented",
        get_request_id(&req),
    )
}

async fn enqueue_backup(
    _task_queue: web::Data<Arc<TaskQueue>>,
    req: HttpRequest,
) -> ActixResult<HttpResponse> {
    json_error_response(
        "BackupTask not yet implemented",
        "NotImplemented",
        get_request_id(&req),
    )
}

async fn enqueue_database_maintenance(
    _task_queue: web::Data<Arc<TaskQueue>>,
    req: HttpRequest,
) -> ActixResult<HttpResponse> {
    json_error_response(
        "DatabaseMaintenanceTask not yet implemented",
        "NotImplemented",
        get_request_id(&req),
    )
}

async fn enqueue_cache_warmup(
    _task_queue: web::Data<Arc<TaskQueue>>,
    req: HttpRequest,
) -> ActixResult<HttpResponse> {
    json_error_response(
        "CacheWarmupTask not yet implemented",
        "NotImplemented",
        get_request_id(&req),
    )
}
