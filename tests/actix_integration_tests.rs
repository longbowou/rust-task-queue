#![cfg(feature = "actix-integration")]

use actix_web::{test, web, App};
use rust_task_queue::prelude::*;
use rust_task_queue::queue::queue_names;
use rust_task_queue::actix::configure_task_queue_routes;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

// Global counter for unique database numbers
static DB_COUNTER: AtomicU8 = AtomicU8::new(10); // Use 10+ to avoid conflicts with other tests

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ActixTestTask {
    data: String,
    should_fail: bool,
}

#[async_trait::async_trait]
impl Task for ActixTestTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        if self.should_fail {
            return Err("ActixTestTask intentionally failed".into());
        }

        sleep(Duration::from_millis(10)).await;

        #[derive(Serialize)]
        struct Response {
            status: String,
            processed_data: String,
        }

        let response = Response {
            status: "completed".to_string(),
            processed_data: format!("Processed: {}", self.data),
        };

        Ok(rmp_serde::to_vec(&response)?)
    }

    fn name(&self) -> &str {
        "actix_test_task"
    }

    fn max_retries(&self) -> u32 {
        1
    }

    fn timeout_seconds(&self) -> u64 {
        10
    }
}

fn setup_isolated_redis_url() -> String {
    let db_num = DB_COUNTER.fetch_add(1, Ordering::SeqCst);
    let base_url =
        std::env::var("REDIS_TEST_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    format!("{}/{}", base_url, db_num)
}

async fn cleanup_test_database(redis_url: &str) {
    if let Ok(client) = redis::Client::open(redis_url) {
        if let Ok(mut conn) = client.get_async_connection().await {
            let _: Result<String, _> = redis::cmd("FLUSHDB").query_async(&mut conn).await;
        }
    }
    sleep(Duration::from_millis(50)).await;
}

async fn setup_test_task_queue() -> (Arc<TaskQueue>, String) {
    let redis_url = setup_isolated_redis_url();
    cleanup_test_database(&redis_url).await;

    // Try to create task queue, with fallback to database 0 if isolated database fails
    let task_queue = match TaskQueue::new(&redis_url).await {
        Ok(tq) => tq,
        Err(_) => {
            // Fallback to main database if isolated database switch fails
            let fallback_url = std::env::var("REDIS_TEST_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
            TaskQueue::new(&fallback_url)
                .await
                .expect("Failed to create task queue even with fallback")
        }
    };

    // Set up task registry
    let registry = TaskRegistry::new();
    registry
        .register_with_name::<ActixTestTask>("actix_test_task")
        .expect("Failed to register task");

    // Start minimal workers for testing
    task_queue
        .start_workers_with_registry(1, Arc::new(registry))
        .await
        .expect("Failed to start workers");

    // Start scheduler for comprehensive testing
    task_queue
        .start_scheduler()
        .await
        .expect("Failed to start scheduler");

    // Add some test data for metrics
    let test_task = ActixTestTask {
        data: "Test data for metrics".to_string(),
        should_fail: false,
    };
    
    // Enqueue a successful task
    let _ = task_queue
        .enqueue(test_task, queue_names::DEFAULT)
        .await
        .expect("Failed to enqueue test task");

    // Wait for task to process
    sleep(Duration::from_millis(500)).await;

    (Arc::new(task_queue), redis_url)
}

async fn cleanup_task_queue(task_queue: &TaskQueue, redis_url: &str) {
    task_queue.stop_workers().await;
    task_queue.stop_scheduler().await;
    sleep(Duration::from_millis(200)).await;
    cleanup_test_database(redis_url).await;
}

// Test helper to validate JSON response structure
fn validate_json_response(body: &Value, expected_fields: &[&str]) {
    for field in expected_fields {
        assert!(
            body.get(field).is_some(),
            "Missing required field '{}' in response: {}",
            field,
            body
        );
    }
}

#[tokio::test]
async fn test_health_endpoint() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    println!("Running test_health_endpoint with ID: {}", test_id);

    let (task_queue, redis_url) = setup_test_task_queue().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(task_queue.clone()))
            .configure(configure_task_queue_routes),
    )
    .await;

    let req = test::TestRequest::get().uri("/tasks/health").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    validate_json_response(&body, &["status", "timestamp", "components"]);

    assert_eq!(body["status"], "healthy");
    assert!(body["components"].is_object());

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_health_endpoint");
}

#[tokio::test]
async fn test_system_status_endpoint() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    println!("Running test_system_status_endpoint with ID: {}", test_id);

    let (task_queue, redis_url) = setup_test_task_queue().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(task_queue.clone()))
            .configure(configure_task_queue_routes),
    )
    .await;

    let req = test::TestRequest::get().uri("/tasks/status").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    validate_json_response(&body, &["health", "workers", "timestamp"]);

    assert!(body["health"].is_object());
    assert!(body["workers"]["active_count"].is_number());
    assert!(body["workers"]["shutting_down"].is_boolean());

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_system_status_endpoint");
}

#[tokio::test]
async fn test_comprehensive_metrics_endpoint() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    println!("Running test_comprehensive_metrics_endpoint with ID: {}", test_id);

    let (task_queue, redis_url) = setup_test_task_queue().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(task_queue.clone()))
            .configure(configure_task_queue_routes),
    )
    .await;

    let req = test::TestRequest::get().uri("/tasks/metrics").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    validate_json_response(&body, &[
        "timestamp",
        "task_queue_metrics",
        "system_metrics", 
        "autoscaler_metrics",
        "scaling_report",
        "worker_count"
    ]);

    // Validate nested structures
    assert!(body["task_queue_metrics"]["queue_metrics"].is_array());
    assert!(body["system_metrics"]["memory"].is_object());
    assert!(body["system_metrics"]["performance"].is_object());
    assert!(body["autoscaler_metrics"]["active_workers"].is_number());

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_comprehensive_metrics_endpoint");
}

#[tokio::test]
async fn test_system_metrics_endpoint() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    println!("Running test_system_metrics_endpoint with ID: {}", test_id);

    let (task_queue, redis_url) = setup_test_task_queue().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(task_queue.clone()))
            .configure(configure_task_queue_routes),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/tasks/metrics/system")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    validate_json_response(&body, &[
        "timestamp",
        "uptime_seconds",
        "memory",
        "performance",
        "tasks",
        "workers"
    ]);

    // Validate memory metrics
    let memory = &body["memory"];
    assert!(memory["current_bytes"].is_number());
    assert!(memory["peak_bytes"].is_number());
    assert!(memory["active_tasks"].is_number());

    // Validate performance metrics
    let performance = &body["performance"];
    assert!(performance["tasks_per_second"].is_number());
    assert!(performance["success_rate"].is_number());
    assert!(performance["error_rate"].is_number());

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_system_metrics_endpoint");
}

#[tokio::test]
async fn test_performance_metrics_endpoint() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    println!("Running test_performance_metrics_endpoint with ID: {}", test_id);

    let (task_queue, redis_url) = setup_test_task_queue().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(task_queue.clone()))
            .configure(configure_task_queue_routes),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/tasks/metrics/performance")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    validate_json_response(&body, &[
        "uptime_seconds",
        "task_performance",
        "active_alerts",
        "sla_violations"
    ]);

    assert!(body["task_performance"].is_object());
    assert!(body["active_alerts"].is_array());
    assert!(body["sla_violations"].is_array());

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_performance_metrics_endpoint");
}

#[tokio::test]
async fn test_autoscaler_metrics_endpoint() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    println!("Running test_autoscaler_metrics_endpoint with ID: {}", test_id);

    let (task_queue, redis_url) = setup_test_task_queue().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(task_queue.clone()))
            .configure(configure_task_queue_routes),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/tasks/metrics/autoscaler")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    validate_json_response(&body, &["metrics", "recommendations", "timestamp"]);

    // Validate autoscaler metrics structure
    let metrics = &body["metrics"];
    assert!(metrics["active_workers"].is_number());
    assert!(metrics["total_pending_tasks"].is_number());
    assert!(metrics["queue_metrics"].is_array());
    assert!(metrics["queue_pressure_score"].is_number());
    assert!(metrics["worker_utilization"].is_number());

    assert!(body["recommendations"].is_string());

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_autoscaler_metrics_endpoint");
}

#[tokio::test]
async fn test_queue_metrics_endpoint() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    println!("Running test_queue_metrics_endpoint with ID: {}", test_id);

    let (task_queue, redis_url) = setup_test_task_queue().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(task_queue.clone()))
            .configure(configure_task_queue_routes),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/tasks/metrics/queues")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    validate_json_response(&body, &["queue_metrics", "errors", "timestamp"]);

    let queue_metrics = body["queue_metrics"].as_array().unwrap();
    assert_eq!(queue_metrics.len(), 3); // default, high_priority, low_priority

    // Validate each queue metric has required fields
    for queue_metric in queue_metrics {
        assert!(queue_metric["queue_name"].is_string());
        assert!(queue_metric["pending_tasks"].is_number());
        assert!(queue_metric["processed_tasks"].is_number());
        assert!(queue_metric["failed_tasks"].is_number());
    }

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_queue_metrics_endpoint");
}

#[tokio::test]
async fn test_worker_metrics_endpoint() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    println!("Running test_worker_metrics_endpoint with ID: {}", test_id);

    let (task_queue, redis_url) = setup_test_task_queue().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(task_queue.clone()))
            .configure(configure_task_queue_routes),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/tasks/metrics/workers")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    validate_json_response(&body, &[
        "active_workers",
        "worker_metrics",
        "is_shutting_down",
        "timestamp"
    ]);

    assert!(body["active_workers"].is_number());
    assert!(body["worker_metrics"].is_object());
    assert!(body["is_shutting_down"].is_boolean());

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_worker_metrics_endpoint");
}

#[tokio::test]
async fn test_memory_metrics_endpoint() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    println!("Running test_memory_metrics_endpoint with ID: {}", test_id);

    let (task_queue, redis_url) = setup_test_task_queue().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(task_queue.clone()))
            .configure(configure_task_queue_routes),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/tasks/metrics/memory")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    validate_json_response(&body, &["memory_metrics", "timestamp"]);

    let memory_metrics = &body["memory_metrics"];
    assert!(memory_metrics["current_bytes"].is_number());
    assert!(memory_metrics["peak_bytes"].is_number());
    assert!(memory_metrics["total_allocations"].is_number());
    assert!(memory_metrics["active_tasks"].is_number());

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_memory_metrics_endpoint");
}

#[tokio::test]
async fn test_metrics_summary_endpoint() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    println!("Running test_metrics_summary_endpoint with ID: {}", test_id);

    let (task_queue, redis_url) = setup_test_task_queue().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(task_queue.clone()))
            .configure(configure_task_queue_routes),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/tasks/metrics/summary")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    validate_json_response(&body, &["summary", "timestamp"]);

    assert!(body["summary"].is_string());
    let summary = body["summary"].as_str().unwrap();
    assert!(summary.contains("TaskQueue Metrics Summary"));
    assert!(summary.contains("Uptime"));
    assert!(summary.contains("Tasks"));

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_metrics_summary_endpoint");
}

#[tokio::test]
async fn test_registry_endpoint() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    println!("Running test_registry_endpoint with ID: {}", test_id);

    let (task_queue, redis_url) = setup_test_task_queue().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(task_queue.clone()))
            .configure(configure_task_queue_routes),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/tasks/registry")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    validate_json_response(&body, &["registry_type", "features", "timestamp"]);

    assert!(body["registry_type"].is_string());
    assert!(body["features"].is_object());
    assert!(body["features"]["auto_register"].is_boolean());

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_registry_info_endpoint");
}

#[tokio::test]
async fn test_alerts_endpoint() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    println!("Running test_alerts_endpoint with ID: {}", test_id);

    let (task_queue, redis_url) = setup_test_task_queue().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(task_queue.clone()))
            .configure(configure_task_queue_routes),
    )
    .await;

    let req = test::TestRequest::get().uri("/tasks/alerts").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    validate_json_response(&body, &["active_alerts", "alert_count", "timestamp"]);

    assert!(body["active_alerts"].is_array());
    assert!(body["alert_count"].is_number());

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_alerts_endpoint");
}

#[tokio::test]
async fn test_sla_status_endpoint() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    println!("Running test_sla_status_endpoint with ID: {}", test_id);

    let (task_queue, redis_url) = setup_test_task_queue().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(task_queue.clone()))
            .configure(configure_task_queue_routes),
    )
    .await;

    let req = test::TestRequest::get().uri("/tasks/sla").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    validate_json_response(&body, &[
        "sla_violations",
        "violation_count",
        "performance_metrics",
        "success_rate_percentage",
        "error_rate_percentage",
        "timestamp"
    ]);

    assert!(body["sla_violations"].is_array());
    assert!(body["violation_count"].is_number());
    assert!(body["performance_metrics"].is_object());
    assert!(body["success_rate_percentage"].is_number());
    assert!(body["error_rate_percentage"].is_number());

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_sla_status_endpoint");
}

#[tokio::test]
async fn test_diagnostics_endpoint() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    println!("Running test_diagnostics_endpoint with ID: {}", test_id);

    let (task_queue, redis_url) = setup_test_task_queue().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(task_queue.clone()))
            .configure(configure_task_queue_routes),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/tasks/diagnostics")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    validate_json_response(&body, &[
        "system_health",
        "performance_report",
        "worker_diagnostics",
        "queue_diagnostics",
        "uptime_seconds",
        "timestamp"
    ]);

    assert!(body["system_health"].is_object());
    assert!(body["performance_report"].is_object());
    assert!(body["worker_diagnostics"].is_object());
    assert!(body["queue_diagnostics"].is_object());
    assert!(body["uptime_seconds"].is_number());

    // Validate queue diagnostics structure
    let queue_diagnostics = body["queue_diagnostics"].as_object().unwrap();
    assert!(queue_diagnostics.contains_key("default"));
    assert!(queue_diagnostics.contains_key("high_priority"));
    assert!(queue_diagnostics.contains_key("low_priority"));

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_diagnostics_endpoint");
}

#[tokio::test]
async fn test_uptime_info_endpoint() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    println!("Running test_uptime_info_endpoint with ID: {}", test_id);

    let (task_queue, redis_url) = setup_test_task_queue().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(task_queue.clone()))
            .configure(configure_task_queue_routes),
    )
    .await;

    let req = test::TestRequest::get().uri("/tasks/uptime").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    validate_json_response(&body, &[
        "uptime",
        "runtime_info",
        "performance_summary",
        "timestamp"
    ]);

    // Validate uptime structure
    let uptime = &body["uptime"];
    assert!(uptime["seconds"].is_number());
    assert!(uptime["formatted"].is_string());
    assert!(uptime["started_at"].is_string());

    // Validate runtime info
    let runtime_info = &body["runtime_info"];
    assert!(runtime_info["total_tasks_executed"].is_number());
    assert!(runtime_info["success_rate_percentage"].is_number());
    assert!(runtime_info["average_tasks_per_second"].is_number());

    // Verify formatted uptime contains expected elements
    let formatted = uptime["formatted"].as_str().unwrap();
    assert!(formatted.contains("d") && formatted.contains("h") && formatted.contains("m") && formatted.contains("s"));

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_uptime_info_endpoint");
}

#[tokio::test]
async fn test_error_handling_for_invalid_endpoints() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    println!("Running test_error_handling_for_invalid_endpoints with ID: {}", test_id);

    let (task_queue, redis_url) = setup_test_task_queue().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(task_queue.clone()))
            .configure(configure_task_queue_routes),
    )
    .await;

    // Test non-existent endpoint
    let req = test::TestRequest::get()
        .uri("/tasks/nonexistent")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);

    // Test invalid method on valid endpoint
    let req = test::TestRequest::post().uri("/tasks/health").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 405);

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_error_handling_for_invalid_endpoints");
}

#[tokio::test]
async fn test_concurrent_endpoint_requests() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    println!("Running test_concurrent_endpoint_requests with ID: {}", test_id);

    let (task_queue, redis_url) = setup_test_task_queue().await;

    // Test multiple concurrent requests to different endpoints sequentially
    // (since actix test app can't be cloned, we test them one by one rapidly)
    let endpoints = vec![
        "/tasks/health",
        "/tasks/status", 
        "/tasks/metrics/system",
        "/tasks/metrics/queues",
        "/tasks/metrics/workers",
        "/tasks/uptime",
    ];

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(task_queue.clone()))
            .configure(configure_task_queue_routes),
    )
    .await;

    // Test all endpoints rapidly to simulate concurrent load
    for endpoint in endpoints {
        let req = test::TestRequest::get().uri(endpoint).to_request();
        let resp = test::call_service(&app, req).await;
        assert!(
            resp.status().is_success(), 
            "Endpoint {} failed during concurrent test", 
            endpoint
        );
    }

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_concurrent_endpoint_requests");
}

#[tokio::test]
async fn test_comprehensive_metrics_with_real_data() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    println!("Running test_comprehensive_metrics_with_real_data with ID: {}", test_id);

    let (task_queue, redis_url) = setup_test_task_queue().await;

    // Add more test data for realistic metrics
    for i in 0..5 {
        let task = ActixTestTask {
            data: format!("Real test data {}", i),
            should_fail: i % 4 == 0, // Some tasks fail for error rate testing
        };
        
        let queue = match i % 3 {
            0 => queue_names::HIGH_PRIORITY,
            1 => queue_names::DEFAULT,
            _ => queue_names::LOW_PRIORITY,
        };
        
        let _ = task_queue.enqueue(task, queue).await.expect("Failed to enqueue task");
    }

    // Wait for tasks to process
    sleep(Duration::from_millis(1000)).await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(task_queue.clone()))
            .configure(configure_task_queue_routes),
    )
    .await;

    // Test comprehensive metrics with real data
    let req = test::TestRequest::get().uri("/tasks/metrics").to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    
    // Verify we have realistic task counts
    let task_queue_metrics = &body["task_queue_metrics"];
    assert!(task_queue_metrics["total_processed_tasks"].as_i64().unwrap() > 0);
    
    // Test queue-specific metrics
    let req = test::TestRequest::get().uri("/tasks/metrics/queues").to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    let queue_metrics = body["queue_metrics"].as_array().unwrap();
    
    // Verify we have tasks in different queues
    let mut total_processed = 0;
    for queue in queue_metrics {
        total_processed += queue["processed_tasks"].as_i64().unwrap_or(0);
    }
    assert!(total_processed > 0, "Expected processed tasks but got {}", total_processed);

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_comprehensive_metrics_with_real_data");
}

#[tokio::test]
async fn test_json_response_consistency() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    println!("Running test_json_response_consistency with ID: {}", test_id);

    let (task_queue, redis_url) = setup_test_task_queue().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(task_queue.clone()))
            .configure(configure_task_queue_routes),
    )
    .await;

    // Test that all endpoints return valid JSON with consistent structure
    let endpoints_with_timestamp = vec![
        "/tasks/health",
        "/tasks/status",
        "/tasks/metrics",
        "/tasks/metrics/system",
        "/tasks/metrics/autoscaler",
        "/tasks/metrics/queues",
        "/tasks/metrics/workers",
        "/tasks/metrics/memory",
        "/tasks/metrics/summary",
        "/tasks/registry",
        "/tasks/alerts",
        "/tasks/sla",
        "/tasks/diagnostics",
        "/tasks/uptime",
    ];

    for endpoint in endpoints_with_timestamp {
        let req = test::TestRequest::get().uri(endpoint).to_request();
        let resp = test::call_service(&app, req).await;
        
        assert!(
            resp.status().is_success(), 
            "Endpoint {} returned non-success status: {}", 
            endpoint, 
            resp.status()
        );

        let body: Value = test::read_body_json(resp).await;
        
        // Every endpoint should have a timestamp
        assert!(
            body.get("timestamp").is_some(),
            "Endpoint {} missing timestamp in response: {}",
            endpoint,
            body
        );

        // Verify timestamp format is valid ISO 8601
        let timestamp_str = body["timestamp"].as_str().unwrap();
        assert!(
            timestamp_str.contains("T") && timestamp_str.contains("Z"),
            "Endpoint {} has invalid timestamp format: {}",
            endpoint,
            timestamp_str
        );
    }

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_json_response_consistency");
}

#[tokio::test]
async fn test_endpoint_performance_and_response_times() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    println!("Running test_endpoint_performance_and_response_times with ID: {}", test_id);

    let (task_queue, redis_url) = setup_test_task_queue().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(task_queue.clone()))
            .configure(configure_task_queue_routes),
    )
    .await;

    // Test that all endpoints respond within reasonable time
    let start_time = std::time::Instant::now();
    
    let req = test::TestRequest::get().uri("/tasks/metrics").to_request();
    let resp = test::call_service(&app, req).await;
    
    let elapsed = start_time.elapsed();
    assert!(resp.status().is_success());
    assert!(elapsed.as_millis() < 5000, "Comprehensive metrics endpoint took too long: {:?}", elapsed);

    // Test system metrics performance
    let start_time = std::time::Instant::now();
    
    let req = test::TestRequest::get().uri("/tasks/metrics/system").to_request();
    let resp = test::call_service(&app, req).await;
    
    let elapsed = start_time.elapsed();
    assert!(resp.status().is_success());
    assert!(elapsed.as_millis() < 1000, "System metrics endpoint took too long: {:?}", elapsed);

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_endpoint_performance_and_response_times");
}

#[tokio::test]
async fn test_metrics_data_accuracy() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    println!("Running test_metrics_data_accuracy with ID: {}", test_id);

    let (task_queue, redis_url) = setup_test_task_queue().await;

    // Add specific test data to verify accuracy
    let success_task = ActixTestTask {
        data: "Success task for accuracy test".to_string(),
        should_fail: false,
    };
    
    let failure_task = ActixTestTask {
        data: "Failure task for accuracy test".to_string(),
        should_fail: true,
    };

    // Enqueue tasks to different queues
    let _ = task_queue.enqueue(success_task.clone(), queue_names::HIGH_PRIORITY).await;
    let _ = task_queue.enqueue(failure_task, queue_names::DEFAULT).await;
    let _ = task_queue.enqueue(success_task, queue_names::LOW_PRIORITY).await;

    // Wait for processing
    sleep(Duration::from_millis(1500)).await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(task_queue.clone()))
            .configure(configure_task_queue_routes),
    )
    .await;

    // Test queue metrics accuracy
    let req = test::TestRequest::get().uri("/tasks/metrics/queues").to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    
    let queue_metrics = body["queue_metrics"].as_array().unwrap();
    let mut total_processed = 0;
    let mut total_failed = 0;
    
    for queue in queue_metrics {
        total_processed += queue["processed_tasks"].as_i64().unwrap_or(0);
        total_failed += queue["failed_tasks"].as_i64().unwrap_or(0);
    }

    // We should have some processed tasks and at least one failure
    assert!(total_processed > 0, "Expected processed tasks but got {}", total_processed);
    assert!(total_failed > 0, "Expected failed tasks but got {}", total_failed);

    // Test system metrics accuracy
    let req = test::TestRequest::get().uri("/tasks/metrics/system").to_request();
    let resp = test::call_service(&app, req).await;
    let body: Value = test::read_body_json(resp).await;
    
    assert!(body["tasks"]["total_executed"].as_u64().unwrap() > 0);
    assert!(body["tasks"]["total_succeeded"].as_u64().unwrap() > 0);
    assert!(body["tasks"]["total_failed"].as_u64().unwrap() > 0);

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_metrics_data_accuracy");
}

#[tokio::test]
async fn test_edge_cases_and_error_conditions() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    println!("Running test_edge_cases_and_error_conditions with ID: {}", test_id);

    let (task_queue, redis_url) = setup_test_task_queue().await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(task_queue.clone()))
            .configure(configure_task_queue_routes),
    )
    .await;

    // Test with malformed requests
    let req = test::TestRequest::get()
        .uri("/tasks/metrics?invalid=param")
        .to_request();
    let resp = test::call_service(&app, req).await;
    // Should still work despite query params
    assert!(resp.status().is_success());

    // Test with extra slashes
    let req = test::TestRequest::get()
        .uri("/tasks//metrics//system")
        .to_request();
    let resp = test::call_service(&app, req).await;
    // This should fail as the path doesn't match
    assert!(!resp.status().is_success());

    // Test case sensitivity
    let req = test::TestRequest::get()
        .uri("/Tasks/Health") // Wrong case
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);

    // Test with valid but long paths
    let req = test::TestRequest::get()
        .uri("/tasks/metrics/system/../system")
        .to_request();
    let resp = test::call_service(&app, req).await;
    // Should be handled by actix routing
    assert!(!resp.status().is_success());

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_edge_cases_and_error_conditions");
} 