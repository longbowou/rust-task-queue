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

// Import tasks to ensure they're compiled into this binary
// This is ESSENTIAL for auto-registration to work with the inventory pattern
#[path = "../tasks.rs"]
mod tasks;

#[path = "../module_tasks/mod.rs"]
mod module_tasks;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Task Worker with Environment Variables Only");
    println!("Make sure to set: REDIS_URL, TASK_QUEUE_WORKERS, etc.");

    // Show what tasks are available from auto-registration
    if let Ok(registry) = rust_task_queue::TaskRegistry::with_auto_registered() {
        let registered_tasks = registry.registered_tasks();
        println!(
            "Found {} auto-registered task types:",
            registered_tasks.len()
        );
        for task_name in &registered_tasks {
            println!("   • {}", task_name);
        }
    }

    // This only uses environment variables, no config files
    start_worker_from_env().await
}
