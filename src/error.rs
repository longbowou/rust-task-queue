use thiserror::Error;

#[derive(Error, Debug)]
pub enum TaskQueueError {
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Connection pool error: {0}")]
    Pool(#[from] deadpool_redis::PoolError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] rmp_serde::encode::Error),

    #[error("Deserialization error: {0}")]
    Deserialization(#[from] rmp_serde::decode::Error),

    #[error("Task execution error: {0}")]
    TaskExecution(String),

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Task timeout: task {id} exceeded {timeout_seconds}s")]
    TaskTimeout { id: String, timeout_seconds: u64 },

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Worker error: {0}")]
    Worker(String),

    #[error("Scheduler error: {0}")]
    Scheduler(String),

    #[error("Queue error: {0}")]
    Queue(String),

    #[error("Broker error: {0}")]
    Broker(String),

    #[error("Scheduling error: {0}")]
    Scheduling(String),

    #[error("Auto-scaling error: {0}")]
    AutoScaling(String),

    #[error("Registry error: {0}")]
    Registry(String),
}

// Helper methods for creating common errors
impl TaskQueueError {
    pub fn task_not_found(task_name: &str) -> Self {
        Self::TaskNotFound(task_name.to_string())
    }

    pub fn task_timeout(task_id: &str, timeout_seconds: u64) -> Self {
        Self::TaskTimeout {
            id: task_id.to_string(),
            timeout_seconds,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_not_found_helper() {
        let error = TaskQueueError::task_not_found("my_task");
        match &error {
            TaskQueueError::TaskNotFound(name) => assert_eq!(name, "my_task"),
            _ => panic!("Expected TaskNotFound error"),
        }
        assert_eq!(error.to_string(), "Task not found: my_task");
    }

    #[test]
    fn test_task_timeout_helper() {
        let error = TaskQueueError::task_timeout("task_123", 300);
        match &error {
            TaskQueueError::TaskTimeout {
                id,
                timeout_seconds,
            } => {
                assert_eq!(id, "task_123");
                assert_eq!(*timeout_seconds, 300);
            }
            _ => panic!("Expected TaskTimeout error"),
        }
        assert_eq!(
            error.to_string(),
            "Task timeout: task task_123 exceeded 300s"
        );
    }

    #[test]
    fn test_error_display_messages() {
        let test_cases = vec![
            (
                TaskQueueError::TaskExecution("Failed to process".to_string()),
                "Task execution error: Failed to process",
            ),
            (
                TaskQueueError::Connection("Redis down".to_string()),
                "Connection error: Redis down",
            ),
            (
                TaskQueueError::Configuration("Invalid config".to_string()),
                "Configuration error: Invalid config",
            ),
            (
                TaskQueueError::Worker("Worker crash".to_string()),
                "Worker error: Worker crash",
            ),
            (
                TaskQueueError::Scheduler("Schedule failed".to_string()),
                "Scheduler error: Schedule failed",
            ),
            (
                TaskQueueError::Queue("Queue full".to_string()),
                "Queue error: Queue full",
            ),
            (
                TaskQueueError::Broker("Broker error".to_string()),
                "Broker error: Broker error",
            ),
            (
                TaskQueueError::Scheduling("Schedule conflict".to_string()),
                "Scheduling error: Schedule conflict",
            ),
            (
                TaskQueueError::AutoScaling("Scale failed".to_string()),
                "Auto-scaling error: Scale failed",
            ),
            (
                TaskQueueError::Registry("Registry error".to_string()),
                "Registry error: Registry error",
            ),
        ];

        for (error, expected_message) in test_cases {
            assert_eq!(error.to_string(), expected_message);
        }
    }

    #[test]
    fn test_serialization_error_conversion() {
        // Test that TaskQueueError can be converted from serialization errors
        // Since it's hard to trigger a serialization error with MessagePack,
        // we'll test the error type and conversion structure

        // Test with a direct TaskQueueError::Serialization
        let custom_error = rmp_serde::encode::Error::Syntax("Test serialization error".to_string());
        let queue_error: TaskQueueError = custom_error.into();

        match queue_error {
            TaskQueueError::Serialization(_) => {} // Success
            _ => panic!("Expected Serialization error"),
        }

        assert!(queue_error.to_string().contains("Serialization error"));
    }

    #[test]
    fn test_deserialization_error_conversion() {
        // Create invalid msgpack data
        let invalid_data = vec![0xFF, 0xFF, 0xFF, 0xFF];
        let result: Result<String, rmp_serde::decode::Error> = rmp_serde::from_slice(&invalid_data);
        assert!(result.is_err());

        let deserialization_error = result.unwrap_err();
        let queue_error: TaskQueueError = deserialization_error.into();

        match queue_error {
            TaskQueueError::Deserialization(_) => {} // Success
            _ => panic!("Expected Deserialization error"),
        }
    }

    #[test]
    fn test_error_debug_formatting() {
        let error = TaskQueueError::TaskExecution("Test error".to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("TaskExecution"));
        assert!(debug_str.contains("Test error"));
    }

    #[test]
    fn test_error_send_sync_traits() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<TaskQueueError>();
        assert_sync::<TaskQueueError>();
    }

    #[test]
    fn test_error_source_chain() {
        use std::error::Error;

        let error = TaskQueueError::TaskExecution("Root cause".to_string());
        assert!(error.source().is_none());

        // Test that our error type properly implements the Error trait
        let _: &dyn Error = &error;
    }
}
