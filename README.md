# Rust Task Queue

[![Crates.io](https://img.shields.io/crates/v/rust-task-queue.svg)](https://crates.io/crates/rust-task-queue)
[![Documentation](https://docs.rs/rust-task-queue/badge.svg)](https://docs.rs/rust-task-queue)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/yourusername/rust-task-queue)

A high-performance, Redis-backed task queue framework with auto-scaling capabilities for async Rust applications.

## Features

- 🔄 **Redis-backed broker** with connection pooling and optimized operations
- 📈 **Auto-scaling workers** based on queue load with configurable thresholds
- ⏰ **Task scheduling** with delay support and persistent scheduling
- 🎯 **Multiple queue priorities** with predefined queue constants
- 🔁 **Retry logic** with exponential backoff and configurable attempts
- ⏱️ **Task timeouts** and comprehensive failure handling
- 📊 **Metrics and monitoring** with health checks and performance tracking
- 🌐 **Actix Web integration** (optional) with built-in endpoints
- 🛠️ **CLI tools** for standalone workers with process separation
- 🤖 **Automatic task registration** with procedural macros
- 🚀 **Production-ready** with robust error handling and safety improvements
- ⚡ **High performance** with MessagePack serialization and connection pooling

## Quick Start

### Feature Selection

Choose the appropriate features for your use case:

```toml
[dependencies]
# Default features (recommended for most users)
rust-task-queue = "0.1"

# Or specify features explicitly:
rust-task-queue = { version = "0.1", features = ["default"] }

# Minimal installation (core functionality only)
rust-task-queue = { version = "0.1", default-features = false }

# Full installation (all features)
rust-task-queue = { version = "0.1", features = ["full"] }

# Custom feature combinations
rust-task-queue = { version = "0.1", features = ["tracing", "auto-register", "actix-integration"] }
```

### Available Features

- `default`: `tracing` + `auto-register` + `config-support` + `cli` (recommended)
- `full`: All features enabled for maximum functionality
- `tracing`: Structured logging with performance insights
- `actix-integration`: Web framework integration with built-in endpoints
- `cli`: Standalone worker binaries with configuration support
- `auto-register`: Automatic task discovery via procedural macros
- `config-support`: External TOML/YAML configuration files

### Feature Combinations for Common Use Cases

```toml
# Web application with separate workers (recommended)
rust-task-queue = { version = "0.1", features = ["tracing", "auto-register", "actix-integration", "config-support", "cli"] }

# Standalone worker processes
rust-task-queue = { version = "0.1", features = ["tracing", "auto-register", "cli", "config-support"] }

# Minimal embedded systems
rust-task-queue = { version = "0.1", default-features = false, features = ["tracing"] }

# Development/testing
rust-task-queue = { version = "0.1", features = ["full"] }

# Library integration (no CLI tools)
rust-task-queue = { version = "0.1", features = ["tracing", "auto-register", "config-support"] }
```

### Feature Examples

**Default Features (Recommended):**

```toml
[dependencies]
rust-task-queue = "0.1"  # Includes: tracing, auto-register, config-support, cli
```

✅ Enables: logging, automatic task discovery, configuration files, and CLI worker tools.

**Web Application:**

```toml
[dependencies]
rust-task-queue = { version = "0.1", features = ["tracing", "auto-register", "actix-integration", "config-support"] }
```

✅ Includes Actix Web routes (`/tasks/health`, `/tasks/stats`) and middleware.

**Minimal Setup:**

```toml
[dependencies]
rust-task-queue = { version = "0.1", default-features = false, features = ["tracing"] }
```

⚠️ Core functionality only - manual task registration required.

**Development/Full Features:**

```toml
[dependencies]
rust-task-queue = { version = "0.1", features = ["full"] }
```

✅ All features including experimental dynamic loading capabilities.

## Integration Patterns

### Pattern 1: Actix Web with Separate Workers (Recommended)

This pattern provides the best separation of concerns and scalability.

**💡 Recommended**: Use external configuration files (`task-queue.toml` or `task-queue.yaml`) for production deployments:

**1. Create worker configuration file at the root of your project (`task-queue.toml`):**

```toml
[redis]
url = "redis://127.0.0.1:6379"
pool_size = 10

[workers]
initial_count = 0  # No workers in web-only mode
max_concurrent_tasks = 10

[autoscaler]
min_workers = 1
max_workers = 20
scale_up_threshold = 5.0
scale_down_threshold = 1.0

[scheduler]
enabled = false  # Disabled in web-only mode

[auto_register]
enabled = true

[actix]
auto_configure_routes = true
route_prefix = "/tasks"
enable_metrics = true
enable_health_check = true
```

**2. Web Application (web-only mode):**

```rust
use rust_task_queue::prelude::*;
use rust_task_queue::queue::queue_names;
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
    match task_queue.enqueue(order_data.into_inner(), queue_names::DEFAULT).await {
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
    // Load configuration from file (recommended for production)
    let config = rust_task_queue::config::TaskQueueConfig::from_file("task-queue.toml")
        .expect("Failed to load configuration");

    // Create task queue using configuration
    let task_queue = Arc::new(
        TaskQueueBuilder::from_config(config)
            .auto_register_tasks()
            // Configuration file sets initial_count = 0 (no workers in web-only mode)
            .build()
            .await
            .expect("Failed to create task queue")
    );

    println!("🌐 Starting web server at http://localhost:3000");
    println!("💡 Start workers separately with configuration file:");
    println!("   cargo run --bin task-worker --features cli,auto-register worker --config task-queue.toml");

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

**4. Start Workers in Separate Terminals:**

```bash
# Terminal 1: Start the web server
cargo run --example actix-integration --features actix-integration,auto-register

# Terminal 2: General workers based on task-queue.toml 
cargo run --bin task-worker
  
# Terminal 2: General workers with auto-scaling (using config file)
cargo run --bin task-worker --features cli,auto-register worker \
  --config task-queue-worker.toml

# Alternative: Manual configuration (not recommended for production)
cargo run --bin task-worker --features cli,auto-register worker \
  --workers 4 \
  --enable-autoscaler

# Terminal 3: Specialized workers for specific queues
cargo run --bin task-worker --features cli,auto-register worker \
  --workers 2 \
  --queues "high_priority,default"
```

### Pattern 2: All-in-One Process

For simpler deployments, you can run everything in one process:

```rust
use rust_task_queue::prelude::*;
use rust_task_queue::queue::queue_names;

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

### Basic Usage

```rust
use rust_task_queue::prelude::*;
use rust_task_queue::queue::queue_names;
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

    // Enqueue a task using predefined queue constants
    let task = MyTask { data: "Hello, World!".to_string() };
    let task_id = task_queue.enqueue(task, queue_names::DEFAULT).await?;

    println!("Enqueued task: {}", task_id);
    Ok(())
}
```

### Queue Constants

The framework provides predefined queue constants for type safety and consistency:

```rust
use rust_task_queue::queue::queue_names;

// Available queue constants
queue_names::DEFAULT        // "default" - Standard priority tasks
queue_names::HIGH_PRIORITY  // "high_priority" - High priority tasks  
queue_names::LOW_PRIORITY   // "low_priority" - Background tasks
```

### Automatic Task Registration

With the `auto-register` feature, tasks can be automatically discovered and registered:

```rust
use rust_task_queue::prelude::*;
use rust_task_queue::queue::queue_names;
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
    let task_id = task_queue.enqueue(task, queue_names::DEFAULT).await?;

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

The framework includes a powerful CLI tool for running workers in separate processes, providing better separation of
concerns and scalability.

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
  --queues "high_priority,default"

# Workers with custom naming
cargo run --bin task-worker --features cli,auto-register worker \
  --workers 2 \
  --worker-prefix "priority-worker" \
  --queues "high_priority"
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
    volumes:
      - ./task-queue-web.toml:/app/task-queue.toml:ro
    command: cargo run --example actix-integration --features actix-integration,auto-register

  workers:
    build: .
    depends_on:
      - redis
    volumes:
      - ./task-queue-worker.toml:/app/task-queue.toml:ro
    command: cargo run --bin task-worker --features cli,auto-register worker --config task-queue.toml
    deploy:
      replicas: 3

  high-priority-workers:
    build: .
    depends_on:
      - redis
    volumes:
      - ./task-queue-priority.toml:/app/task-queue.toml:ro
    command: cargo run --bin task-worker --features cli,auto-register worker --config task-queue.toml
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
          command: [ "cargo", "run", "--bin", "task-worker", "--features", "cli,auto-register", "worker", "--workers", "4", "--enable-autoscaler" ]
          env:
            - name: REDIS_URL
              value: "redis://redis-service:6379"
```

## Configuration and Safety

### Configuration Files (Recommended for Production)

The framework supports both TOML and YAML configuration files for production deployments:

```rust
use rust_task_queue::config::TaskQueueConfig;

// Load from TOML file (recommended)
let config = TaskQueueConfig::from_file("task-queue.toml") ?;

// Load from YAML file  
let config = TaskQueueConfig::from_file("task-queue.yaml") ?;

// Use with TaskQueueBuilder
let task_queue = TaskQueueBuilder::from_config(config)
.auto_register_tasks()
.build()
.await?;
```

Example `task-queue.toml`:

```toml
[redis]
url = "redis://localhost:6379"
pool_size = 10

[workers]
initial_count = 4
max_concurrent_tasks = 10

[autoscaler]
min_workers = 1
max_workers = 20
scale_up_threshold = 5.0
scale_down_threshold = 1.0

[scheduler]
enabled = true

[auto_register]
enabled = true

[actix]
auto_configure_routes = true
route_prefix = "/tasks"
enable_metrics = true
enable_health_check = true
```

### Configuration Validation

The framework includes comprehensive configuration validation:

```rust
use rust_task_queue::config::TaskQueueConfig;

let config = TaskQueueConfig::default ();
// Configuration is automatically validated
config.validate() ?;
```

### Error Handling and Safety

Recent improvements include:

- **Robust error handling**: Eliminated unsafe `unwrap()` calls
- **Connection reliability**: Centralized Redis connection management
- **Graceful failures**: Proper task re-enqueueing on failures
- **Configuration validation**: Comprehensive config validation

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
   # Use default features for most applications
   rust-task-queue = "0.1"
   
   # Or choose specific features (see Feature Selection above)
   rust-task-queue = { version = "0.1", features = ["tracing", "auto-register", "actix-integration", "config-support"] }
   ```

## Examples

The repository includes comprehensive examples:

- [**Performance Test**](examples/performance_test.rs) - Benchmarking and performance testing
- [**Actix Integration**](examples/actix-integration/) - Complete web server integration examples
    - Full-featured task queue endpoints
    - Automatic task registration
    - Production-ready patterns

## Best Practices

### Task Design

- Keep tasks idempotent when possible
- Use meaningful task names for monitoring
- Handle errors gracefully with proper logging
- Keep task payloads reasonably small
- Use appropriate queue constants (`queue_names::*`)

### Deployment

- Use separate worker processes for better isolation
- Scale workers based on queue metrics
- Monitor Redis memory usage and performance
- Set up proper logging and alerting
- Use configuration validation in production

### Development

- Use auto-registration for rapid development
- Test with different worker configurations
- Monitor queue sizes during load testing
- Use the built-in monitoring endpoints
- Take advantage of the CLI tools for testing
- **Use configuration files** (`task-queue.toml`) instead of hardcoded values

## Performance Highlights

- **Task Serialization**: ~40ns per operation (MessagePack)
- **Task Deserialization**: ~34ns per operation
- **Queue Config Lookup**: ~40ns per operation
- **High throughput**: Handles thousands of tasks per second
- **Memory efficient**: Connection pooling and optimized serialization

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
- [Development Guide](DEVELOPMENT.md)
- [Configuration Guide](CONFIGURATION.md)
- [Examples](examples/)

## License

Licensed under either of Apache License, Version 2.0 or MIT License at your option.