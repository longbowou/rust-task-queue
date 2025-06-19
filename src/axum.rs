//! Axum integration helpers with comprehensive metrics endpoints

use crate::prelude::*;
use crate::queue_names;
#[cfg(feature = "axum-integration")]
use axum::{
    extract::State,
    http::StatusCode,
    response::Json as ResponseJson,
    // Json,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(feature = "axum-integration")]
/// Detailed health check with component status
async fn detailed_health_check(
    State(task_queue): State<Arc<TaskQueue>>,
) -> Result<ResponseJson<serde_json::Value>, (StatusCode, ResponseJson<serde_json::Value>)> {
    match task_queue.health_check().await {
        Ok(health_status) => Ok(ResponseJson(json!(health_status))),
        Err(e) => Err((
            StatusCode::SERVICE_UNAVAILABLE,
            ResponseJson(json!({
                "status": "unhealthy",
                "error": e.to_string(),
                "timestamp": Utc::now()
            })),
        )),
    }
}

#[cfg(feature = "axum-integration")]
/// System status with health metrics
async fn system_status(
    State(task_queue): State<Arc<TaskQueue>>,
) -> Result<ResponseJson<serde_json::Value>, (StatusCode, ResponseJson<serde_json::Value>)> {
    let health_status = task_queue.metrics.get_health_status().await;
    let worker_count = task_queue.worker_count().await;
    let is_shutting_down = task_queue.is_shutting_down().await;

    Ok(ResponseJson(json!({
        "health": health_status,
        "workers": {
            "active_count": worker_count,
            "shutting_down": is_shutting_down
        },
        "timestamp": Utc::now()
    })))
}

#[cfg(feature = "axum-integration")]
/// Comprehensive metrics combining all available metrics
async fn get_comprehensive_metrics(
    State(task_queue): State<Arc<TaskQueue>>,
) -> Result<ResponseJson<serde_json::Value>, (StatusCode, ResponseJson<serde_json::Value>)> {
    // Handle mixed Result and non-Result return types separately
    let system_metrics = task_queue.get_system_metrics().await;
    let worker_count = task_queue.worker_count().await;

    match tokio::try_join!(
        task_queue.get_metrics(),
        task_queue.autoscaler.collect_metrics(),
    ) {
        Ok((queue_metrics, autoscaler_metrics)) => Ok(ResponseJson(json!({
            "timestamp": Utc::now(),
            "task_queue_metrics": queue_metrics,
            "system_metrics": system_metrics,
            "autoscaler_metrics": autoscaler_metrics,
            "worker_count": worker_count
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            ResponseJson(json!({
                "error": e.to_string(),
                "timestamp": Utc::now()
            })),
        )),
    }
}

#[cfg(feature = "axum-integration")]
/// Enhanced system metrics with memory and performance data
async fn get_system_metrics(
    State(task_queue): State<Arc<TaskQueue>>,
) -> ResponseJson<serde_json::Value> {
    let system_metrics = task_queue.get_system_metrics().await;
    ResponseJson(json!(system_metrics))
}

#[cfg(feature = "axum-integration")]
/// Performance metrics and performance report
async fn get_performance_metrics(
    State(task_queue): State<Arc<TaskQueue>>,
) -> ResponseJson<serde_json::Value> {
    let performance_report = task_queue.metrics.get_performance_report().await;
    ResponseJson(json!(performance_report))
}

#[cfg(feature = "axum-integration")]
/// AutoScaler metrics and recommendations
async fn get_autoscaler_metrics(
    State(task_queue): State<Arc<TaskQueue>>,
) -> Result<ResponseJson<serde_json::Value>, (StatusCode, ResponseJson<serde_json::Value>)> {
    match tokio::try_join!(task_queue.autoscaler.collect_metrics(),) {
        Ok(metrics) => Ok(ResponseJson(json!({
            "metrics": metrics,
            "timestamp": Utc::now()
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            ResponseJson(json!({
                "error": e.to_string(),
                "timestamp": Utc::now()
            })),
        )),
    }
}

#[cfg(feature = "axum-integration")]
/// Individual queue metrics for all queues
async fn get_queue_metrics(
    State(task_queue): State<Arc<TaskQueue>>,
) -> ResponseJson<serde_json::Value> {
    let queues = [
        queue_names::DEFAULT,
        queue_names::LOW_PRIORITY,
        queue_names::HIGH_PRIORITY,
    ];
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

    ResponseJson(json!({
        "queue_metrics": queue_metrics,
        "errors": errors,
        "timestamp": Utc::now()
    }))
}

#[cfg(feature = "axum-integration")]
/// Worker-specific metrics
async fn get_worker_metrics(
    State(task_queue): State<Arc<TaskQueue>>,
) -> ResponseJson<serde_json::Value> {
    let worker_count = task_queue.worker_count().await;
    let system_metrics = task_queue.get_system_metrics().await;

    ResponseJson(json!({
        "active_workers": worker_count,
        "worker_metrics": system_metrics.workers,
        "is_shutting_down": task_queue.is_shutting_down().await,
        "timestamp": Utc::now()
    }))
}

#[cfg(feature = "axum-integration")]
/// Memory usage metrics
async fn get_memory_metrics(
    State(task_queue): State<Arc<TaskQueue>>,
) -> ResponseJson<serde_json::Value> {
    let system_metrics = task_queue.get_system_metrics().await;
    ResponseJson(json!({
        "memory_metrics": system_metrics.memory,
        "timestamp": Utc::now()
    }))
}

#[cfg(not(all(feature = "axum-integration", feature = "auto-register")))]
/// Fallback when auto-register feature is not enabled
#[allow(dead_code)]
async fn get_registered_tasks() -> ResponseJson<serde_json::Value> {
    ResponseJson(json!({
        "message": "Auto-registration feature not enabled",
        "auto_registration_enabled": false,
        "timestamp": Utc::now()
    }))
}

#[cfg(feature = "axum-integration")]
/// Detailed task registry information
async fn get_registry_info() -> ResponseJson<serde_json::Value> {
    #[cfg(feature = "auto-register")]
    {
        match TaskRegistry::with_auto_registered() {
            Ok(registry) => {
                let registered_tasks = registry.registered_tasks();
                ResponseJson(json!({
                    "registry_type": "auto_registered",
                    "task_count": registered_tasks.len(),
                    "tasks": registered_tasks,
                    "features": {
                        "auto_register": true,
                        "inventory_based": true
                    },
                    "timestamp": Utc::now()
                }))
            }
            Err(e) => ResponseJson(json!({
                "error": e.to_string(),
                "registry_type": "auto_registered",
                "timestamp": Utc::now()
            })),
        }
    }
    #[cfg(not(feature = "auto-register"))]
    {
        ResponseJson(json!({
            "registry_type": "manual",
            "message": "Auto-registration not available - manual registry in use",
            "features": {
                "auto_register": false,
                "inventory_based": false
            },
            "timestamp": Utc::now()
        }))
    }
}

#[cfg(feature = "axum-integration")]
/// Get active alerts from the metrics system
async fn get_active_alerts(
    State(task_queue): State<Arc<TaskQueue>>,
) -> ResponseJson<serde_json::Value> {
    let performance_report = task_queue.metrics.get_performance_report().await;
    ResponseJson(json!({
        "active_alerts": performance_report.active_alerts,
        "alert_count": performance_report.active_alerts.len(),
        "timestamp": Utc::now()
    }))
}

#[cfg(feature = "axum-integration")]
/// SLA status and violations
async fn get_sla_status(
    State(task_queue): State<Arc<TaskQueue>>,
) -> ResponseJson<serde_json::Value> {
    let performance_report = task_queue.metrics.get_performance_report().await;
    let system_metrics = task_queue.get_system_metrics().await;

    ResponseJson(json!({
        "sla_violations": performance_report.sla_violations,
        "violation_count": performance_report.sla_violations.len(),
        "performance_metrics": system_metrics.performance,
        "success_rate_percentage": system_metrics.performance.success_rate * 100.0,
        "error_rate_percentage": system_metrics.performance.error_rate * 100.0,
        "timestamp": Utc::now()
    }))
}

#[cfg(feature = "axum-integration")]
/// Comprehensive diagnostics information
async fn get_diagnostics(
    State(task_queue): State<Arc<TaskQueue>>,
) -> ResponseJson<serde_json::Value> {
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

    ResponseJson(json!({
        "system_health": health_status,
        "performance_report": performance_report,
        "worker_diagnostics": {
            "active_count": worker_count,
            "shutting_down": is_shutting_down
        },
        "queue_diagnostics": queue_diagnostics,
        "uptime_seconds": performance_report.uptime_seconds,
        "timestamp": Utc::now()
    }))
}

#[cfg(feature = "axum-integration")]
/// System uptime and runtime information
async fn get_uptime_info(
    State(task_queue): State<Arc<TaskQueue>>,
) -> ResponseJson<serde_json::Value> {
    let system_metrics = task_queue.get_system_metrics().await;
    let performance_report = task_queue.metrics.get_performance_report().await;

    let uptime_duration = std::time::Duration::from_secs(system_metrics.uptime_seconds);
    let days = uptime_duration.as_secs() / 86400;
    let hours = (uptime_duration.as_secs() % 86400) / 3600;
    let minutes = (uptime_duration.as_secs() % 3600) / 60;
    let seconds = uptime_duration.as_secs() % 60;

    ResponseJson(json!({
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
    }))
}

#[cfg(feature = "axum-integration")]
/// Helper for creating TaskQueue with auto-registration for Axum apps
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

#[cfg(feature = "axum-integration")]
/// Create TaskQueue from global configuration - ideal for Axum apps
pub async fn create_task_queue_from_config() -> Result<Arc<TaskQueue>, TaskQueueError> {
    let task_queue = TaskQueueBuilder::from_global_config()?.build().await?;
    Ok(Arc::new(task_queue))
}

#[cfg(feature = "axum-integration")]
/// Auto-configure TaskQueue for Axum with minimal setup
pub async fn auto_configure_task_queue() -> Result<Arc<TaskQueue>, TaskQueueError> {
    // Initialize global configuration
    TaskQueueConfig::init_global()?;

    // Create TaskQueue from global config
    let task_queue = TaskQueueBuilder::from_global_config()?.build().await?;
    Ok(Arc::new(task_queue))
}

#[cfg(feature = "axum-integration")]
/// Configure Axum routes based on global configuration
pub fn configure_task_queue_routes_auto() -> axum::Router<Arc<TaskQueue>> {
    use axum::routing::get;

    if let Some(config) = TaskQueueConfig::global() {
        #[cfg(feature = "axum-integration")]
        if config.axum.auto_configure_routes {
            let prefix = &config.axum.route_prefix;
            let mut router = axum::Router::new();

            if config.axum.enable_health_check {
                router = router
                    .route("/health", get(detailed_health_check))
                    .route("/status", get(system_status));
            }

            if config.axum.enable_metrics {
                router = router
                    .route("/metrics", get(get_comprehensive_metrics))
                    .route("/metrics/system", get(get_system_metrics))
                    .route("/metrics/performance", get(get_performance_metrics))
                    .route("/metrics/autoscaler", get(get_autoscaler_metrics))
                    .route("/metrics/queues", get(get_queue_metrics))
                    .route("/metrics/workers", get(get_worker_metrics))
                    .route("/metrics/memory", get(get_memory_metrics))
                    .route("/registry", get(get_registry_info))
                    .route("/alerts", get(get_active_alerts))
                    .route("/sla", get(get_sla_status))
                    .route("/diagnostics", get(get_diagnostics))
                    .route("/uptime", get(get_uptime_info));
            }

            return axum::Router::new().nest(prefix, router);
        }
    }

    // Fallback to default comprehensive configuration
    configure_task_queue_routes()
}

#[cfg(feature = "axum-integration")]
/// Comprehensive Axum router configuration with all metrics endpoints
pub fn configure_task_queue_routes() -> axum::Router<Arc<TaskQueue>> {
    use axum::routing::get;

    axum::Router::new().nest(
        "/tasks",
        axum::Router::new()
            // Health and Status endpoints
            .route("/health", get(detailed_health_check))
            .route("/status", get(system_status))
            // Metrics endpoints
            .route("/metrics", get(get_comprehensive_metrics))
            .route("/metrics/system", get(get_system_metrics))
            .route("/metrics/performance", get(get_performance_metrics))
            .route("/metrics/autoscaler", get(get_autoscaler_metrics))
            .route("/metrics/queues", get(get_queue_metrics))
            .route("/metrics/workers", get(get_worker_metrics))
            .route("/metrics/memory", get(get_memory_metrics))
            // Task Registry endpoints
            .route("/registry", get(get_registry_info))
            // Administrative endpoints
            .route("/alerts", get(get_active_alerts))
            .route("/sla", get(get_sla_status))
            .route("/diagnostics", get(get_diagnostics))
            .route("/uptime", get(get_uptime_info)),
    )
}

#[cfg(feature = "axum-integration")]
/// Helper macro for creating typed task endpoints for Axum
#[macro_export]
macro_rules! create_axum_task_endpoint {
    ($task_type:ty, $queue:expr) => {
        async fn enqueue_task(
            axum::extract::State(task_queue): axum::extract::State<std::sync::Arc<$crate::TaskQueue>>,
            axum::Json(task_data): axum::Json<$task_type>,
        ) -> Result<
            axum::Json<std::collections::HashMap<String, String>>,
            (
                axum::http::StatusCode,
                axum::Json<std::collections::HashMap<String, String>>,
            ),
        > {
            match task_queue.enqueue(task_data, $queue).await {
                Ok(task_id) => {
                    let mut response = std::collections::HashMap::new();
                    response.insert("task_id".to_string(), task_id.to_string());
                    response.insert("queue".to_string(), $queue.to_string());
                    response.insert("status".to_string(), "enqueued".to_string());
                    Ok(axum::Json(response))
                }
                Err(e) => {
                    let mut response = std::collections::HashMap::new();
                    response.insert("error".to_string(), e.to_string());
                    response.insert("queue".to_string(), $queue.to_string());
                    Err((
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        axum::Json(response),
                    ))
                }
            }
        }
    };
} 