//! Actix Web integration helpers with auto-registration support

use crate::prelude::*;
#[cfg(feature = "actix-integration")]
use actix_web::{web, HttpResponse, Result as ActixResult};
// MessagePack serialization utilities for responses
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(feature = "actix-integration")]
/// Actix Web service configuration helper with auto-registration support
pub fn configure_task_queue_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/tasks")
            .route("/health", web::get().to(health_check))
            .route("/metrics", web::get().to(get_metrics))
            .route("/registered", web::get().to(get_registered_tasks)),
    );
}

#[cfg(feature = "actix-integration")]
async fn health_check() -> ActixResult<HttpResponse> {
    let mut response = HashMap::new();
    response.insert("status", "healthy");
    Ok(HttpResponse::Ok().json(response))
}

#[cfg(feature = "actix-integration")]
async fn get_metrics(task_queue: web::Data<Arc<TaskQueue>>) -> ActixResult<HttpResponse> {
    match task_queue.autoscaler.get_scaling_recommendations().await {
        Ok(report) => {
            let mut response = HashMap::new();
            response.insert("metrics", report);
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            let mut response = HashMap::new();
            response.insert("error", e.to_string());
            Ok(HttpResponse::InternalServerError().json(response))
        }
    }
}

#[cfg(all(feature = "actix-integration", feature = "auto-register"))]
async fn get_registered_tasks() -> ActixResult<HttpResponse> {
    match TaskRegistry::with_auto_registered() {
        Ok(registry) => {
            let registered_tasks = registry.registered_tasks();
            let mut response = HashMap::new();
            response.insert("auto_registered_tasks", registered_tasks.clone());
            response.insert("count", vec![registered_tasks.len().to_string()]);
            response.insert("auto_registration_enabled", vec!["true".to_string()]);
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            let mut response = HashMap::new();
            response.insert("error", vec![e.to_string()]);
            response.insert("auto_registration_enabled", vec!["true".to_string()]);
            Ok(HttpResponse::InternalServerError().json(response))
        }
    }
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
/// This function will:
/// 1. Load configuration from files/environment
/// 2. Initialize global config
/// 3. Create TaskQueue with auto-registration
/// 4. Return configured TaskQueue ready for use
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
                scope = scope.route("/health", web::get().to(health_check));
            }

            if config.actix.enable_metrics {
                scope = scope.route("/metrics", web::get().to(get_metrics));
                scope = scope.route("/registered", web::get().to(get_registered_tasks));
            }

            cfg.service(scope);
        }
    } else {
        // Fallback to default configuration
        configure_task_queue_routes(cfg);
    }
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
                    Ok(actix_web::HttpResponse::Ok().json(response))
                }
                Err(e) => {
                    let mut response = std::collections::HashMap::new();
                    response.insert("error", e.to_string());
                    Ok(actix_web::HttpResponse::InternalServerError().json(response))
                }
            }
        }
    };
}
