# Rust Task Queue Framework

[![Crates.io](https://img.shields.io/crates/v/rust-task-queue.svg)](https://crates.io/crates/rust-task-queue)
[![Documentation](https://docs.rs/rust-task-queue/badge.svg)](https://docs.rs/rust-task-queue)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/yourusername/rust-task-queue)

A high-performance, Redis-backed task queue framework with auto-scaling capabilities for async Rust applications.

## Features

- 🔄 **Redis-backed broker** for reliable message delivery
- 📈 **Auto-scaling workers** based on queue load
- ⏰ **Task scheduling** with delay support
- 🎯 **Multiple queue priorities**
- 🔁 **Retry logic** with configurable attempts
- ⏱️ **Task timeouts** and failure handling
- 📊 **Metrics and monitoring**
- 🌐 **Actix Web integration** (optional)
- 🛠️ **CLI tools** for standalone workers
- 🤖 **Automatic task registration** (optional)
- 🚀 **Separate worker processes** for better scalability

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
rust-task-queue = "0.1"
```

### Basic Usage

```rust
use rust_task_queue::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct MyTask {
    data: String,
}

#[async_trait]
impl Task for MyTask {
    async fn execute(&self) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        println!("Processing: {}", self.data);
        Ok(serde_json::json!({"status": "completed"}))
    }
    
    fn name(&self) -> &str {
        "my_task"
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create and configure task queue
    let task_queue = TaskQueueBuilder::new("redis://localhost:6379")
        .initial_workers(4)
        .with_scheduler()
        .with_autoscaler()
        .build()
        .await?;
    
    // Enqueue a task
    let task = MyTask { data: "Hello, World!".to_string() };
    let task_id = task_queue.enqueue(task, "default").await?;
    
    println!("Enqueued task: {}", task_id);
    Ok(())
}
```

### Automatic Task Registration

With the `auto-register` feature, tasks can be automatically discovered and registered:

```rust
use rust_task_queue::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default, AutoRegisterTask)]
struct MyTask {
    data: String,
}

#[async_trait]
impl Task for MyTask {
    async fn execute(&self) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        println!("Processing: {}", self.data);
        Ok(serde_json::json!({"status": "completed"}))
    }
    
    fn name(&self) -> &str {
        "my_task"
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Tasks with AutoRegisterTask are automatically discovered!
    let task_queue = TaskQueueBuilder::new("redis://localhost:6379")
        .auto_register_tasks()
        .initial_workers(4)
        .with_scheduler()
        .with_autoscaler()
        .build()
        .await?;
    
    // No manual registration needed!
    let task = MyTask { data: "Hello, World!".to_string() };
    let task_id = task_queue.enqueue(task, "default").await?;
    
    println!("Enqueued task: {}", task_id);
    Ok(())
}
```

You can also use the attribute macro for custom task names:

```rust
#[register_task("custom_name")]
#[derive(Debug, Serialize, Deserialize)]
struct MyTask {
    data: String,
}

impl Default for MyTask {
    fn default() -> Self {
        Self { data: String::new() }
    }
}
```

## CLI Worker Tool

The framework includes a powerful CLI tool for running workers in separate processes, providing better separation of concerns and scalability.

### Installation

Build the CLI binary:

```bash
cargo build --bin task-worker --features cli
```

Or install it globally:

```bash
cargo install --path . --features cli
```

### Basic CLI Usage

```bash
# Start workers with default settings (4 workers, no auto-scaling)
cargo run --bin task-worker --features cli worker

# Start workers with custom configuration
cargo run --bin task-worker --features cli,auto-register worker \
  --redis-url redis://localhost:6379 \
  --workers 8 \
  --enable-autoscaler \
  --enable-scheduler
```

### CLI Options

- `--redis-url, -r`: Redis connection URL (default: `redis://127.0.0.1:6379`)
- `--workers, -w`: Number of initial workers (default: `4`)
- `--enable-autoscaler, -a`: Enable auto-scaling based on queue load
- `--enable-scheduler, -s`: Enable task scheduler for delayed tasks
- `--queues, -q`: Comma-separated list of queue names to process
- `--worker-prefix`: Custom prefix for worker names

### Advanced CLI Examples

```bash
# Workers with auto-scaling and scheduling
cargo run --bin task-worker --features cli,auto-register worker \
  --workers 6 \
  --enable-autoscaler \
  --enable-scheduler

# Workers for specific queues only
cargo run --bin task-worker --features cli,auto-register worker \
  --workers 4 \
  --queues "emails,notifications,reports"

# Workers with custom naming
cargo run --bin task-worker --features cli,auto-register worker \
  --workers 2 \
  --worker-prefix "priority-worker" \
  --queues "high-priority"
```

## Integration Patterns

### Pattern 1: Actix Web with Separate Workers (Recommended)

This pattern provides the best separation of concerns and scalability:

**1. Web Application (web-only mode):**

```rust
use rust_task_queue::prelude::*;
use actix_web::{web, App, HttpServer, HttpResponse, Result as ActixResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default, AutoRegisterTask)]
struct ProcessOrderTask {
    order_id: String,
    customer_email: String,
    amount: f64,
}

#[async_trait]
impl Task for ProcessOrderTask {
    async fn execute(&self) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        println!("Processing order: {}", self.order_id);
        // Your processing logic here
        Ok(serde_json::json!({"status": "completed", "order_id": self.order_id}))
    }

    fn name(&self) -> &str {
        "process_order"
    }
}

async fn create_order(
    task_queue: web::Data<Arc<TaskQueue>>,
    order_data: web::Json<ProcessOrderTask>,
) -> ActixResult<HttpResponse> {
    match task_queue.enqueue(order_data.into_inner(), "orders").await {
        Ok(task_id) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "task_id": task_id,
            "status": "queued"
        }))),
        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "error": e.to_string()
        })))
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Create task queue WITHOUT workers (web-only mode)
    let task_queue = Arc::new(
        TaskQueueBuilder::new("redis://127.0.0.1:6379")
            .auto_register_tasks()
            // Notice: No .initial_workers() - workers run separately!
            .build()
            .await
            .expect("Failed to create task queue")
    );

    println!("🌐 Starting web server at http://localhost:3000");
    println!("💡 Start workers separately with:");
    println!("   cargo run --bin task-worker --features cli,auto-register worker --workers 4");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(task_queue.clone()))
            .route("/order", web::post().to(create_order))
            .configure(rust_task_queue::actix::configure_task_queue_routes)
    })
    .bind("0.0.0.0:3000")?
    .run()
    .await
}
```

**2. Start Workers in Separate Terminals:**

```bash
# Terminal 1: Start the web server
cargo run --example actix_with_separate_workers --features actix-integration,auto-register

# Terminal 2: General workers with auto-scaling
cargo run --bin task-worker --features cli,auto-register worker \
  --workers 4 \
  --enable-autoscaler

# Terminal 3: Specialized workers for specific queues
cargo run --bin task-worker --features cli,auto-register worker \
  --workers 2 \
  --queues "emails,notifications"
```

### Pattern 2: All-in-One Process

For simpler deployments, you can run everything in one process:

```rust
use rust_task_queue::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let task_queue = TaskQueueBuilder::new("redis://localhost:6379")
        .auto_register_tasks()
        .initial_workers(4)  // Workers start automatically
        .with_scheduler()
        .with_autoscaler()
        .build()
        .await?;
    
    // Your application logic here
    Ok(())
}
```

## Production Deployment

### Docker Compose Example

```yaml
version: '3.8'
services:
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
  
  web:
    build: .
    ports:
      - "3000:3000"
    depends_on:
      - redis
    environment:
      - REDIS_URL=redis://redis:6379
    command: cargo run --example actix_with_separate_workers --features actix-integration,auto-register
  
  workers:
    build: .
    depends_on:
      - redis
    environment:
      - REDIS_URL=redis://redis:6379
    command: cargo run --bin task-worker --features cli,auto-register worker --workers 4 --enable-autoscaler
    deploy:
      replicas: 3

  email-workers:
    build: .
    depends_on:
      - redis
    environment:
      - REDIS_URL=redis://redis:6379
    command: cargo run --bin task-worker --features cli,auto-register worker --workers 2 --queues emails
```

### Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: task-workers
spec:
  replicas: 3
  selector:
    matchLabels:
      app: task-workers
  template:
    metadata:
      labels:
        app: task-workers
    spec:
      containers:
      - name: worker
        image: your-app:latest
        command: ["cargo", "run", "--bin", "task-worker", "--features", "cli,auto-register", "worker", "--workers", "4", "--enable-autoscaler"]
        env:
        - name: REDIS_URL
          value: "redis://redis-service:6379"
```

## Monitoring and Observability

The framework provides built-in monitoring endpoints when using Actix integration:

```bash
# View registered tasks
curl http://localhost:3000/tasks/registered

# View queue statistics
curl http://localhost:3000/queues/stats

# Health check
curl http://localhost:3000/health
```

### Logging

Enable structured logging with tracing:

```rust
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    // Your application code here
    Ok(())
}
```

## Feature Flags

- `tracing`: Enable logging support (enabled by default)
- `actix-integration`: Enable Actix Web integration helpers
- `cli`: Enable CLI utilities for standalone workers
- `auto-register`: Enable automatic task registration using derive macros (enabled by default)
- `full`: Enable all features

## Installation

1. Install Redis:
   ```bash
   # macOS
   brew install redis && brew services start redis
   
   # Ubuntu/Debian
   sudo apt install redis-server && sudo systemctl start redis
   
   # Docker
   docker run -d -p 6379:6379 redis:7-alpine
   ```

2. Add to your project:
   ```toml
   [dependencies]
   rust-task-queue = { version = "0.1", features = ["full"] }
   ```

## Examples

- [Basic Usage](examples/basic_usage.rs) - Simple task queue setup
- [Task Registration](examples/task_registration.rs) - Manual task registration patterns
- [Auto Registration](examples/auto_registration.rs) - Automatic task discovery and registration
- [Actix Integration](examples/actix_integration.rs) - Web server integration with auto-registration
- [Simple Actix Auto](examples/simple_actix_auto.rs) - Minimal Actix Web + auto-registration example
- [Actix with Separate Workers](examples/actix_with_separate_workers.rs) - **Recommended pattern** for production
- [Advanced Configuration](examples/advanced_config.rs) - Custom configurations
- [Worker CLI](examples/worker_cli.rs) - CLI usage examples

## Best Practices

### Task Design
- Keep tasks idempotent when possible
- Use meaningful task names for monitoring
- Handle errors gracefully with proper logging
- Keep task payloads reasonably small

### Deployment
- Use separate worker processes for better isolation
- Scale workers based on queue metrics
- Monitor Redis memory usage and performance
- Set up proper logging and alerting

### Development
- Use auto-registration for rapid development
- Test with different worker configurations
- Monitor queue sizes during load testing
- Use the built-in monitoring endpoints

## Performance Tips

- **Queue Organization**: Use separate queues for different task types
- **Worker Scaling**: Start with auto-scaling enabled, then fine-tune
- **Redis Configuration**: Use Redis persistence and clustering for production
- **Monitoring**: Set up alerts on queue depth and worker health

## Troubleshooting

### Common Issues

1. **Workers not processing tasks**: Check Redis connectivity and task registration
2. **High memory usage**: Monitor task payload sizes and Redis memory
3. **Slow processing**: Consider increasing worker count or optimizing task logic
4. **Connection issues**: Verify Redis URL and network connectivity

### Debug Mode

Enable debug logging:

```bash
RUST_LOG=debug cargo run --bin task-worker --features cli worker
```

## Documentation

- [API Documentation](https://docs.rs/rust-task-queue)
- [Examples](examples/)

## License

Licensed under either of Apache License, Version 2.0 or MIT License at your option.