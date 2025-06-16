//! Simple Task App - Actix Web Server with Auto-Configuration
mod module_tasks;
mod tasks; // Original tasks from legacy_tasks.rs // New modular tasks from tasks/ directory

// Import the legacy tasks (from legacy_tasks.rs)
use tasks::{DataProcessingTask, EmailTask, NotificationTask, ScheduleEmailRequest};

// Import the new organized tasks modules (from tasks/ directory)
use module_tasks::{
    // Processing tasks
    AnalyticsTask,
    MLTrainingTask,
    // Communication tasks
    SlackNotificationTask,
    SmsTask,
};

use actix_web::web::Json;
use actix_web::{web, App, HttpResponse, HttpServer, Result as ActixResult};
use rust_task_queue::prelude::*;
use rust_task_queue::TaskQueue;
use rust_task_queue::TaskQueueConfig;
use serde::Serialize;
use std::sync::Arc;
// Removed duplicate imports - these are already imported from legacy_tasks

// API Response structs for JSON
#[derive(Serialize)]
struct TaskResponse {
    task_id: String,
    queue: String,
}

#[derive(Serialize)]
struct ScheduledTaskResponse {
    scheduled_task_id: String,
    delay_seconds: i64,
    queue: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

// Helper function to create JSON responses
fn json_response<T: Serialize>(data: &T) -> ActixResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(data))
}

fn json_error_response<T: Serialize>(data: &T) -> ActixResult<HttpResponse> {
    Ok(HttpResponse::InternalServerError().json(data))
}

// API endpoints for enqueueing tasks
async fn enqueue_email(
    task_queue: web::Data<Arc<TaskQueue>>,
    email_data: Json<EmailTask>,
) -> ActixResult<HttpResponse> {
    match task_queue.enqueue(email_data.into_inner(), "default").await {
        Ok(task_id) => {
            println!("Enqueued email task: {}", task_id);
            let response = TaskResponse {
                task_id: task_id.to_string(),
                queue: "default".to_string(),
            };
            json_response(&response)
        }
        Err(e) => {
            let error_response = ErrorResponse {
                error: e.to_string(),
            };
            json_error_response(&error_response)
        }
    }
}

async fn enqueue_notification(
    task_queue: web::Data<Arc<TaskQueue>>,
    notification_data: Json<NotificationTask>,
) -> ActixResult<HttpResponse> {
    match task_queue
        .enqueue(notification_data.into_inner(), "default")
        .await
    {
        Ok(task_id) => {
            println!("Enqueued notification task: {}", task_id);
            let response = TaskResponse {
                task_id: task_id.to_string(),
                queue: "default".to_string(),
            };
            json_response(&response)
        }
        Err(e) => {
            let error_response = ErrorResponse {
                error: e.to_string(),
            };
            json_error_response(&error_response)
        }
    }
}

async fn enqueue_data_processing(
    task_queue: web::Data<Arc<TaskQueue>>,
    data: Json<DataProcessingTask>,
) -> ActixResult<HttpResponse> {
    match task_queue.enqueue(data.into_inner(), "high_priority").await {
        Ok(task_id) => {
            println!("Enqueued data processing task: {}", task_id);
            let response = TaskResponse {
                task_id: task_id.to_string(),
                queue: "high_priority".to_string(),
            };
            json_response(&response)
        }
        Err(e) => {
            let error_response = ErrorResponse {
                error: e.to_string(),
            };
            json_error_response(&error_response)
        }
    }
}

async fn schedule_email(
    task_queue: web::Data<Arc<TaskQueue>>,
    request: Json<ScheduleEmailRequest>,
) -> ActixResult<HttpResponse> {
    let delay = chrono::Duration::seconds(request.delay_seconds);
    match task_queue
        .schedule(request.email.clone(), "default", delay)
        .await
    {
        Ok(task_id) => {
            println!(
                "⏰ Scheduled email task: {} (delay: {}s)",
                task_id, request.delay_seconds
            );
            let response = ScheduledTaskResponse {
                scheduled_task_id: task_id.to_string(),
                delay_seconds: request.delay_seconds,
                queue: "default".to_string(),
            };
            json_response(&response)
        }
        Err(e) => {
            let error_response = ErrorResponse {
                error: e.to_string(),
            };
            json_error_response(&error_response)
        }
    }
}

// New endpoints for modular tasks

async fn enqueue_slack_notification(
    task_queue: web::Data<Arc<TaskQueue>>,
    data: Json<SlackNotificationTask>,
) -> ActixResult<HttpResponse> {
    match task_queue.enqueue(data.into_inner(), "default").await {
        Ok(task_id) => {
            println!("Enqueued Slack notification: {}", task_id);
            let response = TaskResponse {
                task_id: task_id.to_string(),
                queue: "default".to_string(),
            };
            json_response(&response)
        }
        Err(e) => {
            let error_response = ErrorResponse {
                error: e.to_string(),
            };
            json_error_response(&error_response)
        }
    }
}

async fn enqueue_sms(
    task_queue: web::Data<Arc<TaskQueue>>,
    data: Json<SmsTask>,
) -> ActixResult<HttpResponse> {
    match task_queue.enqueue(data.into_inner(), "high_priority").await {
        Ok(task_id) => {
            println!("Enqueued SMS: {}", task_id);
            let response = TaskResponse {
                task_id: task_id.to_string(),
                queue: "high_priority".to_string(),
            };
            json_response(&response)
        }
        Err(e) => {
            let error_response = ErrorResponse {
                error: e.to_string(),
            };
            json_error_response(&error_response)
        }
    }
}

async fn enqueue_analytics(
    task_queue: web::Data<Arc<TaskQueue>>,
    data: Json<AnalyticsTask>,
) -> ActixResult<HttpResponse> {
    match task_queue.enqueue(data.into_inner(), "low_priority").await {
        Ok(task_id) => {
            println!("Enqueued analytics task: {}", task_id);
            let response = TaskResponse {
                task_id: task_id.to_string(),
                queue: "low_priority".to_string(),
            };
            json_response(&response)
        }
        Err(e) => {
            let error_response = ErrorResponse {
                error: e.to_string(),
            };
            json_error_response(&error_response)
        }
    }
}

async fn enqueue_ml_training(
    task_queue: web::Data<Arc<TaskQueue>>,
    data: Json<MLTrainingTask>,
) -> ActixResult<HttpResponse> {
    match task_queue.enqueue(data.into_inner(), "low_priority").await {
        Ok(task_id) => {
            println!("Enqueued ML training task: {}", task_id);
            let response = TaskResponse {
                task_id: task_id.to_string(),
                queue: "low_priority".to_string(),
            };
            json_response(&response)
        }
        Err(e) => {
            let error_response = ErrorResponse {
                error: e.to_string(),
            };
            json_error_response(&error_response)
        }
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::init();

    println!("Starting Actix Integration App!");
    println!("Looking for task-queue.toml, task-queue.yaml, or environment variables...");

    // Auto-configure everything with dynamic loading support!
    let task_queue = auto_configure_task_queue().await.unwrap();

    println!("Actix Integration Initialized!");
    println!("API Endpoints:");
    println!("   Original Tasks:");
    println!("     POST /email              - Enqueue email task");
    println!("     POST /schedule-email     - Schedule email task");
    println!("     POST /notification       - Enqueue notification task");
    println!("     POST /data-processing    - Enqueue data processing task");
    println!("   Modular Tasks (demonstrating modular organization):");
    println!("     POST /slack-notification - Enqueue Slack notification");
    println!("     POST /sms               - Enqueue SMS");
    println!("     POST /analytics         - Enqueue analytics task");
    println!("     POST /ml-training       - Enqueue ML training task");

    // Show auto-configured routes
    if let Some(config) = TaskQueueConfig::global() {
        if config.actix.auto_configure_routes {
            println!(
                "   Auto-configured routes at {}:",
                config.actix.route_prefix
            );
            if config.actix.enable_health_check {
                println!(
                    "     GET  {}/health      - Health check",
                    config.actix.route_prefix
                );
            }
            if config.actix.enable_metrics {
                println!(
                    "     GET  {}/metrics     - Auto-scaler metrics",
                    config.actix.route_prefix
                );
                println!(
                    "     GET  {}/registered  - Registered tasks",
                    config.actix.route_prefix
                );
            }
        }
    }

    println!();
    println!("Server running at http://localhost:8000");
    println!("Start workers with: cargo run --bin task-worker");
    println!("Try: curl -X POST http://localhost:8000/email -H 'Content-Type: application/json' -d '{{\"to\":\"test@example.com\",\"subject\":\"Hello World\"}}'");

    // Start Actix Web server
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(task_queue.clone()))
            // Original task routes
            .route("/email", web::post().to(enqueue_email))
            .route("/schedule-email", web::post().to(schedule_email))
            .route("/notification", web::post().to(enqueue_notification))
            .route("/data-processing", web::post().to(enqueue_data_processing))
            // New modular task routes (demonstrating modular organization)
            .route(
                "/slack-notification",
                web::post().to(enqueue_slack_notification),
            )
            .route("/sms", web::post().to(enqueue_sms))
            .route("/analytics", web::post().to(enqueue_analytics))
            .route("/ml-training", web::post().to(enqueue_ml_training))
            // Auto-configured routes based on configuration
            .configure(configure_task_queue_routes_auto)
    })
    .bind("0.0.0.0:8000")?
    .run()
    .await
}
