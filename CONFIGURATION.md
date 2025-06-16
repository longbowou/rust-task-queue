# Configuration Guide

The `rust-task-queue` crate provides a powerful and flexible configuration system that allows you to configure Redis URL, auto-scaling, auto-registration, and other settings through configuration files or environment variables.

## Quick Start

The easiest way to get started is with auto-configuration:

```rust
use rust_task_queue::prelude::*;

#[tokio::main]
async fn main() -> Result<(), TaskQueueError> {
    // This will automatically load configuration from files or environment
    let task_queue = TaskQueueBuilder::auto()?.build().await?;
    
    // Everything is configured automatically!
    // - Redis connection
    // - Worker count
    // - Auto-scaling
    // - Task registration
    // - Scheduler
    
    Ok(())
}
```

## Configuration Sources

Configuration is loaded with the following priority (later sources override earlier ones):

1. **Default values** (hardcoded sensible defaults)
2. **Configuration files** (TOML or YAML)
3. **Environment variables**

### Configuration Files

The system automatically looks for configuration files in these locations:

- `task-queue.toml`
- `task-queue.yaml` or `task-queue.yml`
- `config/task-queue.toml`
- `config/task-queue.yaml` or `config/task-queue.yml`

#### TOML Example (`task-queue.toml`)

```toml
[redis]
url = "redis://127.0.0.1:6379"
pool_size = 10
connection_timeout = 30  # seconds
command_timeout = 10     # seconds

[workers]
initial_count = 4
max_concurrent_tasks = 50

[autoscaler]
enabled = true
min_workers = 2
max_workers = 20
scale_up_threshold = 5.0
scale_down_threshold = 2.0

[auto_register]
enabled = true

[scheduler]
enabled = true

[actix]  # Only when using actix-integration feature
auto_configure_routes = true
route_prefix = "/tasks"
enable_metrics = true
enable_health_check = true
```

#### YAML Example (`task-queue.yaml`)

```yaml
redis:
  url: "redis://127.0.0.1:6379"
  pool_size: 10
  connection_timeout: 30
  command_timeout: 10

workers:
  initial_count: 4
  max_concurrent_tasks: 50

autoscaler:
  enabled: true
  min_workers: 2
  max_workers: 20
  scale_up_threshold: 5.0
  scale_down_threshold: 2.0

auto_register:
  enabled: true

scheduler:
  enabled: true

actix:
  auto_configure_routes: true
  route_prefix: "/tasks"
  enable_metrics: true
  enable_health_check: true
```

### Environment Variables

You can configure the system using environment variables:

#### Core Settings

```bash
# Redis connection
export REDIS_URL="redis://127.0.0.1:6379"
export REDIS_POOL_SIZE=10
export REDIS_CONNECTION_TIMEOUT=30
export REDIS_COMMAND_TIMEOUT=10

# Workers
export TASK_QUEUE_WORKERS=4

# Features
export TASK_QUEUE_AUTO_REGISTER=true
export TASK_QUEUE_SCHEDULER=true

# Actix Web (when using actix-integration)
export TASK_QUEUE_AUTO_ROUTES=true
export TASK_QUEUE_ROUTE_PREFIX="/api/tasks"
```

#### Nested Configuration

For nested configuration, use underscore separation:

```bash
export TASK_QUEUE_WORKERS_INITIAL_COUNT=4
export TASK_QUEUE_AUTOSCALER_MIN_WORKERS=2
export TASK_QUEUE_AUTOSCALER_MAX_WORKERS=20
export TASK_QUEUE_AUTOSCALER_SCALE_UP_THRESHOLD=5.0
```

## Configuration Methods

### 1. Auto Configuration (Recommended)

```rust
use rust_task_queue::prelude::*;

// Automatically loads from files and environment
let task_queue = TaskQueueBuilder::auto()?.build().await?;
```

### 2. Global Configuration

```rust
use rust_task_queue::prelude::*;

// Initialize global config once at startup
TaskQueueConfig::init_global()?;

// Use it anywhere in your application
let task_queue = TaskQueueBuilder::from_global_config()?.build().await?;
```

### 3. Manual Configuration

```rust
use rust_task_queue::prelude::*;

let config = TaskQueueConfig::from_env()?;
let task_queue = TaskQueueBuilder::from_config(config).build().await?;
```

### 4. Builder Pattern (Override specific settings)

```rust
use rust_task_queue::prelude::*;

let task_queue = TaskQueueBuilder::auto()?
    .redis_url("redis://custom-host:6379")  // Override Redis URL
    .initial_workers(8)                     // Override worker count
    .build().await?;
```

## Actix Web Integration

For Actix Web applications, use the auto-configuration helpers:

```rust
use actix_web::{web, App, HttpServer};
use rust_task_queue::prelude::*;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Auto-configure task queue with one line
    let task_queue = auto_configure_task_queue().await
        .expect("Failed to configure task queue");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(task_queue.clone()))
            // Auto-configure routes based on config
            .configure(configure_task_queue_routes_auto)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
```

## Configuration Reference

### Redis Configuration

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | String | `redis://127.0.0.1:6379` | Redis connection URL |
| `pool_size` | Option<u32> | None | Connection pool size |
| `connection_timeout` | Option<u64> | None | Connection timeout in seconds |
| `command_timeout` | Option<u64> | None | Command timeout in seconds |

### Worker Configuration

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `initial_count` | usize | 2 | Number of workers to start |
| `max_concurrent_tasks` | Option<usize> | None | Max concurrent tasks per worker |
| `heartbeat_interval` | Option<u64> | None | Worker heartbeat interval in seconds |
| `shutdown_grace_period` | Option<u64> | None | Grace period for shutdown in seconds |

### Auto-Scaler Configuration

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `min_workers` | usize | 1 | Minimum number of workers |
| `max_workers` | usize | 20 | Maximum number of workers |
| `scale_up_threshold` | f64 | 5.0 | Queue length threshold for scaling up |
| `scale_down_threshold` | f64 | 2.0 | Queue length threshold for scaling down |
| `scale_up_cooldown` | u64 | 60 | Cooldown period after scaling up (seconds) |
| `scale_down_cooldown` | u64 | 300 | Cooldown period after scaling down (seconds) |

### Auto-Registration Configuration

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | true | Enable automatic task registration |


### Scheduler Configuration

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | true | Enable the task scheduler |
| `tick_interval` | Option<u64> | None | Scheduler tick interval in seconds |
| `max_tasks_per_tick` | Option<usize> | None | Max tasks to process per tick |

### Actix Web Configuration

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `auto_configure_routes` | bool | true | Auto-configure task routes |
| `route_prefix` | String | "/tasks" | Base path for task routes |
| `enable_metrics` | bool | true | Enable metrics endpoint |
| `enable_health_check` | bool | true | Enable health check endpoint |

## Best Practices

### 1. Use Configuration Files for Complex Setups

For production deployments, use configuration files to maintain all settings in one place:

```toml
# production.toml
[redis]
url = "redis://prod-redis:6379"
pool_size = 20

[workers]
initial_count = 10

[autoscaler]
min_workers = 5
max_workers = 50
```

### 2. Environment Variables for Secrets

Use environment variables for sensitive information:

```bash
export REDIS_URL="redis://user:password@secure-redis:6379"
```

### 3. Layer Configuration

Start with a base configuration file and override with environment variables:

```rust
// This loads config file first, then applies environment overrides
let task_queue = TaskQueueBuilder::auto()?.build().await?;
```

### 4. Global Configuration for Libraries

If you're building a library that uses the task queue, initialize global config once:

```rust
pub fn init_task_system() -> Result<(), TaskQueueError> {
    TaskQueueConfig::init_global()?;
    Ok(())
}

pub async fn get_task_queue() -> Result<TaskQueue, TaskQueueError> {
    TaskQueueBuilder::from_global_config()?.build().await
}
```

### 5. Configuration Validation

Always handle configuration errors gracefully:

```rust
let task_queue = match TaskQueueBuilder::auto() {
    Ok(builder) => builder.build().await?,
    Err(e) => {
        eprintln!("Configuration error: {}", e);
        eprintln!("Using fallback configuration...");
        TaskQueueBuilder::new("redis://127.0.0.1:6379")
            .initial_workers(2)
            .build().await?
    }
};
```

## Feature Flags

Make sure to enable the right features in your `Cargo.toml`:

```toml
[dependencies]
rust-task-queue = { 
    version = "0.1", 
    features = [
        "config-support",      # For file-based configuration
        "actix-integration",   # For Actix Web integration
        "auto-register",       # For automatic task registration
    ] 
}
```

## Examples

See the `examples/` directory for complete working examples:

- `examples/actix_auto_config.rs` - Actix Web with auto-configuration
- `examples/worker_auto_config.rs` - Worker with auto-configuration
- Look at `task-queue.toml` and `task-queue.yaml` for configuration file examples 