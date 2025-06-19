# Rust Task Queue

[![Crates.io](https://img.shields.io/crates/v/rust-task-queue.svg)](https://crates.io/crates/rust-task-queue)
[![Documentation](https://docs.rs/rust-task-queue/badge.svg)](https://docs.rs/rust-task-queue)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/longbowou/rust-task-queue)
[![Downloads](https://img.shields.io/crates/d/rust-task-queue.svg)](https://crates.io/crates/rust-task-queue)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-blue.svg)](https://www.rust-lang.org)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()
[![Test Coverage](https://img.shields.io/badge/tests-220+%20passing-brightgreen.svg)]()

A high-performance, Redis-backed task queue framework with enhanced auto-scaling, intelligent async task spawning,
multidimensional scaling triggers, and advanced backpressure management for async Rust applications.

## Features

- **Redis-backed broker** with connection pooling and optimized operations
- **Enhanced Multi-dimensional Auto-scaling** with 5-metric analysis and adaptive learning
- **Task scheduling** with delay support and persistent scheduling
- **Multiple queue priorities** with predefined queue constants
- **Retry logic** with exponential backoff and configurable attempts
- **Task timeouts** and comprehensive failure handling
- **Advanced metrics and monitoring** with SLA tracking and performance insights
- **Production-grade observability** with comprehensive structured logging and tracing
- **Actix Web integration** (optional) with built-in endpoints
- **Axum integration** (optional) with built-in endpoints
- **CLI tools** for standalone workers with process separation and logging configuration
- **Automatic task registration** with procedural macros
- **High performance** with MessagePack serialization and connection pooling
- **Advanced async task spawning** with intelligent backpressure and resource management
- **Graceful shutdown** with active task tracking and cleanup
- **Smart resource allocation** with semaphore-based concurrency control
- **Comprehensive testing** with unit, integration, performance, and security tests
- **Enterprise-grade tracing** with lifecycle tracking, performance monitoring, and error context
- **Production-ready** with robust error handling and safety improvements

## Why Choose Rust Task Queue?

| Feature       | Rust Task Queue     | Celery    | Sidekiq   | Bull      |
|---------------|---------------------|-----------|-----------|-----------|
| Language      | Rust                | Python    | Ruby      | Node.js   |
| Auto-scaling  | Multi-dimensional   | ❌         | ❌         | ❌         |
| Performance   | <40ns serialization | ~ms       | ~ms       | ~ms       |
| Type Safety   | Compile-time        | ❌ Runtime | ❌ Runtime | ❌ Runtime |
| Memory Safety | Zero-copy           | ❌         | ❌         | ❌         |
| Async/Await   | Native              | ❌         | ❌         | ✅         |

### Performance Highlights

Recent benchmark results demonstrate exceptional performance:

| Operation                  | Time      | Status      |
|----------------------------|-----------|-------------|
| Task Serialization         | ~39.15 ns | Excellent   |
| Task Deserialization       | ~31.51 ns | Excellent   |
| Queue Config Lookup        | ~39.76 ns | Excellent   |
| Queue Management           | ~1.38 µs  | Very Good   |
| Enhanced AutoScaler Config | ~617 ps   | Outstanding |

*Benchmarks run on optimized release builds with statistical analysis*

## Production Ready

- **124+ comprehensive tests** (unit, integration, performance, security)
- **Memory safe** - no unsafe code
- **Performance benchmarked** - <40ns serialization
- **Enterprise logging** with structured tracing
- **Graceful shutdown** and error recovery
- **Redis cluster support** with connection pooling

## Quick Start

### Available Features

- `default`: `tracing` + `auto-register` + `config-support` + `cli` (recommended)
- `full`: All features enabled for maximum functionality
- `tracing`: enterprise-grade structured logging and observability
    - Complete task lifecycle tracking with distributed spans
    - Performance monitoring and execution timing
    - Error chain analysis with deep context
    - Worker activity and resource utilization monitoring
    - Production-ready logging configuration (JSON/compact/pretty formats)
    - Environment-based configuration support
- `actix-integration`: Actix Web framework integration with built-in endpoints
- `axum-integration`: Axum framework integration with comprehensive metrics and CORS support
- `cli`: Standalone worker binaries with logging configuration support
- `auto-register`: Automatic task discovery via procedural macros
- `config-support`: External TOML/YAML configuration files

### Feature Combinations for Common Use Cases

```toml
# Web application with Actix Web (recommended)
rust-task-queue = { version = "0.1", features = ["tracing", "auto-register", "actix-integration", "config-support", "cli"] }

# Web application with Axum framework
rust-task-queue = { version = "0.1", features = ["tracing", "auto-register", "axum-integration", "config-support", "cli"] }

# Standalone worker processes
rust-task-queue = { version = "0.1", features = ["tracing", "auto-register", "cli", "config-support"] }

# Minimal embedded systems
rust-task-queue = { version = "0.1", default-features = false, features = ["tracing"] }

# Development/testing
rust-task-queue = { version = "0.1", features = ["full"] }

# Library integration (no CLI tools)
rust-task-queue = { version = "0.1", features = ["tracing", "auto-register", "config-support"] }
```

## Integration Patterns

### Actix Web with Separate Workers (Recommended)

This pattern provides the best separation of concerns and scalability.

**Recommended**: Use external configuration files (`task-queue.toml` or `task-queue.yaml`) for production deployments:

**1. Create the configuration file:**
Create a configuration file **task-queue.toml** at the **root of your project**. Copy & past the content from this
template [`task-queue.toml`](task-queue.toml). The easy way to configure your worker is through
your configuration file. Adjust it according to your need. Use the default values if you don't know how to adjust it.

**2. Worker configuration:**

Copy [`task-worker.rs`](src/bin/task-worker.rs) to your **src/bin/** folder, and update it by importing your tasks to
ensure they are discoverable by the auto register.

```rust
use rust_task_queue::cli::start_worker;

// Import tasks to ensure they're compiled into this binary
// This is ESSENTIAL for auto-registration to work with the inventory pattern
// Without this import, the AutoRegisterTask derive macros won't be executed
// and the tasks won't be submitted to the inventory for auto-discovery

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Task Worker with Auto-Configuration");
    println!("Looking for task-queue.toml, task-queue.yaml, or environment variables...");

    // Use the simplified consumer helper which handles all the configuration automatically
    start_worker().await
}
```

Update your **Cargo.toml** to include the worker cli

```toml
# Bin declaration required to launch the worker.
[[bin]]
name = "task-worker"
path = "src/bin/task-worker.rs"
```

**3. Web Application (web-only mode with Actix):**

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
    // Create the task_queue automatically (based on your environment variables or your task-queue.toml)
    let task_queue = TaskQueueBuilder::auto().build().await?;

    println!("Starting web server at http://localhost:3000");
    println!("Start workers separately with configuration file:");
    println!("cargo run --bin task-worker");

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

Now you can start web server

```bash
# Start the web server
cargo run
```

**4. Start Workers in Separate Terminal:**

```bash
# Start workers based 
cargo run --bin task-worker
```

### Axum Web with Separate Workers

The same pattern works with Axum, providing a modern async web framework option:

**1. Create the worker configuration file same as the Actix example above**

**2. Do the worker configuration same as the Actix example above**

**3. Web Application (web-only mode using Axum):**

```rust
use rust_task_queue::prelude::*;
use rust_task_queue::queue::queue_names;
use axum::{extract::State, response::Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize, Default, AutoRegisterTask)]
struct ProcessOrderTask {
    order_id: String,
    customer_email: String,
    amount: f64,
}

#[async_trait]
impl Task for ProcessOrderTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        println!("Processing order: {}", self.order_id);
        // Your processing logic here
        let response = serde_json::json!({"status": "completed", "order_id": self.order_id});
        Ok(rmp_serde::to_vec(&response)?)
    }

    fn name(&self) -> &str {
        "process_order"
    }
}

async fn create_order(
    State(task_queue): State<Arc<TaskQueue>>,
    Json(order_data): Json<ProcessOrderTask>,
) -> Json<serde_json::Value> {
    match task_queue.enqueue(order_data, queue_names::DEFAULT).await {
        Ok(task_id) => Json(serde_json::json!({
            "task_id": task_id,
            "status": "queued"
        })),
        Err(e) => Json(serde_json::json!({
            "error": e.to_string()
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the task_queue automatically (based on your environment variables or your task-queue.toml)
    let task_queue = TaskQueueBuilder::auto().build().await?;

    println!("🌐 Starting Axum web server at http://localhost:3000");
    println!("💡 Start workers separately with: cargo run --bin task-worker");

    // Build our application with routes
    let app = Router::new()
        .route("/order", axum::routing::post(create_order))
        .merge(rust_task_queue::axum::configure_task_queue_routes())
        .with_state(task_queue);

    // Run the server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

Now you can start web server

```bash
# Start the web server
cargo run
```

**2. Use the same worker commands as the Actix example above**

### Examples

The repository includes comprehensive examples:

- [**Actix Integration**](examples/actix-integration/) - Complete Actix Web integration examples
    - Full-featured task queue endpoints
    - Automatic task registration
    - Production-ready patterns
- [**Axum Integration**](examples/axum-integration/) - Complete Axum framework integration examples
    - Full-featured task queue endpoints
    - Automatic task registration
    - Production-ready patterns

### All-in-One Process

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
    // Create and configure task queue with enhanced auto-scaling
    let task_queue = TaskQueueBuilder::new("redis://localhost:6379")
        .initial_workers(4)
        .with_scheduler()
        .with_autoscaler()  // Uses enhanced multi-dimensional auto-scaling
        .build()
        .await?;

    // Enqueue a task using predefined queue constants
    let task = MyTask { data: "Hello, World!".to_string() };
    let task_id = task_queue.enqueue(task, queue_names::DEFAULT).await?;

    println!("Enqueued task: {}", task_id);
    Ok(())
}
```

### Enhanced Auto-scaling with Manual Configuration

For programmatic control, use the enhanced configuration API:

```rust
use rust_task_queue::prelude::*;
use rust_task_queue::{ScalingTriggers, SLATargets, AutoScalerConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Enhanced auto-scaling configuration
    let autoscaler_config = AutoScalerConfig {
        min_workers: 2,
        max_workers: 50,
        scale_up_count: 3,
        scale_down_count: 1,

        // Multi-dimensional scaling triggers
        scaling_triggers: ScalingTriggers {
            queue_pressure_threshold: 1.5,
            worker_utilization_threshold: 0.80,
            task_complexity_threshold: 2.0,
            error_rate_threshold: 0.05,
            memory_pressure_threshold: 512.0,
        },

        // Adaptive learning settings
        enable_adaptive_thresholds: true,
        learning_rate: 0.1,
        adaptation_window_minutes: 30,

        // Stability controls
        scale_up_cooldown_seconds: 120,
        scale_down_cooldown_seconds: 600,
        consecutive_signals_required: 2,

        // SLA targets for optimization
        target_sla: SLATargets {
            max_p95_latency_ms: 5000.0,
            min_success_rate: 0.95,
            max_queue_wait_time_ms: 10000.0,
            target_worker_utilization: 0.70,
        },
    };

    let task_queue = TaskQueueBuilder::new("redis://localhost:6379")
        .auto_register_tasks()
        .initial_workers(4)
        .with_scheduler()
        .with_autoscaler_config(autoscaler_config)
        .build()
        .await?;

    // Monitor enhanced auto-scaling
    let recommendations = task_queue.autoscaler()
        .get_scaling_recommendations()
        .await?;
    println!("Auto-scaling recommendations:\n{}", recommendations);

    Ok(())
}
```

### Queue Constants

The framework provides predefined queue constants for type safety and consistency:

```rust
use rust_task_queue::queue::queue_names;

// Available queue constants
queue_names::DEFAULT       // "default" - Standard priority tasks
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

The framework includes a powerful CLI tool for running workers in separate processes, now with enhanced auto-scaling
support.

### Enhanced CLI Usage with Auto-scaling

```bash
# Start workers based on your root task-queue.toml configuration file (recommended)
cargo run --bin task-worker

# Workers with custom auto-scaling thresholds
cargo run --bin task-worker -- \
  --workers 4 \
  --enable-autoscaler \
  --autoscaler-min-workers 2 \
  --autoscaler-max-workers 20 \
  --autoscaler-scale-up-threshold 1.5 \
  --autoscaler-consecutive-signals 3

# Monitor auto-scaling in real-time
cargo run --bin task-worker -- \
  --workers 6 \
  --enable-autoscaler \
  --enable-scheduler \
  --log-level debug  # See detailed auto-scaling decisions
```

### CLI Options

- `--redis-url, -r`: Redis connection URL (default: `redis://127.0.0.1:6379`)
- `--workers, -w`: Number of initial workers (default: `4`)
- `--enable-autoscaler, -a`: Enable enhanced multi-dimensional auto-scaling
- `--enable-scheduler, -s`: Enable task scheduler for delayed tasks
- `--queues, -q`: Comma-separated list of queue names to process
- `--worker-prefix`: Custom prefix for worker names
- `--config, -c`: Path to enhanced configuration file (recommended)
- `--log-level`: Logging level (trace, debug, info, warn, error)
- `--log-format`: Log output format (json, compact, pretty)
- `--autoscaler-min-workers`: Minimum workers for auto-scaling
- `--autoscaler-max-workers`: Maximum workers for auto-scaling
- `--autoscaler-consecutive-signals`: Required consecutive signals for scaling

## Auto-scaling

### **Multi-dimensional Scaling Intelligence**

Our enhanced auto-scaling system analyzes **5 key metrics simultaneously** for intelligent scaling decisions:

- **Queue Pressure Score**: Weighted queue depth accounting for priority levels
- **Worker Utilization**: Real-time busy/idle ratio analysis
- **Task Complexity Factor**: Dynamic execution pattern recognition
- **Error Rate Monitoring**: System health and stability tracking
- **Memory Pressure**: Per-worker resource utilization analysis

### **Adaptive Threshold Learning**

The system automatically adjusts scaling triggers based on actual performance vs. your SLA targets:

```toml
[autoscaler.target_sla]
max_p95_latency_ms = 3000.0           # 3 second P95 latency target
min_success_rate = 0.99               # 99% success rate target
max_queue_wait_time_ms = 5000.0       # 5-second max queue wait
target_worker_utilization = 0.75      # optimal 75% worker utilization
```

### **Stability Controls**

Advanced hysteresis and cooldown mechanisms prevent scaling oscillations:

- **Consecutive Signal Requirements**: Configurable signal thresholds (2-5 signals)
- **Independent Cooldowns**: Separate scale-up (3 min) and scale-down (15 min) periods
- **Performance History**: Learning from past scaling decisions

## Test Coverage

The project maintains comprehensive test coverage across multiple dimensions:

- **Unit Tests**: 124 tests covering all core functionality
- **Integration Tests**: 23 tests for end-to-end workflows
- **Actix Integration Tests**: 22 tests (feature-specific)
- **Axum Integration Tests**: 17 tests for web endpoints and metrics API
- **Error Scenario Tests**: 18 tests for edge cases and failure modes
- **Performance Tests**: 20 tests for throughput and load handling
- **Security Tests**: 24 tests for injection attacks and safety
- **Benchmarks**: 7 performance benchmarks for optimization

**Total**: 248+ tests ensuring reliability and performance

## Enterprise-Grade Observability

The framework includes comprehensive structured logging and tracing capabilities for production systems:

### Tracing Features

- **Complete Task Lifecycle Tracking**: From enqueue to completion with detailed spans
- **Performance Monitoring**: Execution timing, queue metrics, and throughput analysis
- **Error Chain Analysis**: Deep context and source tracking for debugging
- **Worker Activity Monitoring**: Real-time status and resource utilization
- **Distributed Tracing**: Async instrumentation with span correlation
- **Production Logging Configuration**: Multiple output formats (JSON/compact/pretty)

### Logging Configuration

```rust
use rust_task_queue::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure structured logging for production
    configure_production_logging(
        rust_task_queue::LogLevel::Info,
        rust_task_queue::LogFormat::Json
    );

    let task_queue = TaskQueueBuilder::new("redis://localhost:6379")
        .auto_register_tasks()
        .initial_workers(4)
        .build()
        .await?;

    Ok(())
}
```

### Environment-Based Configuration

```bash
# Configure logging via environment variables
export LOG_LEVEL=info        # trace, debug, info, warn, error
export LOG_FORMAT=json       # json, compact, pretty

# Start worker with production logging
cargo run --bin task-worker
```

### Performance Monitoring

The tracing system provides detailed performance insights:

- **Task Execution Timing**: Individual task performance tracking
- **Queue Depth Monitoring**: Real-time queue status and trends
- **Worker Utilization**: Capacity analysis and efficiency metrics
- **Error Rate Tracking**: System health and failure analysis
- **Throughput Analysis**: Tasks per second and bottleneck identification

## Comprehensive Metrics API

The framework includes a production-ready metrics API with 15+ endpoints for monitoring and diagnostics:

### Health & Status Endpoints

- **`/tasks/health`** - Detailed health check with component status (Redis, workers, scheduler)
- **`/tasks/status`** - System status with health metrics and worker information

### Core Metrics Endpoints

- **`/tasks/metrics`** - Comprehensive metrics combining all available data
- **`/tasks/metrics/system`** - Enhanced system metrics with memory and performance data
- **`/tasks/metrics/performance`** - Performance report with task execution metrics and SLA data
- **`/tasks/metrics/autoscaler`** - AutoScaler metrics and scaling recommendations
- **`/tasks/metrics/queues`** - Individual queue metrics for all queues
- **`/tasks/metrics/workers`** - Worker-specific metrics and status
- **`/tasks/metrics/memory`** - Memory usage metrics and tracking
- **`/tasks/metrics/summary`** - Quick metrics summary for debugging

### Task Registry Endpoints

- **`/tasks/registered`** - Auto-registered tasks information
- **`/tasks/registry/info`** - Detailed task registry information and features

### Administrative Endpoints

- **`/tasks/alerts`** - Active alerts from the metrics system
- **`/tasks/sla`** - SLA status and violations with performance percentages
- **`/tasks/diagnostics`** - Comprehensive diagnostics with queue health analysis
- **`/tasks/uptime`** - System uptime and runtime information

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
- Configure appropriate `max_concurrent_tasks` based on workload characteristics
- Monitor active task counts to optimize worker capacity
- Use graceful shutdown patterns to prevent task loss during deployments
- Use auto-registration for rapid development
- Test with different worker configurations
- Monitor queue sizes during load testing
- Use the built-in monitoring endpoints
- Take advantage of the CLI tools for testing
- **Use configuration files** (`task-queue.toml`) instead of hardcoded values

## Troubleshooting

### Common Issues

1. **Workers not processing tasks**: Check Redis connectivity and task registration
2. **High memory usage**: Monitor task payload sizes and Redis memory
3. **Slow processing**: Consider increasing worker count or optimizing task logic
4. **Connection issues**: Verify Redis URL and network connectivity
5. **Tasks getting re-queued frequently**: Increase `max_concurrent_tasks` or optimize task execution time
6. **Workers not shutting down gracefully**: Check for long-running tasks and adjust shutdown timeout
7. **High active task count**: Monitor task execution patterns and consider load balancing

## Documentation

- [API Documentation](https://docs.rs/rust-task-queue)
- [Development Guide](DEVELOPMENT.md) - Comprehensive development documentation

## Maintenance Status

**Actively Developed** - Regular releases, responsive to issues, feature requests welcome.

**Compatibility:**

- Rust 1.70.0+
- Redis 6.0+
- Tokio 1.0+

## Contributing

We welcome contributions! Please see our [Development Guide](DEVELOPMENT.md) for:

- Development setup instructions
- Code style guidelines
- Testing requirements
- Performance benchmarking
- Documentation standards

## License

Licensed under either of Apache License, Version 2.0 or MIT License at your option.