# Rust Task Queue - Development Guide

A high-performance, Redis-backed task queue framework with auto-scaling capabilities designed for async Rust applications.

## 📋 Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [Configuration](#configuration)
- [Testing](#testing)
- [Benchmarking](#benchmarking)
- [API Reference](#api-reference)
- [Performance Characteristics](#performance-characteristics)
- [Contributing](#contributing)
- [Troubleshooting](#troubleshooting)

## 🎯 Overview

### Key Features

- **Redis-backed broker** for reliable message delivery
- **Auto-scaling workers** based on queue load
- **Task scheduling** with delay support
- **Multiple queue priorities** (high, default, low)
- **Retry logic** with configurable attempts
- **Task timeouts** and failure handling
- **Metrics and monitoring** with health checks
- **Actix Web integration** (optional)
- **Automatic task registration** via procedural macros
- **Comprehensive error handling** with structured errors
- **Connection pooling** for optimal Redis performance

### Performance Highlights

- **Task Serialization**: ~40ns per operation
- **Task Deserialization**: ~34ns per operation
- **Queue Config Lookup**: ~40ns per operation
- **AutoScaler Config Creation**: ~651ps per operation

## 🏗️ Architecture

### Core Components

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   TaskQueue     │    │   RedisBroker   │    │   TaskRegistry  │
│                 │◄──►│                 │    │                 │
│ • Coordination  │    │ • Queue Ops     │    │ • Task Types    │
│ • Worker Mgmt   │    │ • Persistence   │    │ • Executors     │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Worker Pool   │    │   Redis Pool    │    │   AutoScaler    │
│                 │    │                 │    │                 │
│ • Task Exec     │    │ • Connections   │    │ • Metrics       │
│ • Heartbeat     │    │ • Health Check  │    │ • Scaling       │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### Data Flow

1. **Task Enqueue**: Tasks are serialized and pushed to Redis queues
2. **Worker Dequeue**: Workers pull tasks from priority-ordered queues
3. **Task Execution**: Tasks are deserialized and executed
4. **Result Handling**: Success/failure is tracked with metrics
5. **Auto-scaling**: System monitors load and adjusts worker count

## 🚀 Development Setup

### Prerequisites

- **Rust** 1.70+ (2021 edition)
- **Redis** 6.0+ (for testing and development)
- **Docker** (optional, for Redis container)

### Quick Start

```bash
# Clone the repository
git clone <repository-url>
cd rust-task-queue

# Start Redis (using Docker)
docker run -d -p 6379:6379 redis:7-alpine

# Or install Redis locally
# macOS: brew install redis && brew services start redis
# Ubuntu: sudo apt install redis-server && sudo systemctl start redis

# Build the project
cargo build

# Run tests
cargo test

# Run benchmarks
cargo bench

# Build documentation
cargo doc --open
```

### Environment Variables

```bash
# Redis Configuration
export REDIS_URL="redis://127.0.0.1:6379"
export REDIS_POOL_SIZE=10
export REDIS_CONNECTION_TIMEOUT=30
export REDIS_COMMAND_TIMEOUT=30

# Worker Configuration
export TASK_QUEUE_WORKERS=4
export TASK_QUEUE_AUTO_REGISTER=true
export TASK_QUEUE_SCHEDULER=true

# Development/Testing
export REDIS_TEST_URL="redis://127.0.0.1:6379/15"
```

## 📁 Project Structure

```
rust-task-queue/
├── src/
│   ├── lib.rs                 # Main library entry point
│   ├── config.rs              # Configuration management
│   ├── error.rs               # Error types & handling
│   ├── broker.rs              # Redis broker implementation
│   ├── worker.rs              # Worker pool management
│   ├── task.rs                # Task trait & registry
│   ├── scheduler.rs           # Task scheduling
│   ├── autoscaler.rs          # Auto-scaling logic
│   ├── queue.rs               # Queue configuration
│   ├── actix.rs               # Actix Web integration
│   ├── cli.rs                 # CLI utilities
│   ├── prelude.rs             # Common imports
│   └── bin/
│       ├── task-worker.rs     # Worker binary
│       └── task-worker-env-only.rs
├── tests/
│   └── integration_tests.rs   # Integration tests
├── benches/
│   ├── task_processing.rs     # Task processing benchmarks
│   └── queue_operations.rs    # Queue operation benchmarks
├── examples/
│   ├── performance_test.rs    # Performance examples
│   └── actix-integration/     # Actix Web examples
├── macros/                    # Procedural macros
├── Cargo.toml                 # Dependencies & metadata
├── README.md                  # User documentation
├── DEVELOPMENT.md             # This file
├── CONFIGURATION.md           # Configuration guide
└── task-queue.{toml,yaml}     # Example configs
```

## ⚙️ Configuration

### Configuration Sources (Priority Order)

1. **Environment Variables** (highest priority)
2. **Configuration Files** (`task-queue.toml`, `task-queue.yaml`)
3. **Default Values** (lowest priority)

### Configuration Options

```toml
# task-queue.toml
[redis]
url = "redis://127.0.0.1:6379"
pool_size = 10
connection_timeout = 30
command_timeout = 30

[workers]
initial_count = 4
max_concurrent_tasks = 10
heartbeat_interval = 30
shutdown_grace_period = 60

[autoscaler]
min_workers = 1
max_workers = 20
scale_up_threshold = 5.0
scale_down_threshold = 1.0
scale_up_count = 2
scale_down_count = 1

[scheduler]
enabled = true
tick_interval = 10
max_tasks_per_tick = 100

[auto_register]
enabled = true

[actix]
auto_configure_routes = true
route_prefix = "/tasks"
enable_metrics = true
enable_health_check = true
```

### Builder Pattern

```rust
let task_queue = TaskQueueBuilder::new("redis://localhost:6379")
    .initial_workers(4)
    .auto_register_tasks()
    .with_scheduler()
    .with_autoscaler()
    .build()
    .await?;
```

## 🧪 Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test integration_tests

# Run with Redis container
docker run -d --name redis-test -p 6379:6379 redis:7-alpine
REDIS_TEST_URL="redis://127.0.0.1:6379/15" cargo test
docker stop redis-test && docker rm redis-test

# Run tests with logging
RUST_LOG=debug cargo test

# Run specific test
cargo test test_basic_task_execution
```

### Test Structure

```rust
// Example test task
#[derive(Debug, Serialize, Deserialize, Clone)]
struct TestTask {
    data: String,
    should_fail: bool,
}

#[async_trait::async_trait]
impl Task for TestTask {
    async fn execute(&self) -> TaskResult {
        if self.should_fail {
            return Err("Intentional failure".into());
        }
        
        let response = serde_json::json!({
            "status": "completed",
            "data": format!("Processed: {}", self.data)
        });
        
        Ok(serde_json::to_vec(&response)?)
    }

    fn name(&self) -> &str { "test_task" }
    fn max_retries(&self) -> u32 { 3 }
    fn timeout_seconds(&self) -> u64 { 30 }
}
```

### Test Coverage Areas

- ✅ **Basic Task Execution**
- ✅ **Retry Mechanism**
- ✅ **Task Scheduling**
- ✅ **Auto-scaling Metrics**
- ✅ **Queue Priorities**
- ✅ **Health Checks**
- ✅ **Error Handling**

## 📊 Benchmarking

### Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench task_processing
cargo bench queue_operations

# Generate detailed reports
cargo bench -- --verbose

# Custom sample size
cargo bench -- --sample-size 1000
```

### Benchmark Results

| Operation | Time | Notes |
|-----------|------|-------|
| Task Serialization | ~40ns | MessagePack encoding |
| Task Deserialization | ~34ns | MessagePack decoding |
| Queue Config Lookup | ~40ns | HashMap access |
| Get Queues by Priority | ~1.46µs | Sorting overhead |
| AutoScaler Config Creation | ~651ps | Struct initialization |

### Adding New Benchmarks

```rust
// benches/my_benchmark.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rust_task_queue::prelude::*;

fn bench_my_operation(c: &mut Criterion) {
    c.bench_function("my_operation", |b| {
        b.iter(|| {
            let result = my_operation();
            black_box(result);
        })
    });
}

criterion_group!(benches, bench_my_operation);
criterion_main!(benches);
```

## 📚 API Reference

### Core Types

```rust
// Main task queue
pub struct TaskQueue {
    pub broker: Arc<RedisBroker>,
    pub scheduler: Arc<TaskScheduler>,
    pub autoscaler: Arc<AutoScaler>,
    // ...
}

// Task trait
#[async_trait]
pub trait Task: Send + Sync + Serialize + for<'de> Deserialize<'de> + Debug {
    async fn execute(&self) -> TaskResult;
    fn name(&self) -> &str;
    fn max_retries(&self) -> u32 { 3 }
    fn timeout_seconds(&self) -> u64 { 300 }
}

// Error types
#[derive(Error, Debug)]
pub enum TaskQueueError {
    Redis(#[from] redis::RedisError),
    TaskExecution(String),
    TaskNotFound(String),
    TaskTimeout { id: String, timeout_seconds: u64 },
    // ...
}
```

### Key Methods

```rust
impl TaskQueue {
    // Creation
    pub async fn new(redis_url: &str) -> Result<Self, TaskQueueError>;
    
    // Worker management
    pub async fn start_workers(&self, count: usize) -> Result<(), TaskQueueError>;
    pub async fn stop_workers(&self);
    pub async fn worker_count(&self) -> usize;
    
    // Task operations
    pub async fn enqueue<T: Task>(&self, task: T, queue: &str) -> Result<TaskId, TaskQueueError>;
    pub async fn schedule<T: Task>(&self, task: T, queue: &str, delay: Duration) -> Result<TaskId, TaskQueueError>;
    
    // Monitoring
    pub async fn health_check(&self) -> Result<HealthStatus, TaskQueueError>;
    pub async fn get_metrics(&self) -> Result<TaskQueueMetrics, TaskQueueError>;
    
    // Scheduler
    pub async fn start_scheduler(&self) -> Result<(), TaskQueueError>;
    pub async fn stop_scheduler(&self);
}
```

### Usage Examples

#### Basic Usage

```rust
use rust_task_queue::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct EmailTask {
    to: String,
    subject: String,
    body: String,
}

#[async_trait::async_trait]
impl Task for EmailTask {
    async fn execute(&self) -> TaskResult {
        // Send email logic here
        println!("Sending email to: {}", self.to);
        
        let response = serde_json::json!({
            "status": "sent",
            "timestamp": chrono::Utc::now()
        });
        
        Ok(serde_json::to_vec(&response)?)
    }

    fn name(&self) -> &str {
        "email_task"
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let task_queue = TaskQueue::new("redis://localhost:6379").await?;
    
    // Register task type
    let mut registry = TaskRegistry::new();
    registry.register_with_name::<EmailTask>("email_task")?;
    
    // Start workers
    task_queue.start_workers_with_registry(2, Arc::new(registry)).await?;
    
    // Enqueue task
    let email = EmailTask {
        to: "user@example.com".to_string(),
        subject: "Welcome!".to_string(),
        body: "Welcome to our service!".to_string(),
    };
    
    let task_id = task_queue.enqueue(email, "default").await?;
    println!("Enqueued email task: {}", task_id);
    
    Ok(())
}
```

#### Auto-Registration

```rust
use rust_task_queue::prelude::*;

#[derive(Debug, Serialize, Deserialize, Default, AutoRegisterTask)]
struct ProcessDataTask {
    data: String,
}

#[async_trait::async_trait]
impl Task for ProcessDataTask {
    async fn execute(&self) -> TaskResult {
        // Process data
        let result = format!("Processed: {}", self.data);
        Ok(result.into_bytes())
    }

    fn name(&self) -> &str {
        "process_data"
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Tasks are automatically registered!
    let task_queue = TaskQueueBuilder::new("redis://localhost:6379")
        .auto_register_tasks()
        .initial_workers(4)
        .build()
        .await?;
    
    let task = ProcessDataTask {
        data: "Hello, World!".to_string(),
    };
    
    let task_id = task_queue.enqueue(task, "default").await?;
    println!("Task enqueued: {}", task_id);
    
    Ok(())
}
```

#### Monitoring and Health Checks

```rust
// Health check
let health = task_queue.health_check().await?;
println!("System status: {}", health.status);

for (component, status) in health.components {
    println!("  {}: {} - {:?}", component, status.status, status.message);
}

// Metrics
let metrics = task_queue.get_metrics().await?;
println!("Active workers: {}", metrics.active_workers);
println!("Tasks per worker: {:.2}", metrics.tasks_per_worker);
println!("Total pending: {}", metrics.total_pending_tasks);

for queue_metric in metrics.queue_metrics {
    println!("Queue '{}': {} pending, {} processed, {} failed", 
        queue_metric.queue_name,
        queue_metric.pending_tasks,
        queue_metric.processed_tasks,
        queue_metric.failed_tasks
    );
}
```

## 🚀 Performance Characteristics

### Throughput

- **Serialization**: 25M+ ops/sec (40ns per task)
- **Deserialization**: 29M+ ops/sec (34ns per task)
- **Queue Operations**: 25M+ ops/sec for config lookups

### Memory Usage

- **Minimal overhead**: MessagePack serialization
- **Connection pooling**: Configurable Redis connections
- **Worker memory**: Isolated task execution

### Scaling Characteristics

- **Horizontal scaling**: Add more workers
- **Auto-scaling**: Based on queue depth
- **Redis scaling**: Single Redis instance or cluster

### Optimization Tips

1. **Connection Pool Size**: Match to worker count
2. **Batch Operations**: Group related tasks
3. **Queue Priorities**: Use appropriate queues
4. **Monitoring**: Regular health checks
5. **Error Handling**: Proper retry strategies

## 🤝 Contributing

### Development Workflow

1. **Fork** the repository
2. **Create** a feature branch: `git checkout -b feature/my-feature`
3. **Make** your changes
4. **Add** tests for new functionality
5. **Run** tests: `cargo test`
6. **Run** benchmarks: `cargo bench`
7. **Run** clippy: `cargo clippy --all-targets --all-features`
8. **Commit** your changes: `git commit -am 'Add my feature'`
9. **Push** to the branch: `git push origin feature/my-feature`
10. **Create** a Pull Request

### Code Standards

- **Rust 2021 Edition**
- **Clippy clean**: No warnings allowed
- **Formatted**: Use `cargo fmt`
- **Documented**: Public APIs must have docs
- **Tested**: New features need tests
- **Benchmarked**: Performance-critical code needs benchmarks

### Adding New Features

1. **Design**: Consider the API design first
2. **Implement**: Add the feature with proper error handling
3. **Test**: Add comprehensive tests
4. **Document**: Update documentation
5. **Benchmark**: Add benchmarks if performance-critical

### Debugging Tips

```bash
# Enable debug logging
RUST_LOG=rust_task_queue=debug cargo test

# Run specific test with output
cargo test test_name -- --nocapture

# Debug Redis operations
redis-cli monitor

# Check Redis keys
redis-cli keys "*"
```

## 🔧 Troubleshooting

### Common Issues

#### Redis Connection Issues

```bash
# Check Redis is running
redis-cli ping

# Check connection string
export REDIS_URL="redis://127.0.0.1:6379"

# Debug connection
RUST_LOG=redis=debug cargo test
```

#### Worker Issues

```bash
# Check worker registration
redis-cli smembers active_workers

# Check worker heartbeats
redis-cli keys "worker:*:heartbeat"

# Monitor worker activity
RUST_LOG=rust_task_queue::worker=debug cargo test
```

#### Task Execution Issues

```bash
# Check task metadata
redis-cli keys "task:*:metadata"

# Check failed tasks
redis-cli smembers "queue:default:failed_tasks"

# Debug task execution
RUST_LOG=rust_task_queue::worker=debug cargo test
```

#### Performance Issues

```bash
# Run benchmarks
cargo bench

# Profile with perf (Linux)
cargo build --release
perf record target/release/my-binary
perf report

# Check Redis performance
redis-cli --latency-history

# Monitor Redis memory
redis-cli info memory
```

### Getting Help

1. **Check Documentation**: Read the docs first
2. **Search Issues**: Look for similar problems
3. **Enable Logging**: Use `RUST_LOG=debug`
4. **Minimal Reproduction**: Create a simple test case
5. **Open Issue**: Provide full context and logs

## 📄 License

This project is licensed under the MIT OR Apache-2.0 license.

## 🚀 What's Next?

- [ ] **Distributed Mode**: Multi-Redis support
- [ ] **Web UI**: Task monitoring dashboard  
- [ ] **More Integrations**: Axum, Warp, etc.
- [ ] **Batch Processing**: Bulk task operations
- [ ] **Dead Letter Queues**: Failed task handling
- [ ] **Task Dependencies**: Workflow support
- [ ] **Metrics Export**: Prometheus integration
- [ ] **Security**: Authentication & authorization

---

Happy coding! 🦀✨ 