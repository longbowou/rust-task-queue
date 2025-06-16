use thiserror::Error;

#[derive(Error, Debug)]
pub enum TaskQueueError {
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] rmp_serde::encode::Error),
    
    #[error("Deserialization error: {0}")]
    Deserialization(#[from] rmp_serde::decode::Error),

    #[error("Task execution error: {0}")]
    TaskExecution(String),

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
}
