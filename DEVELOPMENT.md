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
- [Recent Improvements](#recent-improvements)
- [Contributing](#contributing)
- [Troubleshooting](#troubleshooting)

## 🎯 Overview

### Key Features

- **Redis-backed broker** with connection pooling and optimized operations
- **Auto-scaling workers** based on queue load with configurable thresholds
- **Task scheduling** with delay support and persistent scheduling
- **Multiple queue priorities** with predefined queue constants for type safety
- **Retry logic** with exponential backoff and configurable attempts
- **Task timeouts** and comprehensive failure handling
- **Metrics and monitoring** with health checks and performance tracking
- **Actix Web integration** (optional) with built-in endpoints
- **Automatic task registration** via procedural macros
- **Comprehensive error handling** with eliminated `unwrap()` calls
- **Connection pooling** for optimal Redis performance with centralized connection management
- **Configuration validation** with comprehensive safety checks

### Performance Highlights

- **Task Serialization**: ~40ns per operation (MessagePack)
- **Task Deserialization**: ~34ns per operation
- **Queue Config Lookup**: ~40ns per operation
- **AutoScaler Config Creation**: ~651ps per operation
- **High throughput**: Thousands of tasks per second
- **Memory efficient**: Optimized serialization and connection pooling

## 🏗️ Architecture

### Core Components

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   TaskQueue     │    │   RedisBroker   │    │   TaskRegistry  │
│                 │◄──►│                 │    │                 │
│ • Coordination  │    │ • Queue Ops     │    │ • Task Types    │
│ • Worker Mgmt   │    │ • Persistence   │    │ • Executors     │
│ • Safety        │    │ • Conn Helper   │    │ • Auto-Register │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Worker Pool   │    │   Redis Pool    │    │   AutoScaler    │
│                 │    │                 │    │                 │
│ • Task Exec     │    │ • Connections   │    │ • Metrics       │
│ • Error Handle  │    │ • Health Check  │    │ • Scaling       │
│ • Retry Logic   │    │ • Centralized   │    │ • Validation    │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### Data Flow

1. **Task Enqueue**: Tasks are serialized (MessagePack) and pushed to Redis queues using queue constants
2. **Worker Dequeue**: Workers pull tasks from priority-ordered queues with robust error handling
3. **Task Execution**: Tasks are deserialized and executed with timeout and retry logic
4. **Result Handling**: Success/failure is tracked with comprehensive metrics
5. **Auto-scaling**: System monitors load and adjusts worker count with validation
6. **Connection Management**: Centralized Redis connection handling for reliability

### Queue Constants

The framework provides predefined queue constants for type safety:

```rust
use rust_task_queue::queue::queue_names;

// Available queue constants
queue_names::DEFAULT        // "default" - Standard priority tasks
queue_names::HIGH_PRIORITY  // "high_priority" - High priority tasks  
queue_names::LOW_PRIORITY   // "low_priority" - Background tasks
```

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
docker run -d --name redis-dev -p 6379:6379 redis:7-alpine

# Or install Redis locally
# macOS: brew install redis && brew services start redis
# Ubuntu: sudo apt install redis-server && sudo systemctl start redis

# Build the project
cargo build

# Run tests (requires Redis running)
cargo test

# Run integration tests specifically
cargo test --test integration_tests

# Run benchmarks
cargo bench

# Build documentation
cargo doc --open

# Start worker CLI for testing
cargo run --bin task-worker --features cli,auto-register worker --workers 2
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
export RUST_LOG=rust_task_queue=debug
```

## 📁 Project Structure

```
rust-task-queue/
├── src/
│   ├── lib.rs                 # Main library entry point & exports
│   ├── config.rs              # Configuration management with validation
│   ├── error.rs               # Error types & comprehensive handling
│   ├── broker.rs              # Redis broker with connection helper
│   ├── worker.rs              # Worker pool with safety improvements
│   ├── task.rs                # Task trait & registry with auto-register
│   ├── scheduler.rs           # Task scheduling with persistence
│   ├── autoscaler.rs          # Auto-scaling with validation
│   ├── queue.rs               # Queue constants & configuration
│   ├── actix.rs               # Actix Web integration with endpoints
│   ├── cli.rs                 # CLI utilities with full feature support
│   ├── prelude.rs             # Common imports for convenience
│   └── bin/
│       ├── task-worker.rs     # Main worker binary with CLI
│       └── task-worker-env-only.rs # Environment-only worker
├── tests/
│   └── integration_tests.rs   # Comprehensive integration tests
├── benches/
│   ├── task_processing.rs     # Task processing benchmarks
│   └── queue_operations.rs    # Queue operation benchmarks
├── examples/
│   ├── performance_test.rs    # Performance testing examples
│   └── actix-integration/     # Full Actix Web integration
│       ├── src/
│       │   ├── main.rs        # Complete web server example
│       │   └── tasks.rs       # Task definitions
│       └── src/bin/           # Worker binaries
├── macros/                    # Procedural macros for auto-registration
├── Cargo.toml                 # Dependencies & metadata
├── README.md                  # User-facing documentation
├── DEVELOPMENT.md             # This development guide
├── CONFIGURATION.md           # Configuration documentation
└── docker-compose.yml         # Redis setup for development
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

### Configuration Validation

All configurations are automatically validated:

```rust
use rust_task_queue::config::TaskQueueConfig;
use rust_task_queue::autoscaler::AutoScalerConfig;

// Configuration validation is built-in
let config = TaskQueueConfig::default();
config.validate()?; // Comprehensive validation

let autoscaler_config = AutoScalerConfig::default();
autoscaler_config.validate()?; // Validates worker limits, thresholds, etc.
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
# Run all tests (requires Redis on localhost:6379)
cargo test

# Start Redis for testing
docker run -d --name redis-test -p 6379:6379 redis:7-alpine

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test integration_tests

# Run with debug logging
RUST_LOG=rust_task_queue=debug cargo test

# Run specific test
cargo test test_basic_task_execution

# Clean up test Redis
docker stop redis-test && docker rm redis-test
```

### Test Structure

Current integration tests cover:

```rust
// Example test task with queue constants
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
            "data": format!("Processed: {}", self.data),
            "queue": "test_queue"
        });
        
        Ok(response)
    }

    fn name(&self) -> &str { "test_task" }
    fn max_retries(&self) -> u32 { 3 }
    fn timeout_seconds(&self) -> u64 { 30 }
}

// Example test using queue constants
#[tokio::test]
async fn test_with_queue_constants() {
    let task_queue = setup_test_queue().await;
    let task = TestTask { data: "test".to_string(), should_fail: false };
    
    let task_id = task_queue.enqueue(task, queue_names::DEFAULT).await.unwrap();
    assert!(!task_id.to_string().is_empty());
}
```

### Test Coverage Areas

- ✅ **Basic Task Execution** with queue constants
- ✅ **Retry Mechanism** with proper error handling
- ✅ **Task Scheduling** with delay support
- ✅ **Auto-scaling Metrics** with validation
- ✅ **Queue Priorities** using predefined constants
- ✅ **Health Checks** with comprehensive status
- ✅ **Error Handling** without unsafe operations
- ✅ **Configuration Validation** with all edge cases
- ✅ **Connection Reliability** with centralized management

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

# Custom sample size for more accurate results
cargo bench -- --sample-size 1000

# With profiling output
cargo bench -- --profile-time
```

### Current Benchmark Results

| Operation | Time | Throughput | Notes |
|-----------|------|------------|-------|
| Task Serialization | ~40ns | 25M ops/sec | MessagePack encoding |
| Task Deserialization | ~34ns | 29M ops/sec | MessagePack decoding |
| Queue Config Lookup | ~40ns | 25M ops/sec | DashMap access |
| Get Queues by Priority | ~1.46µs | 685K ops/sec | Sorting overhead |
| AutoScaler Config Creation | ~651ps | 1.5B ops/sec | Struct initialization |
| Redis Connection Helper | ~2.1µs | 476K ops/sec | Connection pooling |

### Adding New Benchmarks

```rust
// benches/my_benchmark.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rust_task_queue::prelude::*;
use rust_task_queue::queue::queue_names;

fn bench_queue_operation(c: &mut Criterion) {
    c.bench_function("queue_operation_with_constants", |b| {
        b.iter(|| {
            let result = some_operation_with_queue_constants();
            black_box(result);
        })
    });
}

criterion_group!(benches, bench_queue_operation);
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

// Task trait with enhanced error handling
#[async_trait]
pub trait Task: Send + Sync + Serialize + for<'de> Deserialize<'de> + Debug {
    async fn execute(&self) -> TaskResult;
    fn name(&self) -> &str;
    fn max_retries(&self) -> u32 { 3 }
    fn timeout_seconds(&self) -> u64 { 300 }
}

// Comprehensive error types
#[derive(Error, Debug)]
pub enum TaskQueueError {
    Redis(#[from] redis::RedisError),
    Connection(String),
    TaskExecution(String),
    TaskNotFound(String),
    TaskTimeout { id: String, timeout_seconds: u64 },
    Configuration(String),
    // ... more error types
}

// Queue constants module
pub mod queue_names {
    pub const DEFAULT: &str = "default";
    pub const HIGH_PRIORITY: &str = "high_priority";
    pub const LOW_PRIORITY: &str = "low_priority";
}
```

### Key Methods

```rust
impl TaskQueue {
    // Creation with validation
    pub async fn new(redis_url: &str) -> Result<Self, TaskQueueError>;
    
    // Worker management with safety
    pub async fn start_workers(&self, count: usize) -> Result<(), TaskQueueError>;
    pub async fn stop_workers(&self);
    pub async fn worker_count(&self) -> usize;
    
    // Task operations with queue constants
    pub async fn enqueue<T: Task>(&self, task: T, queue: &str) -> Result<TaskId, TaskQueueError>;
    pub async fn schedule<T: Task>(&self, task: T, queue: &str, delay: Duration) -> Result<TaskId, TaskQueueError>;
    
    // Monitoring with comprehensive metrics
    pub async fn health_check(&self) -> Result<HealthStatus, TaskQueueError>;
    pub async fn get_metrics(&self) -> Result<TaskQueueMetrics, TaskQueueError>;
    
    // Scheduler with persistence
    pub async fn start_scheduler(&self) -> Result<(), TaskQueueError>;
    pub async fn stop_scheduler(&self);
}

// Enhanced broker with connection helper
impl RedisBroker {
    async fn get_conn(&self) -> Result<deadpool_redis::Connection, TaskQueueError>;
    // ... other methods use this helper
}
```

### Usage Examples

#### Basic Usage with Queue Constants

```rust
use rust_task_queue::prelude::*;
use rust_task_queue::queue::queue_names;
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
            "timestamp": chrono::Utc::now(),
            "recipient": self.to
        });
        
        Ok(response)
    }

    fn name(&self) -> &str {
        "email_task"
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let task_queue = TaskQueueBuilder::new("redis://localhost:6379")
        .auto_register_tasks()
        .initial_workers(2)
        .build()
        .await?;
    
    // Enqueue task using queue constants
    let email = EmailTask {
        to: "user@example.com".to_string(),
        subject: "Welcome!".to_string(),
        body: "Welcome to our service!".to_string(),
    };
    
    let task_id = task_queue.enqueue(email, queue_names::DEFAULT).await?;
    println!("Enqueued email task: {}", task_id);
    
    Ok(())
}
```

#### Auto-Registration with Safety

```rust
use rust_task_queue::prelude::*;
use rust_task_queue::queue::queue_names;

#[derive(Debug, Serialize, Deserialize, Default, AutoRegisterTask)]
struct ProcessDataTask {
    data: String,
}

#[async_trait::async_trait]
impl Task for ProcessDataTask {
    async fn execute(&self) -> TaskResult {
        // Process data with proper error handling
        if self.data.is_empty() {
            return Err("Empty data provided".into());
        }
        
        let result = format!("Processed: {}", self.data);
        Ok(serde_json::json!({"result": result, "status": "success"}))
    }

    fn name(&self) -> &str {
        "process_data"
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Tasks are automatically registered with validation!
    let task_queue = TaskQueueBuilder::new("redis://localhost:6379")
        .auto_register_tasks()
        .initial_workers(4)
        .build()
        .await?;
    
    let task = ProcessDataTask {
        data: "Hello, World!".to_string(),
    };
    
    // Use queue constants for type safety
    let task_id = task_queue.enqueue(task, queue_names::HIGH_PRIORITY).await?;
    println!("Task enqueued: {}", task_id);
    
    Ok(())
}
```

#### Monitoring and Health Checks

```rust
// Comprehensive health check
let health = task_queue.health_check().await?;
println!("System status: {}", health.status);

for (component, status) in health.components {
    println!("  {}: {} - {:?}", component, status.status, status.message);
}

// Detailed metrics
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

- **Serialization**: 25M+ ops/sec (40ns per task) with MessagePack
- **Deserialization**: 29M+ ops/sec (34ns per task)
- **Queue Operations**: 25M+ ops/sec for config lookups
- **Connection Management**: 476K+ ops/sec with pooling
- **Overall throughput**: Thousands of tasks per second in production

### Memory Usage

- **Minimal overhead**: MessagePack serialization is compact
- **Connection pooling**: Configurable Redis connections
- **Worker memory**: Isolated task execution with proper cleanup
- **Queue constants**: Zero-cost abstractions

### Scaling Characteristics

- **Horizontal scaling**: Add more workers or worker processes
- **Auto-scaling**: Based on queue depth with validation
- **Redis scaling**: Single Redis instance or cluster support
- **Monitoring**: Real-time metrics without performance impact

### Optimization Tips

1. **Connection Pool Size**: Match to worker count
2. **Batch Operations**: Group related tasks when possible
3. **Queue Priorities**: Use appropriate queue constants
4. **Monitoring**: Regular health checks without overhead
5. **Error Handling**: Proper retry strategies without panics
6. **Configuration**: Use validation to catch issues early

## 🔧 Recent Improvements

### Safety Enhancements

1. **Eliminated `unwrap()` calls**: All potentially unsafe operations now use proper error handling
2. **Redis connection helper**: Centralized connection management reduces code duplication by 50+ lines
3. **Configuration validation**: Comprehensive validation prevents runtime errors
4. **Queue constants**: Type-safe queue names prevent typos and inconsistencies

### Performance Improvements

1. **Connection pooling optimization**: Better Redis connection management
2. **MessagePack serialization**: Faster and more compact than JSON
3. **Error handling optimization**: Reduced overhead in error paths
4. **Memory management**: Better cleanup and resource management

### API Enhancements

1. **Queue constants module**: `queue_names::DEFAULT`, `queue_names::HIGH_PRIORITY`, etc.
2. **Builder pattern improvements**: More intuitive configuration
3. **Better error messages**: More descriptive error information
4. **Enhanced monitoring**: More detailed metrics and health checks

## 🤝 Contributing

### Development Workflow

1. **Fork** the repository
2. **Create** a feature branch: `git checkout -b feature/my-feature`
3. **Make** your changes following the coding standards
4. **Add** tests for new functionality
5. **Run** tests: `cargo test`
6. **Run** clippy: `cargo clippy --all-targets --all-features`
7. **Run** formatting: `cargo fmt`
8. **Run** benchmarks if performance-critical: `cargo bench`
9. **Commit** your changes with clear messages
10. **Push** to the branch: `git push origin feature/my-feature`
11. **Create** a Pull Request with detailed description

### Code Standards

- **Rust 2021 Edition**
- **Clippy clean**: No warnings allowed
- **Formatted**: Use `cargo fmt`
- **Documented**: Public APIs must have comprehensive docs
- **Tested**: New features need comprehensive tests
- **Benchmarked**: Performance-critical code needs benchmarks
- **Safe**: No `unwrap()`, `expect()`, or `panic!()` in production code
- **Type-safe**: Use queue constants and proper error types

### Adding New Features

1. **Design**: Consider the API design and safety implications
2. **Implement**: Add the feature with proper error handling
3. **Test**: Add comprehensive tests including edge cases
4. **Document**: Update documentation and examples
5. **Benchmark**: Add benchmarks if performance-critical
6. **Validate**: Ensure configuration validation if applicable

### Debugging Tips

```bash
# Enable debug logging
RUST_LOG=rust_task_queue=debug cargo test

# Enable trace-level logging for detailed output
RUST_LOG=rust_task_queue=trace cargo test

# Run specific test with output
cargo test test_name -- --nocapture

# Debug Redis operations
redis-cli monitor

# Check Redis keys and data
redis-cli keys "*"
redis-cli smembers active_workers

# Debug worker activity
RUST_LOG=rust_task_queue::worker=debug cargo run --bin task-worker --features cli worker
```

## 🔧 Troubleshooting

### Common Issues

#### Redis Connection Issues

```bash
# Check Redis is running
redis-cli ping

# Check connection string format
export REDIS_URL="redis://127.0.0.1:6379"

# Debug connection with detailed logging
RUST_LOG=redis=debug,rust_task_queue=debug cargo test

# Check Redis version (6.0+ required)
redis-cli info server
```

#### Worker Issues

```bash
# Check worker registration
redis-cli smembers active_workers

# Check worker heartbeats
redis-cli keys "worker:*:heartbeat"

# Monitor worker activity with logging
RUST_LOG=rust_task_queue::worker=debug cargo test

# Check for failed tasks
redis-cli smembers "queue:default:failed_tasks"
```

#### Configuration Issues

```bash
# Validate configuration
RUST_LOG=rust_task_queue::config=debug cargo test

# Check environment variables
env | grep REDIS
env | grep TASK_QUEUE

# Test configuration loading
cargo run --example performance_test -- --help
```

#### Performance Issues

```bash
# Run benchmarks to establish baseline
cargo bench

# Profile with perf (Linux) or instruments (macOS)
cargo build --release

# Check Redis performance
redis-cli --latency-history

# Monitor Redis memory usage
redis-cli info memory
```

#### Queue Constant Issues

```bash
# Verify queue constants are being used
grep -r "queue_names::" src/
grep -r "\"default\"" src/  # Should be minimal

# Check for hardcoded queue names in examples
grep -r "\"default\"\|\"high_priority\"" examples/
```

### Debugging Checklist

1. ✅ **Redis is running and accessible**
2. ✅ **Correct Redis URL format**
3. ✅ **All dependencies are up to date**
4. ✅ **Environment variables are set correctly**
5. ✅ **Tasks are properly registered**
6. ✅ **Queue constants are used instead of hardcoded strings**
7. ✅ **Configuration is valid**
8. ✅ **No `unwrap()` or `panic!()` in production code**

### Getting Help

1. **Check logs**: Enable debug logging first
2. **Review documentation**: API docs and examples
3. **Run tests**: Ensure basic functionality works
4. **Check Redis**: Verify Redis is working correctly
5. **Search issues**: Look for similar problems in the repository
6. **Create minimal reproduction**: Isolate the problem

For more detailed troubleshooting, see the main [README.md](README.md) or create an issue with:
- Rust version
- Redis version
- Complete error messages
- Minimal reproduction code
- Environment details

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