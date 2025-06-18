//! Actix Web integration helpers with comprehensive metrics endpoints

use crate::prelude::*;
#[cfg(feature = "actix-integration")]
use actix_web::{web, HttpResponse, Result as ActixResult};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(feature = "actix-integration")]
/// Detailed health check with component status
async fn detailed_health_check(task_queue: web::Data<Arc<TaskQueue>>) -> ActixResult<HttpResponse> {
    match task_queue.health_check().await {
        Ok(health_status) => Ok(HttpResponse::Ok().json(health_status)),
        Err(e) => Ok(HttpResponse::ServiceUnavailable().json(json!({
            "status": "unhealthy",
            "error": e.to_string(),
            "timestamp": Utc::now()
        }))),
    }
}

#[cfg(feature = "actix-integration")]
/// System status with health metrics
async fn system_status(task_queue: web::Data<Arc<TaskQueue>>) -> ActixResult<HttpResponse> {
    let health_status = task_queue.metrics.get_health_status().await;
    let worker_count = task_queue.worker_count().await;
    let is_shutting_down = task_queue.is_shutting_down().await;

    Ok(HttpResponse::Ok().json(json!({
        "health": health_status,
        "workers": {
            "active_count": worker_count,
            "shutting_down": is_shutting_down
        },
        "timestamp": Utc::now()
    })))
}

#[cfg(feature = "actix-integration")]
/// Comprehensive metrics combining all available metrics
async fn get_comprehensive_metrics(
    task_queue: web::Data<Arc<TaskQueue>>,
) -> ActixResult<HttpResponse> {
    // Handle mixed Result and non-Result return types separately
    let system_metrics = task_queue.get_system_metrics().await;
    let worker_count = task_queue.worker_count().await;

    match tokio::try_join!(
        task_queue.get_metrics(),
        task_queue.autoscaler.collect_metrics(),
        task_queue.autoscaler.get_scaling_recommendations()
    ) {
        Ok((queue_metrics, autoscaler_metrics, scaling_report)) => {
            Ok(HttpResponse::Ok().json(json!({
                "timestamp": Utc::now(),
                "task_queue_metrics": queue_metrics,
                "system_metrics": system_metrics,
                "autoscaler_metrics": autoscaler_metrics,
                "scaling_report": scaling_report,
                "worker_count": worker_count
            })))
        }
        Err(e) => Ok(HttpResponse::InternalServerError().json(json!({
            "error": e.to_string(),
            "timestamp": Utc::now()
        }))),
    }
}

#[cfg(feature = "actix-integration")]
/// Enhanced system metrics with memory and performance data
async fn get_system_metrics(task_queue: web::Data<Arc<TaskQueue>>) -> ActixResult<HttpResponse> {
    let system_metrics = task_queue.get_system_metrics().await;
    Ok(HttpResponse::Ok().json(system_metrics))
}

#[cfg(feature = "actix-integration")]
/// Performance metrics and performance report
async fn get_performance_metrics(
    task_queue: web::Data<Arc<TaskQueue>>,
) -> ActixResult<HttpResponse> {
    let performance_report = task_queue.metrics.get_performance_report().await;
    Ok(HttpResponse::Ok().json(performance_report))
}

#[cfg(feature = "actix-integration")]
/// AutoScaler metrics and recommendations
async fn get_autoscaler_metrics(
    task_queue: web::Data<Arc<TaskQueue>>,
) -> ActixResult<HttpResponse> {
    match tokio::try_join!(
        task_queue.autoscaler.collect_metrics(),
        task_queue.autoscaler.get_scaling_recommendations()
    ) {
        Ok((metrics, recommendations)) => Ok(HttpResponse::Ok().json(json!({
            "metrics": metrics,
            "recommendations": recommendations,
            "timestamp": Utc::now()
        }))),
        Err(e) => Ok(HttpResponse::InternalServerError().json(json!({
            "error": e.to_string(),
            "timestamp": Utc::now()
        }))),
    }
}

#[cfg(feature = "actix-integration")]
/// Individual queue metrics for all queues
async fn get_queue_metrics(task_queue: web::Data<Arc<TaskQueue>>) -> ActixResult<HttpResponse> {
    let queues = ["default", "high_priority", "low_priority"];
    let mut queue_metrics = Vec::new();
    let mut errors = Vec::new();

    for queue in &queues {
        match task_queue.broker.get_queue_metrics(queue).await {
            Ok(metrics) => queue_metrics.push(metrics),
            Err(e) => errors.push(json!({
                "queue": queue,
                "error": e.to_string()
            })),
        }
    }

    Ok(HttpResponse::Ok().json(json!({
        "queue_metrics": queue_metrics,
        "errors": errors,
        "timestamp": Utc::now()
    })))
}

#[cfg(feature = "actix-integration")]
/// Worker-specific metrics
async fn get_worker_metrics(task_queue: web::Data<Arc<TaskQueue>>) -> ActixResult<HttpResponse> {
    let worker_count = task_queue.worker_count().await;
    let system_metrics = task_queue.get_system_metrics().await;

    Ok(HttpResponse::Ok().json(json!({
        "active_workers": worker_count,
        "worker_metrics": system_metrics.workers,
        "is_shutting_down": task_queue.is_shutting_down().await,
        "timestamp": Utc::now()
    })))
}

#[cfg(feature = "actix-integration")]
/// Memory usage metrics
async fn get_memory_metrics(task_queue: web::Data<Arc<TaskQueue>>) -> ActixResult<HttpResponse> {
    let system_metrics = task_queue.get_system_metrics().await;
    Ok(HttpResponse::Ok().json(json!({
        "memory_metrics": system_metrics.memory,
        "timestamp": Utc::now()
    })))
}

#[cfg(feature = "actix-integration")]
/// Quick metrics summary for debugging
async fn get_metrics_summary(task_queue: web::Data<Arc<TaskQueue>>) -> ActixResult<HttpResponse> {
    let summary = task_queue.get_metrics_summary().await;
    Ok(HttpResponse::Ok().json(json!({
        "summary": summary,
        "timestamp": Utc::now()
    })))
}

#[cfg(all(feature = "actix-integration", feature = "auto-register"))]
/// Get auto-registered tasks information
async fn get_registered_tasks() -> ActixResult<HttpResponse> {
    match TaskRegistry::with_auto_registered() {
        Ok(registry) => {
            let registered_tasks = registry.registered_tasks();
            Ok(HttpResponse::Ok().json(json!({
                "auto_registered_tasks": registered_tasks,
                "count": registered_tasks.len(),
                "auto_registration_enabled": true,
                "timestamp": Utc::now()
            })))
        }
        Err(e) => Ok(HttpResponse::InternalServerError().json(json!({
            "error": e.to_string(),
            "auto_registration_enabled": true,
            "timestamp": Utc::now()
        }))),
    }
}

#[cfg(not(all(feature = "actix-integration", feature = "auto-register")))]
/// Fallback when auto-register feature is not enabled
async fn get_registered_tasks() -> ActixResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(json!({
        "message": "Auto-registration feature not enabled",
        "auto_registration_enabled": false,
        "timestamp": Utc::now()
    })))
}

#[cfg(feature = "actix-integration")]
/// Detailed task registry information
async fn get_registry_info() -> ActixResult<HttpResponse> {
    #[cfg(feature = "auto-register")]
    {
        match TaskRegistry::with_auto_registered() {
            Ok(registry) => {
                let registered_tasks = registry.registered_tasks();
                Ok(HttpResponse::Ok().json(json!({
                    "registry_type": "auto_registered",
                    "task_count": registered_tasks.len(),
                    "tasks": registered_tasks,
                    "features": {
                        "auto_register": true,
                        "inventory_based": true
                    },
                    "timestamp": Utc::now()
                })))
            }
            Err(e) => Ok(HttpResponse::InternalServerError().json(json!({
                "error": e.to_string(),
                "registry_type": "auto_registered",
                "timestamp": Utc::now()
            }))),
        }
    }
    #[cfg(not(feature = "auto-register"))]
    {
        Ok(HttpResponse::Ok().json(json!({
            "registry_type": "manual",
            "message": "Auto-registration not available - manual registry in use",
            "features": {
                "auto_register": false,
                "inventory_based": false
            },
            "timestamp": Utc::now()
        })))
    }
}

#[cfg(feature = "actix-integration")]
/// Get active alerts from the metrics system
async fn get_active_alerts(task_queue: web::Data<Arc<TaskQueue>>) -> ActixResult<HttpResponse> {
    let performance_report = task_queue.metrics.get_performance_report().await;
    Ok(HttpResponse::Ok().json(json!({
        "active_alerts": performance_report.active_alerts,
        "alert_count": performance_report.active_alerts.len(),
        "timestamp": Utc::now()
    })))
}

#[cfg(feature = "actix-integration")]
/// SLA status and violations
async fn get_sla_status(task_queue: web::Data<Arc<TaskQueue>>) -> ActixResult<HttpResponse> {
    let performance_report = task_queue.metrics.get_performance_report().await;
    let system_metrics = task_queue.get_system_metrics().await;

    Ok(HttpResponse::Ok().json(json!({
        "sla_violations": performance_report.sla_violations,
        "violation_count": performance_report.sla_violations.len(),
        "performance_metrics": system_metrics.performance,
        "success_rate_percentage": system_metrics.performance.success_rate * 100.0,
        "error_rate_percentage": system_metrics.performance.error_rate * 100.0,
        "timestamp": Utc::now()
    })))
}

#[cfg(feature = "actix-integration")]
/// Comprehensive diagnostics information
async fn get_diagnostics(task_queue: web::Data<Arc<TaskQueue>>) -> ActixResult<HttpResponse> {
    let health_status = task_queue.metrics.get_health_status().await;
    let performance_report = task_queue.metrics.get_performance_report().await;
    let worker_count = task_queue.worker_count().await;
    let is_shutting_down = task_queue.is_shutting_down().await;

    // Get individual queue diagnostics
    let queues = ["default", "high_priority", "low_priority"];
    let mut queue_diagnostics = HashMap::new();

    for queue in &queues {
        if let Ok(metrics) = task_queue.broker.get_queue_metrics(queue).await {
            queue_diagnostics.insert(queue.to_string(), json!({
                "pending_tasks": metrics.pending_tasks,
                "processed_tasks": metrics.processed_tasks,
                "failed_tasks": metrics.failed_tasks,
                "health": if metrics.failed_tasks > 0 && metrics.processed_tasks > 0 {
                    let error_rate = metrics.failed_tasks as f64 / (metrics.processed_tasks + metrics.failed_tasks) as f64;
                    if error_rate > 0.1 { "unhealthy" } else if error_rate > 0.05 { "warning" } else { "healthy" }
                } else {
                    "healthy"
                }
            }));
        }
    }

    Ok(HttpResponse::Ok().json(json!({
        "system_health": health_status,
        "performance_report": performance_report,
        "worker_diagnostics": {
            "active_count": worker_count,
            "shutting_down": is_shutting_down
        },
        "queue_diagnostics": queue_diagnostics,
        "uptime_seconds": performance_report.uptime_seconds,
        "timestamp": Utc::now()
    })))
}

#[cfg(feature = "actix-integration")]
/// System uptime and runtime information
async fn get_uptime_info(task_queue: web::Data<Arc<TaskQueue>>) -> ActixResult<HttpResponse> {
    let system_metrics = task_queue.get_system_metrics().await;
    let performance_report = task_queue.metrics.get_performance_report().await;

    let uptime_duration = std::time::Duration::from_secs(system_metrics.uptime_seconds);
    let days = uptime_duration.as_secs() / 86400;
    let hours = (uptime_duration.as_secs() % 86400) / 3600;
    let minutes = (uptime_duration.as_secs() % 3600) / 60;
    let seconds = uptime_duration.as_secs() % 60;

    Ok(HttpResponse::Ok().json(json!({
        "uptime": {
            "seconds": system_metrics.uptime_seconds,
            "formatted": format!("{}d {}h {}m {}s", days, hours, minutes, seconds),
            "started_at": Utc::now() - Duration::seconds(system_metrics.uptime_seconds as i64)
        },
        "runtime_info": {
            "total_tasks_executed": system_metrics.tasks.total_executed,
            "total_tasks_succeeded": system_metrics.tasks.total_succeeded,
            "total_tasks_failed": system_metrics.tasks.total_failed,
            "success_rate_percentage": system_metrics.performance.success_rate * 100.0,
            "average_tasks_per_second": system_metrics.performance.tasks_per_second
        },
        "performance_summary": {
            "task_performance": performance_report.task_performance
        },
        "timestamp": Utc::now()
    })))
}

#[cfg(feature = "actix-integration")]
/// Helper for creating TaskQueue with auto-registration for Actix Web apps
pub async fn create_auto_registered_task_queue(
    redis_url: &str,
    initial_workers: Option<usize>,
) -> Result<Arc<TaskQueue>, TaskQueueError> {
    let mut builder = TaskQueueBuilder::new(redis_url);

    #[cfg(feature = "auto-register")]
    {
        builder = builder.auto_register_tasks();
    }

    if let Some(workers) = initial_workers {
        builder = builder.initial_workers(workers);
    }

    Ok(Arc::new(builder.build().await?))
}

#[cfg(feature = "actix-integration")]
/// Create TaskQueue from global configuration - ideal for Actix apps
pub async fn create_task_queue_from_config() -> Result<Arc<TaskQueue>, TaskQueueError> {
    let task_queue = TaskQueueBuilder::from_global_config()?.build().await?;
    Ok(Arc::new(task_queue))
}

#[cfg(feature = "actix-integration")]
/// Auto-configure TaskQueue for Actix Web with minimal setup
pub async fn auto_configure_task_queue() -> Result<Arc<TaskQueue>, TaskQueueError> {
    // Initialize global configuration
    TaskQueueConfig::init_global()?;

    // Create TaskQueue from global config
    let task_queue = TaskQueueBuilder::from_global_config()?.build().await?;
    Ok(Arc::new(task_queue))
}

#[cfg(feature = "actix-integration")]
/// Configure Actix Web routes based on global configuration
pub fn configure_task_queue_routes_auto(cfg: &mut web::ServiceConfig) {
    if let Some(config) = TaskQueueConfig::global() {
        #[cfg(feature = "actix-integration")]
        if config.actix.auto_configure_routes {
            let prefix = &config.actix.route_prefix;
            let mut scope = web::scope(prefix);

            if config.actix.enable_health_check {
                scope = scope
                    .route("/health", web::get().to(detailed_health_check))
                    .route("/status", web::get().to(system_status));
            }

            if config.actix.enable_metrics {
                scope = scope
                    .route("/metrics", web::get().to(get_comprehensive_metrics))
                    .route("/metrics/system", web::get().to(get_system_metrics))
                    .route(
                        "/metrics/performance",
                        web::get().to(get_performance_metrics),
                    )
                    .route("/metrics/autoscaler", web::get().to(get_autoscaler_metrics))
                    .route("/metrics/queues", web::get().to(get_queue_metrics))
                    .route("/metrics/workers", web::get().to(get_worker_metrics))
                    .route("/metrics/memory", web::get().to(get_memory_metrics))
                    .route("/metrics/summary", web::get().to(get_metrics_summary))
                    .route("/registered", web::get().to(get_registered_tasks))
                    .route("/registry/info", web::get().to(get_registry_info))
                    .route("/alerts", web::get().to(get_active_alerts))
                    .route("/sla", web::get().to(get_sla_status))
                    .route("/diagnostics", web::get().to(get_diagnostics))
                    .route("/uptime", web::get().to(get_uptime_info));
            }

            cfg.service(scope);
        }
    } else {
        // Fallback to default comprehensive configuration
        configure_task_queue_routes(cfg);
    }
}

#[cfg(feature = "actix-integration")]
/// Comprehensive Actix Web service configuration with all metrics endpoints
pub fn configure_task_queue_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/tasks")
            // Health and Status endpoints
            .route("/health", web::get().to(detailed_health_check))
            .route("/status", web::get().to(system_status))
            // Metrics endpoints
            .route("/metrics", web::get().to(get_comprehensive_metrics))
            .route("/metrics/system", web::get().to(get_system_metrics))
            .route(
                "/metrics/performance",
                web::get().to(get_performance_metrics),
            )
            .route("/metrics/autoscaler", web::get().to(get_autoscaler_metrics))
            .route("/metrics/queues", web::get().to(get_queue_metrics))
            .route("/metrics/workers", web::get().to(get_worker_metrics))
            .route("/metrics/memory", web::get().to(get_memory_metrics))
            .route("/metrics/summary", web::get().to(get_metrics_summary))
            // Task Registry endpoints
            .route("/registered", web::get().to(get_registered_tasks))
            .route("/registry/info", web::get().to(get_registry_info))
            // Administrative endpoints
            .route("/alerts", web::get().to(get_active_alerts))
            .route("/sla", web::get().to(get_sla_status))
            .route("/diagnostics", web::get().to(get_diagnostics))
            .route("/uptime", web::get().to(get_uptime_info)),
    );
}

#[cfg(feature = "actix-integration")]
/// Helper macro for creating typed task endpoints
#[macro_export]
macro_rules! create_task_endpoint {
    ($task_type:ty, $queue:expr) => {
        async fn enqueue_task(
            task_queue: actix_web::web::Data<std::sync::Arc<$crate::TaskQueue>>,
            task_data: actix_web::web::Json<$task_type>,
        ) -> actix_web::Result<actix_web::HttpResponse> {
            match task_queue.enqueue(task_data.into_inner(), $queue).await {
                Ok(task_id) => {
                    let mut response = std::collections::HashMap::new();
                    response.insert("task_id", task_id.to_string());
                    response.insert("queue", $queue.to_string());
                    response.insert("status", "enqueued".to_string());
                    Ok(actix_web::HttpResponse::Ok().json(response))
                }
                Err(e) => {
                    let mut response = std::collections::HashMap::new();
                    response.insert("error", e.to_string());
                    response.insert("queue", $queue.to_string());
                    Ok(actix_web::HttpResponse::InternalServerError().json(response))
                }
            }
        }
    };
}
