#![cfg(feature = "axum-integration")]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use rust_task_queue::axum::configure_task_queue_routes;
use rust_task_queue::prelude::*;
use rust_task_queue::queue::queue_names;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tower::util::ServiceExt;
use uuid::Uuid;

// Global counter for unique database numbers
static DB_COUNTER: AtomicU8 = AtomicU8::new(20); // Use 20+ to avoid conflicts with other tests

#[derive(Debug, Serialize, Deserialize, Clone)]
struct AxumTestTask {
    data: String,
    should_fail: bool,
}

#[async_trait::async_trait]
impl Task for AxumTestTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        if self.should_fail {
            return Err("AxumTestTask intentionally failed".into());
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
        "axum_test_task"
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
        .register_with_name::<AxumTestTask>("axum_test_task")
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
    let test_task = AxumTestTask {
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

    let app = configure_task_queue_routes().with_state(task_queue.clone());

    let request = Request::builder()
        .uri("/tasks/health")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_body: Value = serde_json::from_slice(&body).unwrap();

    validate_json_response(&json_body, &["status", "timestamp", "components"]);
    assert_eq!(json_body["status"], "healthy");
    assert!(json_body["components"].is_object());

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_health_endpoint");
}

#[tokio::test]
async fn test_system_status_endpoint() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    println!("Running test_system_status_endpoint with ID: {}", test_id);

    let (task_queue, redis_url) = setup_test_task_queue().await;

    let app = configure_task_queue_routes().with_state(task_queue.clone());

    let request = Request::builder()
        .uri("/tasks/status")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_body: Value = serde_json::from_slice(&body).unwrap();

    validate_json_response(&json_body, &["health", "workers", "timestamp"]);
    assert!(json_body["health"].is_object());
    assert!(json_body["workers"]["active_count"].is_number());
    assert!(json_body["workers"]["shutting_down"].is_boolean());

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_system_status_endpoint");
}

#[tokio::test]
async fn test_comprehensive_metrics_endpoint() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    println!(
        "Running test_comprehensive_metrics_endpoint with ID: {}",
        test_id
    );

    let (task_queue, redis_url) = setup_test_task_queue().await;

    let app = configure_task_queue_routes().with_state(task_queue.clone());

    let request = Request::builder()
        .uri("/tasks/metrics")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_body: Value = serde_json::from_slice(&body).unwrap();

    validate_json_response(
        &json_body,
        &[
            "timestamp",
            "task_queue_metrics",
            "system_metrics",
            "autoscaler_metrics",
            "worker_count",
        ],
    );

    // Validate nested structures
    assert!(json_body["task_queue_metrics"]["queue_metrics"].is_array());
    assert!(json_body["system_metrics"]["memory"].is_object());
    assert!(json_body["system_metrics"]["performance"].is_object());
    assert!(json_body["autoscaler_metrics"]["active_workers"].is_number());

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_comprehensive_metrics_endpoint");
}

#[tokio::test]
async fn test_system_metrics_endpoint() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    println!("Running test_system_metrics_endpoint with ID: {}", test_id);

    let (task_queue, redis_url) = setup_test_task_queue().await;

    let app = configure_task_queue_routes().with_state(task_queue.clone());

    let request = Request::builder()
        .uri("/tasks/metrics/system")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_body: Value = serde_json::from_slice(&body).unwrap();

    validate_json_response(
        &json_body,
        &[
            "timestamp",
            "uptime_seconds",
            "memory",
            "performance",
            "tasks",
            "workers",
        ],
    );

    // Validate memory metrics
    let memory = &json_body["memory"];
    assert!(memory["current_bytes"].is_number());
    assert!(memory["peak_bytes"].is_number());
    assert!(memory["active_tasks"].is_number());

    // Validate performance metrics
    let performance = &json_body["performance"];
    assert!(performance["tasks_per_second"].is_number());
    assert!(performance["success_rate"].is_number());
    assert!(performance["error_rate"].is_number());

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_system_metrics_endpoint");
}

#[tokio::test]
async fn test_performance_metrics_endpoint() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    println!(
        "Running test_performance_metrics_endpoint with ID: {}",
        test_id
    );

    let (task_queue, redis_url) = setup_test_task_queue().await;

    let app = configure_task_queue_routes().with_state(task_queue.clone());

    let request = Request::builder()
        .uri("/tasks/metrics/performance")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_body: Value = serde_json::from_slice(&body).unwrap();

    validate_json_response(
        &json_body,
        &[
            "uptime_seconds",
            "task_performance",
            "active_alerts",
            "sla_violations",
        ],
    );

    assert!(json_body["task_performance"].is_object());
    assert!(json_body["active_alerts"].is_array());
    assert!(json_body["sla_violations"].is_array());

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_performance_metrics_endpoint");
}

#[tokio::test]
async fn test_autoscaler_metrics_endpoint() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    println!(
        "Running test_autoscaler_metrics_endpoint with ID: {}",
        test_id
    );

    let (task_queue, redis_url) = setup_test_task_queue().await;

    let app = configure_task_queue_routes().with_state(task_queue.clone());

    let request = Request::builder()
        .uri("/tasks/metrics/autoscaler")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_body: Value = serde_json::from_slice(&body).unwrap();

    validate_json_response(&json_body, &["metrics", "timestamp"]);

    // Validate autoscaler metrics structure
    let metrics = &json_body["metrics"];
    // The autoscaler metrics might return a tuple, so let's check the structure more carefully
    let autoscaler_data = if metrics.is_array() {
        // If it's returned as a tuple result from try_join!, use the first element
        &metrics[0]
    } else {
        // If it's returned as a direct object, use it directly
        metrics
    };
    
    assert!(autoscaler_data["active_workers"].is_number());
    assert!(autoscaler_data["total_pending_tasks"].is_number());
    assert!(autoscaler_data["queue_metrics"].is_array());
    assert!(autoscaler_data["queue_pressure_score"].is_number());
    assert!(autoscaler_data["worker_utilization"].is_number());

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_autoscaler_metrics_endpoint");
}

#[tokio::test]
async fn test_queue_metrics_endpoint() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    println!("Running test_queue_metrics_endpoint with ID: {}", test_id);

    let (task_queue, redis_url) = setup_test_task_queue().await;

    let app = configure_task_queue_routes().with_state(task_queue.clone());

    let request = Request::builder()
        .uri("/tasks/metrics/queues")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_body: Value = serde_json::from_slice(&body).unwrap();

    validate_json_response(&json_body, &["queue_metrics", "errors", "timestamp"]);

    let queue_metrics = json_body["queue_metrics"].as_array().unwrap();
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

    let app = configure_task_queue_routes().with_state(task_queue.clone());

    let request = Request::builder()
        .uri("/tasks/metrics/workers")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_body: Value = serde_json::from_slice(&body).unwrap();

    validate_json_response(
        &json_body,
        &[
            "active_workers",
            "worker_metrics",
            "is_shutting_down",
            "timestamp",
        ],
    );

    assert!(json_body["active_workers"].is_number());
    assert!(json_body["worker_metrics"].is_object());
    assert!(json_body["is_shutting_down"].is_boolean());

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_worker_metrics_endpoint");
}

#[tokio::test]
async fn test_registry_endpoint() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    println!("Running test_registry_endpoint with ID: {}", test_id);

    let (task_queue, redis_url) = setup_test_task_queue().await;

    let app = configure_task_queue_routes().with_state(task_queue.clone());

    let request = Request::builder()
        .uri("/tasks/registry")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json_body: Value = serde_json::from_slice(&body).unwrap();

    validate_json_response(&json_body, &["registry_type", "features", "timestamp"]);

    // Check registry type is either "auto_registered" or "manual"
    let registry_type = json_body["registry_type"].as_str().unwrap();
    assert!(
        registry_type == "auto_registered" || registry_type == "manual",
        "Unexpected registry type: {}",
        registry_type
    );

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_registry_endpoint");
}

#[tokio::test]
async fn test_concurrent_endpoint_requests() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    println!(
        "Running test_concurrent_endpoint_requests with ID: {}",
        test_id
    );

    let (task_queue, redis_url) = setup_test_task_queue().await;

    let app = configure_task_queue_routes().with_state(task_queue.clone());

    // Test concurrent requests to different endpoints
    let endpoints = vec![
        "/tasks/health",
        "/tasks/status", 
        "/tasks/metrics/system",
        "/tasks/metrics/workers",
        "/tasks/registry",
    ];

    let mut handles = Vec::new();

    for endpoint in endpoints {
        let app_clone = app.clone();
        let endpoint_owned = endpoint.to_string();
        
        let handle = tokio::spawn(async move {
            let request = Request::builder()
                .uri(&endpoint_owned)
                .body(Body::empty())
                .unwrap();

            let response = app_clone.oneshot(request).await.unwrap();
            (endpoint_owned, response.status())
        });
        
        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        let (endpoint, status) = handle.await.unwrap();
        assert_eq!(
            status,
            StatusCode::OK,
            "Endpoint {} failed with status: {}",
            endpoint,
            status
        );
    }

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_concurrent_endpoint_requests");
}

#[tokio::test]
async fn test_error_handling_for_invalid_endpoints() {
    let test_id = Uuid::new_v4().to_string()[..8].to_string();
    println!(
        "Running test_error_handling_for_invalid_endpoints with ID: {}",
        test_id
    );

    let (task_queue, redis_url) = setup_test_task_queue().await;

    let app = configure_task_queue_routes().with_state(task_queue.clone());

    // Test invalid endpoint
    let request = Request::builder()
        .uri("/tasks/nonexistent")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    cleanup_task_queue(&task_queue, &redis_url).await;
    println!("Completed test_error_handling_for_invalid_endpoints");
} 