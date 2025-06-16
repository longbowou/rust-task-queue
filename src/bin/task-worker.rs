//! Auto-Configured Task Worker
//!
//! This worker automatically loads configuration from files or environment variables.
//! No manual setup needed!

use rust_task_queue::cli::start_worker;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Task Worker with Auto-Configuration");
    println!("Looking for task-queue.toml, task-queue.yaml, or environment variables...");

    // Use the simplified consumer helper which handles all the configuration automatically
    start_worker().await
}
