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
