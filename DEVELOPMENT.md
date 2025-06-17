# Rust Task Queue - Development Guide

A high-performance, Redis-backed task queue framework with **Enhanced Auto-scaling**, intelligent async task spawning, multi-dimensional scaling triggers, and advanced backpressure management designed for async Rust applications.

## Table of Contents

- [Overview](#overview)
- [Enhanced Auto-scaling Architecture](#enhanced-auto-scaling-architecture)
- [Architecture](#architecture)
- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [Configuration](#configuration)
- [Testing](#testing)
- [Benchmarking](#benchmarking)
- [Code Quality](#code-quality)
- [API Reference](#api-reference)
- [Performance Characteristics](#performance-characteristics)
- [Recent Improvements](#recent-improvements)
- [Contributing](#contributing)
- [Troubleshooting](#troubleshooting)

## Overview

### Key Features

- **Redis-backed broker** with connection pooling and optimized operations
- **Enhanced Multi-dimensional Auto-scaling** with 5-metric analysis and adaptive learning
- **Task scheduling** with delay support and persistent scheduling
- **Multiple queue priorities** with predefined queue constants for type safety
- **Retry logic** with exponential backoff and configurable attempts
- **Task timeouts** and comprehensive failure handling
- **Advanced metrics and monitoring** with SLA tracking and performance insights
- **Actix Web integration** (optional) with built-in endpoints
- **Automatic task registration** via procedural macros
- **Comprehensive error handling** with eliminated `unwrap()` calls
- **Connection pooling** for optimal Redis performance with centralized connection management
- **Configuration validation** with comprehensive safety checks
- **Intelligent async task spawning** with context-based execution and proper resource management
- **Advanced backpressure management** with automatic task re-queuing and capacity control
- **Active task tracking** with atomic counters for real-time monitoring
- **Graceful shutdown** with active task completion waiting

### Performance Highlights

- **Task Serialization**: ~39.15ns per operation (MessagePack)
- **Task Deserialization**: ~31.51ns per operation
- **Queue Config Lookup**: ~39.76ns per operation
- **Queue Management**: ~1.38µs per operation
- **Enhanced AutoScaler Config Creation**: ~617ps per operation
- **High throughput**: Thousands of tasks per second
- **Memory efficient**: Optimized serialization and connection pooling
- **Smart concurrency**: Atomic task tracking with minimal overhead
- **Efficient spawning**: Context-based execution reduces resource allocation
- **Intelligent backpressure**: Task re-queuing prevents system overload

### Recent Improvements

- **Comprehensive Test Suite**: 162 total tests across all categories
  - 121 unit tests covering core functionality
  - 9 integration tests for end-to-end workflows
  - 9 error scenario tests for edge cases
  - 6 performance tests for load handling
  - 7 security tests for injection protection
  - 5 benchmark tests for performance tracking

- **Enhanced Auto-scaling**:
  - Multi-dimensional scaling with 5 simultaneous metrics
  - Adaptive threshold learning based on SLA targets
  - Advanced hysteresis and stability controls
  - Performance history analysis and trend monitoring
  - Consecutive signal requirements and intelligent cooldowns

- **Code Quality Improvements**: 
  - Zero clippy warnings with strict linting
  - Enhanced error handling with proper TaskQueueError creation
  - Eliminated private method access issues
  - Streamlined Default trait implementations

- **Performance Optimizations**:
  - Sub-40ns serialization/deserialization
  - Improved queue operation efficiency
  - Optimized configuration handling

- **Development Experience**:
  - Automated test script with Redis container management
  - Comprehensive benchmark suite
  - Enhanced development documentation
  - CI/CD ready test infrastructure

## Enhanced Auto-scaling Architecture

### Multi-dimensional Intelligence

The enhanced auto-scaling system represents a significant evolution from simple threshold-based scaling to intelligent, multi-dimensional decision making.

#### **Five-Metric Analysis System**

```
┌────────────────────────────────────────────────────────────────────┐
│                    Multi-dimensional Scaling Engine                │
├────────────────────────────────────────────────────────────────────┤
│ 1. Queue Pressure Score    │ Weighted queue depth by priority      │
│ 2. Worker Utilization      │ Real-time busy/idle ratio analysis    │
│ 3. Task Complexity Factor  │ Dynamic execution pattern recognition │
│ 4. Error Rate Monitoring   │ System health and stability tracking  │
│ 5. Memory Pressure         │ Per-worker resource utilization       │
└────────────────────────────────────────────────────────────────────┘
```

#### **Adaptive Threshold Learning**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SLATargets {
    pub max_p95_latency_ms: f64,        // 3000.0ms target
    pub min_success_rate: f64,          // 99% success rate
    pub max_queue_wait_time_ms: f64,    // 5000.0ms max wait
    pub target_worker_utilization: f64, // 75% optimal utilization
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingTriggers {
    pub queue_pressure_threshold: f64,      // 1.2 weighted queue depth
    pub worker_utilization_threshold: f64,  // 85% utilization trigger
    pub task_complexity_threshold: f64,     // 2.0 complexity factor
    pub error_rate_threshold: f64,          // 3% error rate limit
    pub memory_pressure_threshold: f64,     // 1024MB per worker
}
```

#### **Stability and Hysteresis Controls**

```
Scaling Decision Pipeline:
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  Multi-metric   │───►│ Consecutive     │───►│ Cooldown        │
│  Analysis       │    │ Signal Check    │    │ Period Check    │
│  (5 metrics)    │    │ (2-5 signals)   │    │ (3-15 minutes)  │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
   Scale Decision         Signal History          Final Action
   (Up/Down/None)        Buffer & Count         (Execute/Skip)
```

#### **Configuration Structure**

```toml
[autoscaler]
# Basic scaling parameters
min_workers = 5
max_workers = 50
scale_up_count = 2
scale_down_count = 1

# Multi-dimensional triggers
[autoscaler.scaling_triggers]
queue_pressure_threshold = 1.2
worker_utilization_threshold = 0.85
task_complexity_threshold = 2.0
error_rate_threshold = 0.03
memory_pressure_threshold = 1024.0

# Adaptive learning (SLA-driven)
enable_adaptive_thresholds = true
learning_rate = 0.05
adaptation_window_minutes = 60

# Stability controls
scale_up_cooldown_seconds = 180     # 3 minutes
scale_down_cooldown_seconds = 900   # 15 minutes
consecutive_signals_required = 3

# Performance targets
[autoscaler.target_sla]
max_p95_latency_ms = 3000.0
min_success_rate = 0.99
max_queue_wait_time_ms = 5000.0
target_worker_utilization = 0.75
```

#### **API Changes and Usage**

**Enhanced AutoScaler Constructor:**

```rust
// New API requires broker parameter for metrics collection
let autoscaler = AutoScaler::with_config(broker.clone(), enhanced_config);

// Configuration structure uses explicit field initialization
let config = AutoScalerConfig {
    min_workers: 2,
    max_workers: 20,
    scaling_triggers: ScalingTriggers {
        queue_pressure_threshold: 1.5,
        worker_utilization_threshold: 0.80,
        task_complexity_threshold: 2.0,
        error_rate_threshold: 0.05,
        memory_pressure_threshold: 512.0,
    },
    target_sla: SLATargets {
        max_p95_latency_ms: 5000.0,
        min_success_rate: 0.95,
        max_queue_wait_time_ms: 10000.0,
        target_worker_utilization: 0.70,
    },
    // ... other fields
};
```

**Breaking Changes from Simple Scaling:**

- Old: `scale_up_threshold` and `scale_down_threshold` (removed)
- New: `scaling_triggers` with 5-dimensional analysis
- Old: `AutoScaler::new()` without broker parameter  
- New: `AutoScaler::with_config(broker, config)` required
- Old: `ScalingTriggers::default()` calls
- New: Explicit struct initialization required

## Architecture

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
┌─────────────────┐    ┌─────────────────┐    ┌────────────────────┐
│   Worker Pool   │    │   Redis Pool    │    │Enhanced AutoScaler │
│                 │    │                 │    │                    │
│ • Smart Spawn   │    │ • Connections   │    │ • 5-Metric Analysis│
│ • Backpressure  │    │ • Health Check  │    │ • Adaptive Learning│
│ • Active Track  │    │ • Centralized   │    │ • SLA Monitoring   │
│ • Graceful Stop │    │ • Pool Mgmt     │    │ • Stability Control│
└─────────────────┘    └─────────────────┘    └────────────────────┘
```

### Enhanced Auto-scaling Data Flow

1. **Enhanced Metrics Collection**: 5-dimensional real-time analysis
2. **Multi-signal Analysis**: Simultaneous evaluation of all scaling factors
3. **Adaptive Threshold Check**: SLA-driven threshold adjustments
4. **Consecutive Signal Validation**: Stability through signal consistency
5. **Cooldown Period Enforcement**: Prevent oscillation with time-based controls
6. **Scaling Decision Execution**: Intelligent worker count adjustments
7. **Performance History Recording**: Learning from scaling outcomes
8. **SLA Compliance Monitoring**: Continuous performance vs. target analysis

### Data Flow

1. **Task Enqueue**: Tasks are serialized (MessagePack) and pushed to Redis queues using queue constants
2. **Worker Dequeue**: Workers pull tasks from priority-ordered queues with robust error handling
3. **Capacity Check**: Intelligent semaphore-based concurrency control prevents worker overload
4. **Smart Spawning**: Tasks are spawned asynchronously or re-queued based on available capacity
5. **Task Execution**: Tasks are executed with context-based tracking, timeout and retry logic
6. **Active Monitoring**: Real-time task tracking with atomic counters for observability
7. **Result Handling**: Success/failure is tracked with comprehensive metrics and proper cleanup
8. **Enhanced Auto-scaling**: System monitors 5 metrics and adjusts worker count with SLA optimization
9. **Graceful Shutdown**: Workers wait for active tasks before terminating
10. **Connection Management**: Centralized Redis connection handling for reliability

## Configuration

### Enhanced Auto-scaling Configuration

The enhanced auto-scaling introduces a sophisticated configuration structure:

```toml
[autoscaler]
enabled = true
min_workers = 5
max_workers = 50
scale_up_count = 2           # workers to add when scaling up
scale_down_count = 1         # workers to remove when scaling down

# Multi-dimensional scaling triggers (Enhancement)
[autoscaler.scaling_triggers]
queue_pressure_threshold = 1.2        # weighted queue depth per worker
worker_utilization_threshold = 0.85   # target worker utilization (85%)
task_complexity_threshold = 1.5       # complex task overload factor
error_rate_threshold = 0.05           # maximum 5% error rate
memory_pressure_threshold = 512.0    # memory usage per worker (MB)

# Adaptive threshold learning (SLA-driven optimization)
enable_adaptive_thresholds = true
learning_rate = 0.1
adaptation_window_minutes = 30
scale_up_cooldown_seconds = 60
scale_down_cooldown_seconds = 300
consecutive_signals_required = 2

# SLA performance targets for adaptive learning
[autoscaler.target_sla]
max_p95_latency_ms = 5000.0
min_success_rate = 0.95
max_queue_wait_time_ms = 10000.0
target_worker_utilization = 0.70
```

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

# Enhanced auto-scaling configuration
[autoscaler]
enabled = true
min_workers = 1
max_workers = 20
scale_up_count = 2
scale_down_count = 1

[autoscaler.scaling_triggers]
queue_pressure_threshold = 0.75
worker_utilization_threshold = 0.80
task_complexity_threshold = 1.5
error_rate_threshold = 0.05
memory_pressure_threshold = 512.0

enable_adaptive_thresholds = true
learning_rate = 0.1
adaptation_window_minutes = 30
scale_up_cooldown_seconds = 60
scale_down_cooldown_seconds = 300
consecutive_signals_required = 2

[autoscaler.target_sla]
max_p95_latency_ms = 5000.0
min_success_rate = 0.95
max_queue_wait_time_ms = 10000.0
target_worker_utilization = 0.70

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

All configurations are automatically validated with enhanced auto-scaling validation:

```rust
use rust_task_queue::config::TaskQueueConfig;
use rust_task_queue::{AutoScalerConfig, ScalingTriggers, SLATargets};

// Enhanced auto-scaling configuration validation
let autoscaler_config = AutoScalerConfig {
    min_workers: 2,
    max_workers: 50,
    scaling_triggers: ScalingTriggers {
        queue_pressure_threshold: 1.5,
        worker_utilization_threshold: 0.80,
        task_complexity_threshold: 2.0,
        error_rate_threshold: 0.05,
        memory_pressure_threshold: 512.0,
    },
    target_sla: SLATargets {
        max_p95_latency_ms: 5000.0,
        min_success_rate: 0.95,
        max_queue_wait_time_ms: 10000.0,
        target_worker_utilization: 0.70,
    },
    // ... other fields
};

// Comprehensive validation is built-in
autoscaler_config.validate()?; // Validates all thresholds, limits, and ranges
```

### Enhanced Builder Pattern

```rust
let task_queue = TaskQueueBuilder::new("redis://localhost:6379")
    .initial_workers(4)
    .auto_register_tasks()
    .with_scheduler()
    .with_autoscaler_config(enhanced_autoscaler_config)  // Use enhanced configuration
    .build()
    .await?;
```

### Queue Constants

The framework provides predefined queue constants for type safety:

```rust
use rust_task_queue::queue::queue_names;

// Available queue constants
queue_names::DEFAULT        // "default" - Standard priority tasks
queue_names::HIGH_PRIORITY  // "high_priority" - High priority tasks  
queue_names::LOW_PRIORITY   // "low_priority" - Background tasks
```

### Enhanced Worker Architecture

#### **🔧 Intelligent Task Spawning System**

The framework features a sophisticated async task spawning architecture designed for high-performance, reliable task processing:

##### **Core Components:**

1. **TaskExecutionContext**: Centralized context containing all necessary resources
   ```rust
   struct TaskExecutionContext {
       broker: Arc<RedisBroker>,
       task_registry: Arc<TaskRegistry>,
       worker_id: String,
       semaphore: Option<Arc<Semaphore>>,
       active_tasks: Arc<AtomicUsize>,
   }
   ```

2. **SpawnResult Enum**: Tracks task spawning outcomes
   ```rust
   enum SpawnResult {
       Spawned,              // Task successfully spawned
       Rejected(TaskWrapper), // Task rejected due to capacity
       Failed(TaskQueueError), // Failed to spawn task
   }
   ```

3. **Atomic Task Tracking**: Real-time monitoring with `AtomicUsize` counters
   ```rust
   // Monitor active tasks in real-time
   let active_count = worker.active_task_count();
   println!("Worker has {} active tasks", active_count);
   ```

##### **Execution Flow:**

```
Task Dequeued → Capacity Check → Decision Point
                                        ↓
              ┌─────────────────────────┼─────────────────────────┐
              ▼                         ▼                         ▼
        Has Capacity              At Capacity                No Semaphore
              ↓                         ↓                         ↓
      Spawn Async Task            Re-queue Task             Execute Direct
              ↓                         ↓                         ↓
      Track + Execute           Apply Backpressure          Track + Execute
              ↓                         ↓                         ↓
      Cleanup & Release           Delay & Retry                Cleanup
```

##### **Advanced Features:**

- **Backpressure Management**: Automatic task re-queuing when at capacity
- **Resource Safety**: RAII patterns ensure proper cleanup
- **Graceful Shutdown**: Workers wait up to 30 seconds for active tasks
- **Context-based Spawning**: Centralized resource management
- **Intelligent Delays**: Configurable delays prevent tight loops

##### **Configuration Options:**

```rust
// Basic worker with concurrency limits
let worker = Worker::new("worker-1".to_string(), broker, scheduler)
    .with_max_concurrent_tasks(10)
    .with_task_registry(registry);

// Advanced backpressure configuration
let backpressure_config = WorkerBackpressureConfig {
    max_concurrent_tasks: 10,
    queue_size_threshold: 100,
    backpressure_delay_ms: 50,
};

let worker = Worker::new("worker-advanced".to_string(), broker, scheduler)
    .with_backpressure_config(backpressure_config)
    .with_task_registry(registry);
```

##### **Development Benefits:**

- **Consistent Architecture**: Structured approach to task spawning
- **Resource Safety**: Eliminates resource leaks and improper cleanup
- **Testability**: Individual components can be tested in isolation
- **Observability**: Real-time metrics for debugging and monitoring
- **Reliability**: Proper error handling throughout the execution pipeline

## Development Setup

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
# Use automated test script (recommended)
./scripts/run-tests.sh

# Or run tests manually
cargo test

# Run integration tests specifically
cargo test --test integration_tests

# Run benchmarks
cargo bench

# Build documentation
cargo doc --open

# Start worker CLI for testing with enhanced auto-scaling
cargo run --bin task-worker --features cli,auto-register worker \
  --workers 2 --enable-autoscaler
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

# Enhanced Auto-scaling Configuration
export TASK_QUEUE_AUTOSCALER_ENABLED=true
export TASK_QUEUE_AUTOSCALER_MIN_WORKERS=2
export TASK_QUEUE_AUTOSCALER_MAX_WORKERS=20
export TASK_QUEUE_ADAPTIVE_THRESHOLDS=true

# Development/Testing
export REDIS_TEST_URL="redis://127.0.0.1:6379/15"
export RUST_LOG=rust_task_queue=debug
```

## Project Structure

```
rust-task-queue/
├── src/
│   ├── lib.rs                 # Main library entry point & exports
│   ├── config.rs              # Configuration management with validation
│   ├── error.rs               # Error types & comprehensive handling
│   ├── broker.rs              # Redis broker with connection helper
│   ├── worker.rs              # Enhanced worker with intelligent task spawning & backpressure
│   ├── task.rs                # Task trait & registry with auto-register
│   ├── scheduler.rs           # Task scheduling with persistence
│   ├── autoscaler.rs          # Enhanced multi-dimensional auto-scaling
│   ├── queue.rs               # Queue constants & configuration
│   ├── actix.rs               # Actix Web integration with endpoints
│   ├── cli.rs                 # CLI utilities with full feature support
│   ├── metrics.rs             # Metrics collection and SLA monitoring
│   ├── prelude.rs             # Common imports for convenience
│   └── bin/
│       ├── task-worker.rs     # Main worker binary with CLI
│       └── task-worker-env-only.rs # Environment-only worker
├── tests/
│   ├── integration_tests.rs   # Comprehensive integration tests (9 tests)
│   ├── error_scenarios_tests.rs     # Error handling tests (9 tests)
│   ├── performance_tests.rs   # Performance tests (6 tests)
│   └── security_tests.rs      # Security tests (7 tests)
├── benches/
│   ├── task_processing.rs     # Task processing benchmarks
│   ├── queue_operations.rs    # Queue operation benchmarks
│   ├── autoscaler.rs          # Enhanced auto-scaling benchmarks
│   └── serialization.rs       # MessagePack serialization benchmarks
├── examples/
│   ├── performance_test.rs    # Performance testing examples
│   └── actix-integration/     # Full Actix Web integration
│       ├── src/
│       │   ├── main.rs        # Complete web server example
│       │   └── tasks.rs       # Task definitions
│       └── src/bin/           # Worker binaries
├── macros/                    # Procedural macros for auto-registration
├── scripts/                   # Development and testing scripts
│   ├── run-tests.sh          # Comprehensive automated test suite
│   ├── cleanup-redis.sh      # Redis container cleanup
│   └── run-benchmarks-with-reports.sh # Benchmark execution and reporting
├── Cargo.toml                 # Dependencies & metadata
├── README.md                  # User-facing documentation
├── DEVELOPMENT.md             # This development guide
└── docker-compose.yml         # Redis setup for development
```

## Testing

### Comprehensive Test Suite

The project maintains a comprehensive test suite with **162 total tests** across multiple categories:

```bash
# Automated test script (recommended) - handles Redis setup/cleanup
./scripts/run-tests.sh

# If tests fail and leave containers running, clean up with:
./scripts/cleanup-redis.sh

# Individual test suites:
cargo test --lib                    # Unit tests (121 tests)
cargo test --test integration_tests # Integration tests (9 tests)
cargo test --test error_scenarios_tests   # Error handling tests (9 tests)
cargo test --test performance_tests # Performance tests (6 tests)
cargo test --test security_tests    # Security tests (7 tests)
cargo bench                         # Benchmark tests (5 benchmarks)
```

### Test Categories

#### **Unit Tests (121 tests)**
Covers all core functionality including:
- Broker operations and connection handling
- Worker lifecycle and task spawning
- Configuration validation and builder patterns
- Queue management and priority handling
- Auto-scaler logic and metrics collection
- Error handling and type conversions
- Task registry and automatic registration

#### **Integration Tests (9 tests)**
End-to-end workflow testing:
- `test_basic_task_execution` - Complete task lifecycle
- `test_task_retry_mechanism` - Failure and retry handling
- `test_scheduler` - Scheduled task execution
- `test_autoscaler_metrics` - Auto-scaling functionality
- `test_queue_priorities` - Priority queue handling
- `test_integration_comprehensive` - Full system integration
- `test_improved_async_task_spawning` - Advanced worker features
- `test_backpressure_handling` - Capacity management
- `test_graceful_shutdown_with_active_tasks` - Clean shutdown

#### **Error Scenario Tests (9 tests)**
Edge cases and failure modes:
- Redis connection failure recovery
- Malformed task data handling
- Task timeout scenarios
- Memory usage and leak prevention
- Worker panic recovery
- Configuration validation edge cases
- Queue name validation
- Redis pool exhaustion
- Concurrent worker scaling edge cases

#### **Performance Tests (6 tests)**
Load and throughput validation:
- High throughput task processing
- Concurrent task safety
- CPU intensive workload handling
- Memory intensive workload handling
- Queue priority performance
- Auto-scaler performance under load

#### **Security Tests (7 tests)**
Safety and injection protection:
- Redis connection string injection
- Queue name injection attacks
- Malicious payload handling
- Task deserialization bomb prevention
- Oversized task handling
- Configuration tampering protection
- Concurrent access safety

### Test Infrastructure

The project includes robust testing infrastructure with improved container management:

#### **Automated Testing Scripts**

```bash
# Primary test script (recommended)
./scripts/run-tests.sh                      # Comprehensive test suite with automatic cleanup

# Cleanup script for recovery
./scripts/cleanup-redis.sh                  # Removes leftover containers from failed runs

# Manual Redis setup for development
docker run -d --name redis-test -p 6379:6379 redis:7-alpine

# Run with debug logging
RUST_LOG=rust_task_queue=debug cargo test

# Run specific test
cargo test test_basic_task_execution

# Clean up test Redis
docker stop redis-test && docker rm redis-test
```

#### **Improved Container Management**

The `scripts/run-tests.sh` script now includes:

- **Trap Handlers**: Automatic cleanup on script exit (success, failure, or interruption)
- **Port Checking**: Detects and handles existing containers on the target port
- **Container Recovery**: Automatically removes leftover containers from previous runs
- **Error Handling**: Continues with remaining tests even if some fail
- **Colored Output**: Enhanced visual feedback with status indicators
- **Comprehensive Logging**: Detailed progress and error reporting

#### **Container Cleanup Features**

- **EXIT Traps**: Ensure cleanup happens regardless of how script exits
- **Container Detection**: Check for existing containers before starting
- **Port Conflict Resolution**: Handle port conflicts gracefully
- **Force Removal**: Remove stuck containers with force flags
- **Status Reporting**: Clear feedback on cleanup operations

### Test Isolation

- Each test uses separate Redis databases (0-15)
- Automatic cleanup prevents test interference
- Comprehensive setup/teardown in test scripts
- Concurrent test execution safety
- Robust container management with trap handlers
- Automatic detection and cleanup of leftover containers
- Dedicated cleanup script for manual intervention

## Code Quality

### Linting and Formatting

The project maintains strict code quality standards:

```bash
# Clippy linting (zero warnings enforced)
cargo clippy --all-targets --all-features -- -D warnings

# Code formatting
cargo fmt

# Check compilation without building
cargo check --all-targets --all-features
```

### Quality Standards

- **Zero Clippy Warnings**: All code passes strict linting rules
- **Consistent Formatting**: Automated formatting with rustfmt
- **Comprehensive Error Handling**: No `unwrap()` calls in production code
- **Type Safety**: Leverages Rust's type system for correctness
- **Documentation**: All public APIs are documented
- **Testing**: High test coverage across all components

### Recent Quality Improvements

- **Enhanced Error Handling**: Proper TaskQueueError creation patterns
- **Eliminated Private Method Access**: Proper encapsulation with test helpers
- **Streamlined Implementations**: Derive macros for Default traits
- **Removed Unnecessary Casts**: Type-safe operations throughout
- **Dead Code Cleanup**: Proper annotations for test utilities

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

- **Basic Task Execution** with queue constants
- **Retry Mechanism** with proper error handling
- **Task Scheduling** with delay support
- **Auto-scaling Metrics** with validation
- **Queue Priorities** using predefined constants
- **Health Checks** with comprehensive status
- **Error Handling** without unsafe operations
- **Configuration Validation** with all edge cases
- **Connection Reliability** with centralized management

## Benchmarking

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

| Operation | Time | Performance Level | Notes |
|-----------|------|-----------|---------------|
| Task Serialization | ~39.15 ns | Excellent | MessagePack encoding |
| Task Deserialization | ~31.51 ns | Excellent | MessagePack decoding |
| Queue Config Lookup | ~39.76 ns | Excellent | DashMap access |
| Queue Management | ~1.38 µs | Very Good | Priority sorting |
| AutoScaler Config | ~617 ps | Outstanding | Struct creation |

### Benchmark Categories

#### **Task Processing Benchmarks** (`task_processing.rs`)
- `task_serialization`: MessagePack encoding performance
- `task_deserialization`: MessagePack decoding performance

#### **Queue Operations Benchmarks** (`queue_operations.rs`)
- `queue_manager_get_queues`: Queue retrieval by priority
- `queue_manager_get_queue_config`: Configuration lookup
- `autoscaler_config_creation`: Configuration object creation

### Performance Tracking

Benchmarks are tracked over time to detect performance regressions and improvements.

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

## API Reference

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

## Performance Characteristics

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
7. **Concurrency Limits**: Set `max_concurrent_tasks` based on resource capacity
8. **Backpressure Delays**: Configure appropriate delays to prevent tight loops
9. **Active Task Monitoring**: Use `active_task_count()` for real-time insights
10. **Graceful Shutdown**: Allow sufficient time for task completion (30s default)
11. **Context Reuse**: Leverage TaskExecutionContext for efficient resource management
12. **Semaphore Configuration**: Match semaphore size to system capacity

## Recent Improvements

### Worker Architecture Overhaul

1. **Intelligent Task Spawning**: Complete redesign of async task execution with context-based spawning
2. **Advanced Backpressure Management**: Automatic task re-queuing and capacity-aware processing
3. **Active Task Tracking**: Real-time monitoring with atomic counters for precise observability
4. **Graceful Shutdown**: Workers wait for active tasks to complete before terminating
5. **Resource Safety**: Proper RAII patterns and cleanup throughout the execution lifecycle
6. **Semaphore-based Concurrency**: Intelligent capacity management prevents system overload

### Safety Enhancements

1. **Eliminated `unwrap()` calls**: All potentially unsafe operations now use proper error handling
2. **Redis connection helper**: Centralized connection management reduces code duplication by 50+ lines
3. **Configuration validation**: Comprehensive validation prevents runtime errors
4. **Queue constants**: Type-safe queue names prevent typos and inconsistencies
5. **Borrow checker compliance**: Resolved all lifetime and borrowing issues in worker spawning
6. **Context-based execution**: Centralized resource management eliminates resource leaks

### Performance Improvements

1. **Connection pooling optimization**: Better Redis connection management
2. **MessagePack serialization**: Faster and more compact than JSON
3. **Error handling optimization**: Reduced overhead in error paths
4. **Memory management**: Better cleanup and resource management
5. **Atomic task tracking**: Minimal overhead for real-time monitoring
6. **Efficient spawning**: Context reuse reduces allocation overhead

### API Enhancements

1. **Queue constants module**: `queue_names::DEFAULT`, `queue_names::HIGH_PRIORITY`, etc.
2. **Builder pattern improvements**: More intuitive configuration
3. **Better error messages**: More descriptive error information
4. **Enhanced monitoring**: More detailed metrics and health checks
5. **Worker configuration**: Flexible backpressure and concurrency settings
6. **Real-time metrics**: Active task count and capacity monitoring

## Contributing

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

# Debug worker activity with spawning details
RUST_LOG=rust_task_queue::worker=debug cargo run --bin task-worker --features cli worker

# Monitor task spawning and backpressure
RUST_LOG=rust_task_queue::worker=trace cargo test test_improved_async_task_spawning -- --nocapture

# Track active task counts
RUST_LOG=rust_task_queue=debug cargo test test_graceful_shutdown_with_active_tasks -- --nocapture
```

## Troubleshooting

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

#### Worker Spawning and Backpressure Issues

```bash
# Monitor task spawning behavior
RUST_LOG=rust_task_queue::worker=trace cargo test -- --nocapture

# Check semaphore and capacity limits
cargo test test_improved_async_task_spawning -- --nocapture

# Verify graceful shutdown behavior
cargo test test_graceful_shutdown_with_active_tasks -- --nocapture

# Test backpressure handling
cargo test test_backpressure_handling -- --nocapture

# Monitor active task counts in real-time
RUST_LOG=debug cargo run --example worker_monitoring
```

#### Active Task Tracking Issues

```bash
# Check if active task counts are accurate
cargo test --test integration_tests -- test_active_task_count --nocapture

# Verify task cleanup after completion
RUST_LOG=rust_task_queue::worker=debug cargo test -- --nocapture | grep -i "cleanup\|active"

# Monitor for task leaks
watch -n 1 'redis-cli eval "return redis.call(\"keys\", \"task:*\")" 0'
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

1. **Redis is running and accessible**
2. **Correct Redis URL format**
3. **All dependencies are up to date**
4. **Environment variables are set correctly**
5. **Tasks are properly registered**
6. **Queue constants are used instead of hardcoded strings**
7. **Configuration is valid**
8. **No `unwrap()` or `panic!()` in production code**
9. **Worker concurrency limits are properly configured**
10. **Active task counts are being tracked correctly**
11. **Backpressure management is functioning as expected**
12. **Graceful shutdown completes within timeout**
13. **Task spawning context is properly initialized**
14. **Semaphore permits are correctly managed**

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

## License

This project is licensed under the MIT OR Apache-2.0 license.

## What's Next?

- [ ] **Distributed Mode**: Multi-Redis support
- [ ] **Web UI**: Task monitoring dashboard  
- [ ] **More Integrations**: Axum, Warp, etc.
- [ ] **Batch Processing**: Bulk task operations
- [ ] **Dead Letter Queues**: Failed task handling
- [ ] **Task Dependencies**: Workflow support
- [ ] **Metrics Export**: Prometheus integration
- [ ] **Security**: Authentication & authorization

---

Happy coding!