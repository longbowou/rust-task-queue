//! Environment-Only Task Worker
//!
//! This worker uses only environment variables for configuration,
//! skipping file-based configuration entirely.
//!
//! Set these environment variables:
//! - REDIS_URL=redis://localhost:6379
//! - TASK_QUEUE_WORKERS=3
//! - TASK_QUEUE_AUTO_REGISTER=true
//! - TASK_QUEUE_SCHEDULER=true

use rust_task_queue::cli::start_worker_from_env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Task Worker with Environment Variables Only");
    println!("Make sure to set: REDIS_URL, TASK_QUEUE_WORKERS, etc.");

    // This only uses environment variables, no config files
    start_worker_from_env().await
}
