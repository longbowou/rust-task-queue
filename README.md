# Rust Task Queue

[![Crates.io](https://img.shields.io/crates/v/rust-task-queue.svg)](https://crates.io/crates/rust-task-queue)
[![Documentation](https://docs.rs/rust-task-queue/badge.svg)](https://docs.rs/rust-task-queue)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/longbowou/rust-task-queue)
[![Downloads](https://img.shields.io/crates/d/rust-task-queue.svg)](https://crates.io/crates/rust-task-queue)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-blue.svg)](https://www.rust-lang.org)
[![CI](https://github.com/longbowou/rust-task-queue/workflows/CI/badge.svg)](https://github.com/longbowou/rust-task-queue/actions)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()
[![Test Coverage](https://img.shields.io/badge/tests-220+%20passing-brightgreen.svg)]()

A high-performance, Redis-backed task queue framework with **Enhanced Auto-scaling**, intelligent async task spawning,
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
- **CLI tools** for standalone workers with process separation and logging configuration
- **Automatic task registration** with procedural macros
- **Production-ready** with robust error handling and safety improvements
- **High performance** with MessagePack serialization and connection pooling
- **Advanced async task spawning** with intelligent backpressure and resource management
- **Graceful shutdown** with active task tracking and cleanup
- **Smart resource allocation** with semaphore-based concurrency control
- **Comprehensive testing** with unit, integration, performance, and security tests
- **Enterprise-grade tracing** with lifecycle tracking, performance monitoring, and error context

## Why Choose Rust Task Queue?

| Feature       | Rust Task Queue     | Celery    | Sidekiq   | Bull      |
|---------------|---------------------|-----------|-----------|-----------|
| Language      | Rust                | Python    | Ruby      | Node.js   |
| Auto-scaling  | Multi-dimensional   | ❌         | ❌         | ❌         |
| Performance   | <40ns serialization | ~ms       | ~ms       | ~ms       |
| Type Safety   | Compile-time        | ❌ Runtime | ❌ Runtime | ❌ Runtime |
| Memory Safety | Zero-copy           | ❌         | ❌         | ❌         |
| Async/Await   | Native              | ❌         | ❌         | ✅         |

## Production Ready

- **220+ comprehensive tests** (unit, integration, performance, security)
- **Zero clippy warnings** with strict linting
- **Memory safe** - no unsafe code
- **Performance benchmarked** - <40ns serialization
- **Enterprise logging** with structured tracing
- **Graceful shutdown** and error recovery
- **Redis cluster support** with connection pooling

## Recent Improvements

- **Comprehensive test suite**: 220+ total tests (122 unit + 9 integration + 22 actix + 6 performance + 11 security + 9
  error scenario + 5 benchmarks)
- **New Actix Web metrics API**: 15+ comprehensive endpoints for monitoring and diagnostics
- **Performance optimizations**: Sub-40ns serialization/deserialization
- **Clippy compliance**: Zero warnings with strict linting rules
- **Enhanced error handling**: Improved TaskQueueError creation and private method access
- **Benchmark suite**: Detailed performance metrics for all core operations
- **CI/CD improvements**: Automated testing with Redis container support
- **Configuration improvements**: Streamlined Default trait implementations
- **Enhanced Auto-scaling**: Multi-dimensional scaling with adaptive learning

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
- `tracing`: **Enterprise-grade structured logging and observability**
    - Complete task lifecycle tracking with distributed spans
    - Performance monitoring and execution timing
    - Error chain analysis with deep context
    - Worker activity and resource utilization monitoring
    - Production-ready logging configuration (JSON/compact/pretty formats)
    - Environment-based configuration support
- `actix-integration`: Web framework integration with built-in endpoints
- `cli`: Standalone worker binaries with logging configuration support
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

Enables: logging, automatic task discovery, configuration files, and CLI worker tools.

**Web Application:**

```toml
[dependencies]
rust-task-queue = { version = "0.1", features = ["tracing", "auto-register", "actix-integration", "config-support"] }
```

Includes Actix Web routes (`/tasks/health`, `/tasks/stats`) and middleware.

**Minimal Setup:**

```toml
[dependencies]
rust-task-queue = { version = "0.1", default-features = false, features = ["tracing"] }
```

Core functionality only - manual task registration required.

**Development/Full Features:**

```toml
[dependencies]
rust-task-queue = { version = "0.1", features = ["full"] }
```

All features including experimental dynamic loading capabilities.

## Integration Patterns

### Pattern 1: Actix Web with Separate Workers (Recommended)

This pattern provides the best separation of concerns and scalability.

**Recommended**: Use external configuration files (`task-queue.toml` or `task-queue.yaml`) for production deployments:

**1. Create worker configuration file at the root of your project (`task-queue.toml`):**

```toml
[redis]
url = "redis://127.0.0.1:6379"
pool_size = 10

[workers]
initial_count = 0  # No workers in web-only mode
max_concurrent_tasks = 10

[autoscaler]
enabled = true
min_workers = 1
max_workers = 20
scale_up_count = 2           # workers to add when scaling up
scale_down_count = 1         # workers to remove when scaling down

# Multi-dimensional scaling triggers (Enhanced Auto-scaling)
[autoscaler.scaling_triggers]
queue_pressure_threshold = 1.2        # weighted queue depth per worker
worker_utilization_threshold = 0.85   # target worker utilization (85%)
task_complexity_threshold = 2.0       # complex task overload factor
error_rate_threshold = 0.03           # maximum 3% error rate
memory_pressure_threshold = 1024.0    # memory usage per worker (MB)

# Adaptive threshold learning (SLA-driven optimization)
enable_adaptive_thresholds = true
learning_rate = 0.05                  # conservative learning for production
adaptation_window_minutes = 60        # longer window for stability

# Hysteresis and stability controls
scale_up_cooldown_seconds = 180       # 3 minutes between scale-ups
scale_down_cooldown_seconds = 900     # 15 minutes between scale-downs
consecutive_signals_required = 3      # require 3 consecutive signals

# SLA performance targets for adaptive learning
[autoscaler.target_sla]
max_p95_latency_ms = 3000.0           # 3 second P95 latency target
min_success_rate = 0.99               # 99% success rate target
max_queue_wait_time_ms = 5000.0       # 5 second max queue wait
target_worker_utilization = 0.75     # optimal 75% worker utilization

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

The framework includes a powerful CLI tool for running workers in separate processes, now with enhanced auto-scaling
support.

### Enhanced CLI Usage with Auto-scaling

```bash
# Start workers with enhanced multi-dimensional auto-scaling
cargo run --bin task-worker --features cli,auto-register worker \
  --redis-url redis://localhost:6379 \
  --workers 8 \
  --enable-autoscaler \
  --enable-scheduler \
  --config task-queue.toml  # Use enhanced configuration

# Workers with custom auto-scaling thresholds
cargo run --bin task-worker --features cli,auto-register worker \
  --workers 4 \
  --enable-autoscaler \
  --autoscaler-min-workers 2 \
  --autoscaler-max-workers 20 \
  --autoscaler-scale-up-threshold 1.5 \
  --autoscaler-consecutive-signals 3

# Monitor auto-scaling in real-time
cargo run --bin task-worker --features cli,auto-register worker \
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

### Advanced CLI Examples

```bash
# Workers with auto-scaling and scheduling
cargo run --bin task-worker --features cli,auto-register worker \
  --workers 6 \
  --enable-autoscaler \
  --enable-scheduler

# Workers with production logging (JSON format)
cargo run --bin task-worker --features cli,auto-register worker \
  --workers 4 \
  --log-level info \
  --log-format json

# Development workers with detailed tracing
cargo run --bin task-worker --features cli,auto-register worker \
  --workers 2 \
  --log-level debug \
  --log-format pretty

# Workers for specific queues only
cargo run --bin task-worker --features cli,auto-register worker \
  --workers 4 \
  --queues "high_priority,default" \
  --log-level info

# Workers with custom naming and logging
cargo run --bin task-worker --features cli,auto-register worker \
  --workers 2 \
  --worker-prefix "priority-worker" \
  --queues "high_priority" \
  --log-level debug \
  --log-format compact
```

## Enhanced Auto-scaling

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

## Performance Benchmarks

Recent benchmark results demonstrate exceptional performance:

| Operation                  | Time      | Status      |
|----------------------------|-----------|-------------|
| Task Serialization         | ~39.15 ns | Excellent   |
| Task Deserialization       | ~31.51 ns | Excellent   |
| Queue Config Lookup        | ~39.76 ns | Excellent   |
| Queue Management           | ~1.38 µs  | Very Good   |
| Enhanced AutoScaler Config | ~617 ps   | Outstanding |

*Benchmarks run on optimized release builds with statistical analysis*

## Test Coverage

The project maintains comprehensive test coverage across multiple dimensions:

- **Unit Tests**: 122 tests covering all core functionality
- **Integration Tests**: 9 tests for end-to-end workflows
- **Actix Integration Tests**: 22 tests for web endpoints and metrics API
- **Error Scenario Tests**: 9 tests for edge cases and failure modes
- **Performance Tests**: 6 tests for throughput and load handling
- **Security Tests**: 11 tests for injection attacks and safety
- **Benchmarks**: 5 performance benchmarks for optimization

**Total**: 220+ tests ensuring reliability and performance

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
cargo run --bin task-worker worker --workers 4
```

### Structured Logging Output

**JSON Format (Production):**

```json
{
  "timestamp": "2024-01-15T10:30:00.123Z",
  "level": "INFO",
  "target": "rust_task_queue::broker",
  "span": {
    "name": "enqueue_task",
    "task_id": "abc123"
  },
  "fields": {
    "task_type": "ProcessOrderTask",
    "queue": "high_priority",
    "payload_size": 1024,
    "priority": "high"
  },
  "message": "Task enqueued successfully"
}
```

**Compact Format (Development):**

```
2024-01-15T10:30:00.123Z INFO enqueue_task{task_id=abc123}: rust_task_queue::broker: Task enqueued successfully task_type="ProcessOrderTask" queue="high_priority"
```

### Tracing Integration

```rust
use rust_task_queue::prelude::*;
use tracing::{info, warn, error};

#[derive(Debug, Serialize, Deserialize, Default, AutoRegisterTask)]
struct MyTask {
    data: String,
}

#[async_trait]
impl Task for MyTask {
    async fn execute(&self) -> TaskResult {
        // Automatic tracing spans are created for task execution
        info!("Processing task with data: {}", self.data);

        // Your business logic here
        let result = process_data(&self.data).await?;

        Ok(serde_json::json!({"result": result}))
    }

    fn name(&self) -> &str { "my_task" }
}
```

### Performance Monitoring

The tracing system provides detailed performance insights:

- **Task Execution Timing**: Individual task performance tracking
- **Queue Depth Monitoring**: Real-time queue status and trends
- **Worker Utilization**: Capacity analysis and efficiency metrics
- **Error Rate Tracking**: System health and failure analysis
- **Throughput Analysis**: Tasks per second and bottleneck identification

### Production Integration

Perfect for integration with observability platforms:

- **ELK Stack**: Elasticsearch, Logstash, and Kibana
- **Datadog**: Application Performance Monitoring
- **Splunk**: Log aggregation and analysis
- **Grafana**: Metrics visualization and alerting
- **OpenTelemetry**: Distributed tracing standards

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

```bash
# Example usage
curl http://localhost:3000/tasks/health              # Component health check
curl http://localhost:3000/tasks/metrics            # All metrics combined
curl http://localhost:3000/tasks/metrics/queues     # Queue-specific metrics
curl http://localhost:3000/tasks/diagnostics        # System diagnostics
```

## Production Deployment with Enhanced Auto-scaling

### Recommended Production Configuration

```toml
# production-task-queue.toml
[redis]
url = "rediss://redis-cluster.production.com:6380"
pool_size = 20
connection_timeout = 10
command_timeout = 5

[workers]
initial_count = 10
max_concurrent_tasks = 25
heartbeat_interval = 30
shutdown_grace_period = 120

[autoscaler]
enabled = true
min_workers = 5
max_workers = 50
scale_up_count = 2
scale_down_count = 1

# Production-tuned multi-dimensional scaling
[autoscaler.scaling_triggers]
queue_pressure_threshold = 1.2
worker_utilization_threshold = 0.85
task_complexity_threshold = 2.0
error_rate_threshold = 0.03
memory_pressure_threshold = 1024.0

# Conservative adaptive learning for production
enable_adaptive_thresholds = true
learning_rate = 0.05
adaptation_window_minutes = 60

# Stability-first cooldowns
scale_up_cooldown_seconds = 180     # 3 minutes
scale_down_cooldown_seconds = 900   # 15 minutes
consecutive_signals_required = 3

# Strict SLA targets
[autoscaler.target_sla]
max_p95_latency_ms = 3000.0
min_success_rate = 0.99
max_queue_wait_time_ms = 5000.0
target_worker_utilization = 0.75

[metrics]
enabled = true
export_interval = 60
collect_scaling_metrics = true
collect_sla_metrics = true
collect_adaptive_threshold_metrics = true

[logging]
level = "info"              # trace, debug, info, warn, error
format = "json"             # json, compact, pretty
enable_task_lifecycle = true
enable_performance_monitoring = true
enable_error_chain_analysis = true

[alerts]
max_queue_pressure_score = 2.0
min_worker_utilization = 0.10
max_worker_utilization = 0.95
max_scaling_frequency_per_hour = 10
```

### Production Deployment Script

```bash
#!/bin/bash
# deploy-production.sh

# Web server (no workers)
docker run -d \
  --name task-queue-web \
  -p 3000:3000 \
  -v $(pwd)/production-task-queue.toml:/app/task-queue.toml \
  -e REDIS_URL=rediss://redis-cluster.production.com:6380 \
  your-app:latest

# Enhanced auto-scaling workers (separate containers)
docker run -d \
  --name task-queue-workers \
  -v $(pwd)/production-task-queue.toml:/app/task-queue.toml \
  -e LOG_LEVEL=info \
  -e LOG_FORMAT=json \
  --restart unless-stopped \
  your-app:latest \
  cargo run --bin task-worker --features cli,auto-register worker \
    --config /app/task-queue.toml

# Specialized high-priority workers
docker run -d \
  --name task-queue-priority-workers \
  -v $(pwd)/production-task-queue.toml:/app/task-queue.toml \
  --restart unless-stopped \
  your-app:latest \
  cargo run --bin task-worker --features cli,auto-register worker \
    --config /app/task-queue.toml \
    --queues "high_priority" \
    --workers 4
```

### Monitoring Enhanced Auto-scaling

```bash
# Monitor auto-scaling decisions in real-time with structured logging
docker logs -f task-queue-workers | jq 'select(.fields.operation == "scaling_decision")'

# View detailed scaling metrics
curl http://localhost:3000/tasks/metrics | jq '.autoscaler'

# Check SLA performance
curl http://localhost:3000/tasks/health | jq '.sla_metrics'

# Monitor task lifecycle events
docker logs -f task-queue-workers | jq 'select(.span.name == "task_execution")'

# Track performance metrics
docker logs -f task-queue-workers | jq 'select(.fields.event_type == "performance_metric")'

# View error chain analysis
docker logs -f task-queue-workers | jq 'select(.level == "ERROR")' | jq '.fields.error_chain'
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
- **Enhanced async task spawning**: Restructured task execution with proper resource management
- **Intelligent backpressure**: Automatic task queuing when workers are at capacity
- **Active task tracking**: Real-time monitoring of running tasks with atomic counters
- **Graceful shutdown**: Workers wait for active tasks to complete before terminating

## Advanced Worker Architecture

### Intelligent Task Spawning

The framework features a sophisticated async task spawning system that provides:

#### **Structured Task Execution**

- **Context-based spawning**: Centralized execution context for better resource management
- **Atomic task tracking**: Real-time monitoring of active tasks using `AtomicUsize` counters
- **Resource safety**: Proper cleanup and RAII patterns throughout the lifecycle

#### **Backpressure Management**

```rust
// Workers automatically handle capacity limits
let worker = Worker::new("worker-1".to_string(), broker, scheduler)
.with_max_concurrent_tasks(10)  // Limit concurrent execution
.with_task_registry(registry);

let started_worker = worker.start().await?;

// Real-time task monitoring
println!("Active tasks: {}", started_worker.active_task_count());
```

#### **Execution Flow**

1. **Task Dequeuing**: Workers pull tasks from priority-ordered queues
2. **Capacity Check**: Semaphore-based concurrency control prevents overload
3. **Smart Spawning**: Tasks are spawned asynchronously or queued based on capacity
4. **Resource Tracking**: Active task counters provide real-time visibility
5. **Graceful Cleanup**: Automatic resource cleanup when tasks complete

#### **Backpressure Strategies**

- **At Capacity**: Tasks are automatically re-queued for later processing
- **Intelligent Delays**: Configurable delays prevent tight loops during high load
- **Resource Monitoring**: Real-time feedback on worker utilization

#### **Graceful Shutdown**

```rust
// Workers wait for active tasks before shutting down
worker.stop().await;  // Waits up to 30 seconds for task completion
```

### Worker Configuration Examples

#### **Basic Worker Setup**

```rust
use rust_task_queue::prelude::*;

let worker = Worker::new("worker-1".to_string(), broker, scheduler)
.with_max_concurrent_tasks(5)
.with_task_registry(registry);

let started_worker = worker.start().await?;
```

#### **Advanced Configuration**

```rust
use rust_task_queue::WorkerBackpressureConfig;

let backpressure_config = WorkerBackpressureConfig {
max_concurrent_tasks: 10,
queue_size_threshold: 100,
backpressure_delay_ms: 50,
};

let worker = Worker::new("worker-advanced".to_string(), broker, scheduler)
.with_backpressure_config(backpressure_config)
.with_task_registry(registry);
```

#### **Production Monitoring**

```rust
// Monitor worker health and performance
let active_tasks = worker.active_task_count();
let queue_metrics = task_queue.broker.get_queue_metrics("default").await?;

println!("Worker Status: {} active tasks", active_tasks);
println!("Queue Status: {} pending tasks", queue_metrics.pending_tasks);
```

## Monitoring and Observability

The framework provides comprehensive built-in monitoring endpoints when using Actix integration:

```bash
# Health and system status
curl http://localhost:3000/tasks/health             # Component health
curl http://localhost:3000/tasks/status             # System status

# Comprehensive metrics
curl http://localhost:3000/tasks/metrics            # All metrics combined
curl http://localhost:3000/tasks/metrics/system     # System metrics
curl http://localhost:3000/tasks/metrics/queues     # Queue metrics
curl http://localhost:3000/tasks/metrics/workers    # Worker metrics
curl http://localhost:3000/tasks/metrics/autoscaler # AutoScaler metrics

# Administrative endpoints
curl http://localhost:3000/tasks/diagnostics        # System diagnostics
curl http://localhost:3000/tasks/alerts             # Active alerts
curl http://localhost:3000/tasks/sla                # SLA violations
curl http://localhost:3000/tasks/uptime             # Runtime information

# Task registry
curl http://localhost:3000/tasks/registered         # Registered tasks
curl http://localhost:3000/tasks/registry/info      # Registry information
```

### Logging and Observability

Enable comprehensive structured logging and tracing:

```rust
use rust_task_queue::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Production logging configuration
    configure_production_logging(
        LogLevel::Info,
        LogFormat::Json
    );

    // Alternative: Environment-based configuration
    // LOG_LEVEL=info LOG_FORMAT=json
    configure_logging_from_env();

    let task_queue = TaskQueueBuilder::new("redis://localhost:6379")
        .auto_register_tasks()
        .initial_workers(4)
        .build()
        .await?;

    // All operations are automatically traced
    let task = MyTask { data: "example".to_string() };
    let task_id = task_queue.enqueue(task, queue_names::DEFAULT).await?;

    Ok(())
}
```

**Development Setup:**

```rust
// For development with detailed tracing
configure_production_logging(LogLevel::Debug, LogFormat::Pretty);
```

**Production Setup:**

```rust
// For production with JSON structured logs
configure_production_logging(LogLevel::Info, LogFormat::Json);
```

### Worker Monitoring

The enhanced worker architecture provides comprehensive monitoring capabilities:

```rust
use rust_task_queue::prelude::*;

// Real-time worker status
let active_count = worker.active_task_count();
println!("Worker has {} active tasks", active_count);

// Queue metrics for capacity planning
let queue_metrics = task_queue.broker.get_queue_metrics("default").await?;
println!("Queue depth: {} pending, {} processed",
         queue_metrics.pending_tasks,
         queue_metrics.processed_tasks);

// Auto-scaler insights  
let autoscaler_metrics = task_queue.autoscaler.collect_metrics().await?;
println!("Tasks per worker: {:.2}", autoscaler_metrics.tasks_per_worker);
```

#### **Key Metrics to Monitor**

- **Active Task Count**: Real-time view of worker utilization
- **Queue Depth**: Pending tasks across all priority queues
- **Task Processing Rate**: Completed tasks per second
- **Backpressure Events**: Tasks re-queued due to capacity limits
- **Graceful Shutdown Time**: How long workers take to complete active tasks

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
- Configure appropriate `max_concurrent_tasks` based on workload characteristics
- Monitor active task counts to optimize worker capacity
- Use graceful shutdown patterns to prevent task loss during deployments

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
- **Smart concurrency**: Atomic task tracking with minimal overhead
- **Efficient spawning**: Context-based execution reduces resource allocation
- **Backpressure handling**: Intelligent task re-queuing prevents system overload

## Development and Testing

### Running Tests

The project includes a comprehensive test suite with automated Redis setup:

```bash
# Run all tests with Redis container (recommended)
./scripts/run-tests.sh

# If tests fail and leave containers running, clean up with:
./scripts/cleanup-redis.sh

# Or run individual test suites:
cargo test --lib                           # Unit tests (122 tests)
cargo test --test integration_tests        # Integration tests (9 tests)
cargo test --test actix_integration_tests  # Actix Web tests (22 tests)
cargo test --test error_scenarios_tests    # Error handling tests (9 tests) 
cargo test --test performance_tests        # Performance tests (6 tests)
cargo test --test security_tests           # Security tests (11 tests)

# Run benchmarks
cargo bench                                # Performance benchmarks (5 benchmarks)
```

### Code Quality

The project maintains high code quality standards:

```bash
# Clippy linting (zero warnings)
cargo clippy --all-targets --all-features -- -D warnings

# Format code
cargo fmt

# Check compilation
cargo check --all-targets --all-features
```

### Testing Infrastructure

- **Automated Redis Setup**: Tests automatically start/stop Redis containers
- **Robust Cleanup**: Improved container management with trap handlers for graceful cleanup
- **Container Recovery**: Automatic detection and cleanup of leftover containers
- **Test Isolation**: Each test uses separate Redis databases
- **Comprehensive Coverage**: Unit, integration, performance, and security tests
- **CI/CD Ready**: All tests pass with strict linting enabled
- **Performance Tracking**: Benchmarks track performance regressions
- **Failure Recovery**: Dedicated cleanup script for manual container management

## Troubleshooting

### Common Issues

1. **Workers not processing tasks**: Check Redis connectivity and task registration
2. **High memory usage**: Monitor task payload sizes and Redis memory
3. **Slow processing**: Consider increasing worker count or optimizing task logic
4. **Connection issues**: Verify Redis URL and network connectivity
5. **Tasks getting re-queued frequently**: Increase `max_concurrent_tasks` or optimize task execution time
6. **Workers not shutting down gracefully**: Check for long-running tasks and adjust shutdown timeout
7. **High active task count**: Monitor task execution patterns and consider load balancing

### Debug Mode

Enable debug logging with multiple options:

```bash
# Use built-in logging configuration
cargo run --bin task-worker --features cli worker \
  --log-level debug --log-format pretty

# Environment-based configuration
LOG_LEVEL=debug LOG_FORMAT=pretty \
  cargo run --bin task-worker --features cli worker

# Detailed tracing for specific components
LOG_LEVEL=trace \
  cargo run --bin task-worker --features cli worker

# Production debugging with JSON format
LOG_LEVEL=debug LOG_FORMAT=json \
  cargo run --bin task-worker --features cli worker | jq
```

### Testing Issues

If tests fail with Redis connection errors:

```bash
# Ensure Redis is running
docker run -d --name redis-test -p 6379:6379 redis:7-alpine

# Or use the automated test script
./scripts/run-tests.sh
```

## Documentation

- [API Documentation](https://docs.rs/rust-task-queue)
- [Development Guide](DEVELOPMENT.md) - Comprehensive development documentation
- [Examples](examples/) - Working examples and patterns

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