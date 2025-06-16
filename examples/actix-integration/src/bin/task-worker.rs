//! Auto-Configured Task Worker
//!
//! This worker automatically loads configuration from files or environment variables.
//! No manual setup needed!

use rust_task_queue::cli::start_worker;

// Import tasks to ensure they're compiled into this binary
// This is ESSENTIAL for auto-registration to work with the inventory pattern
// Without this import, the AutoRegisterTask derive macros won't be executed
// and the tasks won't be submitted to the inventory for auto-discovery
#[path = "../tasks.rs"]
mod tasks;

#[path = "../module_tasks/mod.rs"]
mod module_tasks;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    start_worker().await
}
