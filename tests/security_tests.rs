use rust_task_queue::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::time::{sleep, Duration};

// Global counter for unique database numbers
static DB_COUNTER: AtomicU32 = AtomicU32::new(1);

fn setup_isolated_redis_url() -> String {
    let db_num = DB_COUNTER.fetch_add(1, Ordering::SeqCst) % 16; // Redis has databases 0-15
    let base_url = std::env::var("REDIS_TEST_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
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

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SecurityTestTask {
    payload: String,
    size_mb: usize,
}

#[async_trait::async_trait]
impl Task for SecurityTestTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        // Simulate processing with potential security implications
        #[derive(Serialize)]
        struct Response {
            processed_length: usize,
            status: String,
        }
        
        let response = Response {
            processed_length: self.payload.len(),
            status: "completed".to_string(),
        };
        
        Ok(rmp_serde::to_vec(&response)?)
    }

    fn name(&self) -> &str {
        "security_test_task"
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct MaliciousTask {
    script_content: String,
    command: String,
}

#[async_trait::async_trait]
impl Task for MaliciousTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        // This task should NOT execute arbitrary commands or scripts
        // It should safely handle and sanitize any input
        
        // Safe processing - just return information about the input
        #[derive(Serialize)]
        struct Response {
            input_length: usize,
            contains_script: bool,
            contains_command: bool,
            status: String,
        }
        
        let response = Response {
            input_length: self.script_content.len() + self.command.len(),
            contains_script: !self.script_content.is_empty(),
            contains_command: !self.command.is_empty(),
            status: "safely_processed".to_string(),
        };
        
        Ok(rmp_serde::to_vec(&response)?)
    }

    fn name(&self) -> &str {
        "malicious_task"
    }
}

#[tokio::test]
async fn test_oversized_task_handling() {
    let redis_url = setup_isolated_redis_url();
    cleanup_test_database(&redis_url).await;
    
    let task_queue = TaskQueue::new(&redis_url).await.expect("Failed to create task queue");
    let registry = Arc::new(TaskRegistry::new());
    
    registry.register_with_name::<SecurityTestTask>("security_test_task").expect("Failed to register task");
    
    task_queue.start_workers_with_registry(1, registry).await.expect("Failed to start workers");
    
    // Test with extremely large payload
    let large_payload = "A".repeat(10 * 1024 * 1024); // 10MB string
    let oversized_task = SecurityTestTask {
        payload: large_payload,
        size_mb: 10,
    };
    
    // This should either be rejected or handled gracefully
    let result = task_queue.enqueue(oversized_task, "default").await;
    
    match result {
        Ok(_) => {
            // If accepted, ensure it's processed without crashing the system
            sleep(Duration::from_millis(5000)).await;
            
            let metrics = task_queue.broker.get_queue_metrics("default").await.expect("Failed to get metrics");
            // Task should either be processed or failed, but system should remain stable
            assert!(metrics.processed_tasks + metrics.failed_tasks > 0, "Task should be processed or failed");
        }
        Err(e) => {
            // Rejection is acceptable for oversized tasks
            println!("Oversized task rejected as expected: {}", e);
        }
    }
    
    // Ensure system is still responsive with normal tasks
    let normal_task = SecurityTestTask {
        payload: "normal payload".to_string(),
        size_mb: 0,
    };
    
    let normal_result = task_queue.enqueue(normal_task, "default").await;
    assert!(normal_result.is_ok(), "Normal task should be accepted after oversized task");
    
    // Wait for normal task to process
    sleep(Duration::from_millis(1000)).await;
    
    // Cleanup
    task_queue.shutdown().await.expect("Failed to shutdown task queue");
    cleanup_test_database(&redis_url).await;
}

#[tokio::test]
async fn test_malicious_payload_safety() {
    let redis_url = setup_isolated_redis_url();
    cleanup_test_database(&redis_url).await;
    
    let task_queue = TaskQueue::new(&redis_url).await.expect("Failed to create task queue");
    let registry = Arc::new(TaskRegistry::new());
    
    registry.register_with_name::<MaliciousTask>("malicious_task").expect("Failed to register task");
    
    task_queue.start_workers_with_registry(1, registry).await.expect("Failed to start workers");
    
    // Test various potentially malicious payloads
    let malicious_payloads = vec![
        // Script injection attempts
        MaliciousTask {
            script_content: "<script>alert('xss')</script>".to_string(),
            command: "".to_string(),
        },
        // Command injection attempts
        MaliciousTask {
            script_content: "".to_string(),
            command: "rm -rf /; cat /etc/passwd".to_string(),
        },
        // SQL injection patterns
        MaliciousTask {
            script_content: "'; DROP TABLE users; --".to_string(),
            command: "1' OR '1'='1".to_string(),
        },
        // Buffer overflow attempts
        MaliciousTask {
            script_content: "A".repeat(100000),
            command: "B".repeat(100000),
        },
        // Null bytes and control characters
        MaliciousTask {
            script_content: "test\0null\x01\x02\x03".to_string(),
            command: "\r\n\t\x7f".to_string(),
        },
    ];
    
    for (i, malicious_task) in malicious_payloads.into_iter().enumerate() {
        println!("Testing malicious payload {}", i + 1);
        
        let result = task_queue.enqueue(malicious_task, "default").await;
        assert!(result.is_ok(), "Malicious task {} should be accepted but processed safely", i + 1);
        
        // Small delay between tasks
        sleep(Duration::from_millis(100)).await;
    }
    
    // Wait for all tasks to be processed
    let mut attempts = 0;
    loop {
        let metrics = task_queue.broker.get_queue_metrics("default").await.expect("Failed to get metrics");
        let total_handled = metrics.processed_tasks + metrics.failed_tasks;
        
        if total_handled >= 5 {
            break;
        }
        
        if attempts >= 100 {
            panic!("Malicious tasks did not complete processing. Handled: {}/5", total_handled);
        }
        
        sleep(Duration::from_millis(100)).await;
        attempts += 1;
    }
    
    // Verify system is still stable and responsive
    let test_task = MaliciousTask {
        script_content: "normal content".to_string(),
        command: "normal command".to_string(),
    };
    
    let final_result = task_queue.enqueue(test_task, "default").await;
    assert!(final_result.is_ok(), "System should remain responsive after processing malicious payloads");
    
    // Cleanup
    task_queue.shutdown().await.expect("Failed to shutdown task queue");
    cleanup_test_database(&redis_url).await;
}

#[tokio::test]
async fn test_queue_name_injection_attacks() {
    let redis_url = setup_isolated_redis_url();
    cleanup_test_database(&redis_url).await;
    
    let task_queue = TaskQueue::new(&redis_url).await.expect("Failed to create task queue");
    let task = SecurityTestTask {
        payload: "test".to_string(),
        size_mb: 0,
    };
    
    // Test various queue name injection attempts
    let long_name = "A".repeat(10000);
    let malicious_queue_names = vec![
        // Redis command injection
        "queue\nFLUSHALL\nqueue",
        "queue;FLUSHDB;queue",
        // Path traversal
        "../../../etc/passwd",
        "queue\\..\\..\\windows\\system32",
        // Special Redis patterns
        "*",
        "queue:*:all",
        // Very long names
        long_name.as_str(),
        // Null bytes
        "queue\0injection",
        // Control characters
        "queue\r\nset evil value\r\nqueue",
    ];
    
    for malicious_name in malicious_queue_names {
        println!("Testing malicious queue name: {:?}", malicious_name);
        
        let result = task_queue.enqueue(task.clone(), malicious_name).await;
        
        // The system should either reject the malicious name or handle it safely
        match result {
            Ok(_) => {
                // If accepted, verify it doesn't affect system integrity
                println!("Malicious queue name accepted, checking system integrity...");
                
                // Try to enqueue to a normal queue to verify system is still working
                let normal_result = task_queue.enqueue(task.clone(), "normal_queue").await;
                assert!(normal_result.is_ok(), "System should still work after malicious queue name");
            }
            Err(e) => {
                // Rejection is acceptable and preferred for malicious names
                println!("Malicious queue name rejected: {}", e);
            }
        }
    }
    
    // Cleanup
    task_queue.shutdown().await.expect("Failed to shutdown task queue");
    cleanup_test_database(&redis_url).await;
}

#[tokio::test]
async fn test_task_deserialization_bomb() {
    let redis_url = setup_isolated_redis_url();
    cleanup_test_database(&redis_url).await;
    
    let task_queue = TaskQueue::new(&redis_url).await.expect("Failed to create task queue");
    let registry = Arc::new(TaskRegistry::new());
    
    registry.register_with_name::<SecurityTestTask>("security_test_task").expect("Failed to register task");
    
    task_queue.start_workers_with_registry(1, registry).await.expect("Failed to start workers");
    
    // Create a task with deeply nested structure that could cause stack overflow
    let nested_payload = (0..1000).map(|i| format!("level_{}_", i)).collect::<String>();
    
    let bomb_task = SecurityTestTask {
        payload: nested_payload,
        size_mb: 0,
    };
    
    let result = task_queue.enqueue(bomb_task, "default").await;
    
    match result {
        Ok(_) => {
            // Wait for processing with timeout
            let mut attempts = 0;
            while attempts < 50 {
                let metrics = task_queue.broker.get_queue_metrics("default").await.expect("Failed to get metrics");
                if metrics.processed_tasks + metrics.failed_tasks > 0 {
                    break;
                }
                sleep(Duration::from_millis(100)).await;
                attempts += 1;
            }
            
            // Verify system is still responsive
            let normal_task = SecurityTestTask {
                payload: "normal".to_string(),
                size_mb: 0,
            };
            
            let normal_result = task_queue.enqueue(normal_task, "default").await;
            assert!(normal_result.is_ok(), "System should remain responsive after deserialization bomb");
        }
        Err(e) => {
            println!("Deserialization bomb rejected: {}", e);
        }
    }
    
    // Cleanup
    task_queue.shutdown().await.expect("Failed to shutdown task queue");
    cleanup_test_database(&redis_url).await;
}

#[tokio::test]
async fn test_redis_connection_string_injection() {
    // Test various malicious Redis connection strings
    // Note: Some Redis URLs that might seem malicious are actually valid Redis URLs
    // The Redis client library handles URL parsing, so we test the most obviously invalid ones
    let long_url = format!("redis://localhost:6379/{}", "A".repeat(10000));
    let malicious_urls = vec![
        // Invalid protocols (these should definitely be rejected)
        "file:///etc/passwd",
        "http://evil.com/steal-data", 
        "javascript:alert('xss')",
        // Extremely long URL
        long_url.as_str(),
        // Malformed URLs
        "not_a_url",
        "",
        "redis://",
        "redis://localhost:-1",
    ];
    
    for malicious_url in malicious_urls {
        println!("Testing malicious Redis URL: {:?}", malicious_url);
        
        let result = TaskQueue::new(malicious_url).await;
        
        // All malicious URLs should be rejected
        assert!(result.is_err(), "Malicious Redis URL should be rejected: {}", malicious_url);
        
        if let Err(e) = result {
            println!("Malicious URL rejected with error: {}", e);
            // Error should indicate configuration or connection failure
            assert!(
                e.to_string().contains("Connection") || 
                e.to_string().contains("Configuration") ||
                e.to_string().contains("Failed"),
                "Error should indicate proper validation: {}", e
            );
        }
    }
}

#[tokio::test]
async fn test_configuration_tampering() {
    // Test configuration validation with invalid values
    let invalid_configs = vec![
        // Negative values
        AutoScalerConfig {
            min_workers: 0,
            max_workers: 10,
            scale_up_threshold: 5.0,
            scale_down_threshold: 1.0,
            scale_up_count: 2,
            scale_down_count: 1,
        },
        // Impossible ratios
        AutoScalerConfig {
            min_workers: 10,
            max_workers: 5, // max < min
            scale_up_threshold: 5.0,
            scale_down_threshold: 1.0,
            scale_up_count: 2,
            scale_down_count: 1,
        },
        // Extreme values
        AutoScalerConfig {
            min_workers: 1,
            max_workers: 999999, // Unreasonably high
            scale_up_threshold: 5.0,
            scale_down_threshold: 1.0,
            scale_up_count: 2,
            scale_down_count: 1,
        },
        // Invalid thresholds
        AutoScalerConfig {
            min_workers: 1,
            max_workers: 5,
            scale_up_threshold: -1.0, // Negative threshold
            scale_down_threshold: 1.0,
            scale_up_count: 2,
            scale_down_count: 1,
        },
        // Scale counts that could cause DoS
        AutoScalerConfig {
            min_workers: 1,
            max_workers: 5,
            scale_up_threshold: 5.0,
            scale_down_threshold: 1.0,
            scale_up_count: 999999, // Extreme scale up
            scale_down_count: 1,
        },
    ];
    
    for (i, config) in invalid_configs.into_iter().enumerate() {
        println!("Testing invalid config {}", i + 1);
        
        let validation_result = config.validate();
        assert!(validation_result.is_err(), "Invalid config {} should be rejected", i + 1);
        
        if let Err(e) = validation_result {
            println!("Invalid config rejected: {}", e);
        }
    }
}

#[tokio::test]
async fn test_concurrent_access_safety() {
    let redis_url = setup_isolated_redis_url();
    cleanup_test_database(&redis_url).await;
    
    let task_queue = Arc::new(TaskQueue::new(&redis_url).await.expect("Failed to create task queue"));
    let registry = Arc::new(TaskRegistry::new());
    
    registry.register_with_name::<SecurityTestTask>("security_test_task").expect("Failed to register task");
    
    task_queue.start_workers_with_registry(4, registry).await.expect("Failed to start workers");
    
    // Spawn multiple concurrent tasks that try to access the same resources
    let mut handles = Vec::new();
    
    for i in 0..10 {
        let task_queue_clone = task_queue.clone();
        let handle = tokio::spawn(async move {
            for j in 0..10 {
                let task = SecurityTestTask {
                    payload: format!("concurrent_task_{}_{}", i, j),
                    size_mb: 0,
                };
                
                let result = task_queue_clone.enqueue(task, "default").await;
                assert!(result.is_ok(), "Concurrent task should be enqueued successfully");
                
                // Small random delay to increase contention
                sleep(Duration::from_millis((i * j) % 50)).await;
            }
        });
        handles.push(handle);
    }
    
    // Wait for all concurrent operations to complete
    for handle in handles {
        handle.await.expect("Concurrent task should complete");
    }
    
    // Wait for all tasks to be processed
    let mut attempts = 0;
    loop {
        let metrics = task_queue.broker.get_queue_metrics("default").await.expect("Failed to get metrics");
        if metrics.processed_tasks >= 100 {
            break;
        }
        
        if attempts >= 200 {
            panic!("Concurrent tasks did not complete. Processed: {}/100", metrics.processed_tasks);
        }
        
        sleep(Duration::from_millis(100)).await;
        attempts += 1;
    }
    
    // Verify system integrity
    let final_metrics = task_queue.broker.get_queue_metrics("default").await.expect("Failed to get final metrics");
    assert_eq!(final_metrics.processed_tasks, 100, "All concurrent tasks should be processed exactly once");
    
    // Cleanup
    task_queue.shutdown().await.expect("Failed to shutdown task queue");
    cleanup_test_database(&redis_url).await;
}

#[tokio::test]
async fn test_redis_injection_prevention() {
    let redis_url = setup_isolated_redis_url();
    cleanup_test_database(&redis_url).await;
    
    let task_queue = TaskQueue::new(&redis_url).await.expect("Failed to create task queue");
    
    // Test various Redis injection attempts
    let malicious_queue_names = vec![
        "eval_malicious_code",
        "script_load",
        "flushall_attack",
        "config_set_dangerous",
        "queue; FLUSHDB",
        "queue\nDEL *",
        "queue\rSHUTDOWN",
        "../../../etc/passwd",
        "queue`rm -rf /`",
        "queue$(cat /etc/passwd)",
        "queue' OR 1=1 --",
    ];
    
    let test_task = SecurityTestTask {
        payload: "test_payload".to_string(),
        size_mb: 0,
    };
    
    for malicious_queue in malicious_queue_names {
        let result = task_queue.enqueue(test_task.clone(), malicious_queue).await;
        
        // Should fail due to input validation
        assert!(result.is_err(), "Queue name '{}' should be rejected", malicious_queue);
        
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("Queue name contains invalid characters") || 
            error_msg.contains("potentially dangerous pattern"),
            "Error should indicate security violation for '{}': {}", malicious_queue, error_msg
        );
    }
    
    // Test valid queue names should work
    let valid_queue_names = vec![
        "default",
        "high-priority",
        "worker_queue",
        "queue:namespace:subqueue",
        "queue123",
    ];
    
    for valid_queue in valid_queue_names {
        let result = task_queue.enqueue(test_task.clone(), valid_queue).await;
        assert!(result.is_ok(), "Valid queue name '{}' should be accepted", valid_queue);
    }
    
    cleanup_test_database(&redis_url).await;
}

#[tokio::test]
async fn test_payload_size_limits() {
    let redis_url = setup_isolated_redis_url();
    cleanup_test_database(&redis_url).await;
    
    let task_queue = TaskQueue::new(&redis_url).await.expect("Failed to create task queue");
    
    // Test oversized payload
    let huge_payload = "x".repeat(17 * 1024 * 1024); // 17MB > 16MB limit
    let large_task = SecurityTestTask {
        payload: huge_payload,
        size_mb: 0,
    };
    
    let result = task_queue.enqueue(large_task, "default").await;
    assert!(result.is_err(), "Oversized payload should be rejected");
    
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Task payload too large"), "Error should indicate payload size limit: {}", error_msg);
    
    // Test normal-sized payload
    let normal_task = SecurityTestTask {
        payload: "normal_payload".to_string(),
        size_mb: 0,
    };
    
    let result = task_queue.enqueue(normal_task, "default").await;
    assert!(result.is_ok(), "Normal payload should be accepted");
    
    cleanup_test_database(&redis_url).await;
}

#[tokio::test]
async fn test_task_name_validation() {
    use rust_task_queue::TaskWrapper;
    use chrono::Utc;
    
    let redis_url = setup_isolated_redis_url();
    cleanup_test_database(&redis_url).await;
    
    let task_queue = TaskQueue::new(&redis_url).await.expect("Failed to create task queue");
    
    // Test empty task name
    let task_wrapper = TaskWrapper {
        metadata: rust_task_queue::TaskMetadata {
            id: uuid::Uuid::new_v4(),
            name: "".to_string(), // Empty name
            created_at: Utc::now(),
            attempts: 0,
            max_retries: 3,
            timeout_seconds: 300,
        },
        payload: b"test".to_vec(),
    };
    
    let result = task_queue.broker.enqueue_task_wrapper(task_wrapper, "default").await;
    assert!(result.is_err(), "Empty task name should be rejected");
    
    // Test overly long task name
    let long_name = "x".repeat(300); // > 255 character limit
    let task_wrapper = TaskWrapper {
        metadata: rust_task_queue::TaskMetadata {
            id: uuid::Uuid::new_v4(),
            name: long_name,
            created_at: Utc::now(),
            attempts: 0,
            max_retries: 3,
            timeout_seconds: 300,
        },
        payload: b"test".to_vec(),
    };
    
    let result = task_queue.broker.enqueue_task_wrapper(task_wrapper, "default").await;
    assert!(result.is_err(), "Overly long task name should be rejected");
    
    cleanup_test_database(&redis_url).await;
}

#[tokio::test]
async fn test_comprehensive_input_sanitization() {
    let redis_url = setup_isolated_redis_url();
    cleanup_test_database(&redis_url).await;
    
    let task_queue = TaskQueue::new(&redis_url).await.expect("Failed to create task queue");
    
    // Test various injection patterns
    let long_name = "a".repeat(300);
    let test_cases = vec![
        ("", false, "Empty queue name"),
        (long_name.as_str(), false, "Too long queue name"),
        ("queue\0null", false, "Null byte injection"),
        ("queue\x1b[31mcolored", false, "ANSI escape injection"),
        ("queue\u{202e}rtl", false, "Unicode direction override"),
        ("valid_queue", true, "Valid queue name"),
        ("queue-123", true, "Valid with dash and numbers"),
        ("namespace:queue", true, "Valid with colon"),
    ];
    
    let test_task = SecurityTestTask {
        payload: "test".to_string(),
        size_mb: 0,
    };
    
    for (queue_name, should_succeed, description) in test_cases {
        let result = task_queue.enqueue(test_task.clone(), queue_name).await;
        
        if should_succeed {
            assert!(result.is_ok(), "{}: should succeed but got {:?}", description, result);
        } else {
            assert!(result.is_err(), "{}: should fail but succeeded", description);
        }
    }
    
    cleanup_test_database(&redis_url).await;
} 