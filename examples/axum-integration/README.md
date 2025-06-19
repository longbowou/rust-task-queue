# Axum Integration Example

This comprehensive example demonstrates all features of the `rust-task-queue` crate integrated with the Axum web
framework. It showcases production-ready patterns, comprehensive error handling, metrics, health checks, and advanced
task processing capabilities.

## Features Demonstrated

### Core Task Queue Features

- **TaskQueueBuilder** with all configuration options
- **Auto-registration** and manual task registry
- **Multiple queue priorities** (default, high, low)
- **Task scheduling** with delay support
- **Auto-scaling workers** based on queue load
- **Graceful shutdown** handling
- **Comprehensive error handling**
- **Task timeouts, retries, and cancellation**

### Axum-Specific Features

- **All metrics endpoints** with JSON responses
- **Health checks** and system status
- **CORS support** for web clients
- **Request tracing** with structured logging
- **Typed extractors** for task enqueueing
- **Error responses** with proper HTTP status codes
- **State management** with Arc<TaskQueue>

### Advanced Features

- **Configuration management** (TOML, YAML, ENV)
- **SLA monitoring** and alerts
- **Performance metrics** and diagnostics
- **Batch operations** and bulk processing
- **Task registry** information and introspection
- **Auto-scaler metrics** and recommendations

## Quick Start

### Prerequisites

1. **Redis server** running on localhost:6379 (or set `REDIS_URL`)
2. **Rust 1.70+** with cargo

### Running the Example

1. **Start the Axum server**:
   ```bash
   cd examples/axum-integration
   cargo run
   ```

2. **Start the task worker** (in another terminal):
   ```bash
   cd examples/axum-integration
   cargo run --bin task-worker
   ```

3. **Open your browser** to http://127.0.0.1:3000

## API Endpoints

### Health & Status

- `GET /tasks/health` - Detailed health check with component status
- `GET /tasks/status` - System status with worker information
- `GET /api/status` - Comprehensive system status

### Metrics & Monitoring

- `GET /tasks/metrics` - Comprehensive metrics (all systems)
- `GET /tasks/metrics/system` - System metrics (memory, CPU, uptime)
- `GET /tasks/metrics/performance` - Performance metrics and SLA
- `GET /tasks/metrics/autoscaler` - Auto-scaler metrics and recommendations
- `GET /tasks/metrics/queues` - Individual queue metrics
- `GET /tasks/metrics/workers` - Worker-specific metrics
- `GET /tasks/metrics/memory` - Memory usage metrics

### Task Management

- `POST /api/tasks/email` - Enqueue email task
- `POST /api/tasks/notification` - Enqueue notification task
- `POST /api/tasks/data-processing` - Enqueue data processing task
- `POST /api/tasks/slack-notification` - Enqueue Slack notification
- `POST /api/tasks/sms` - Enqueue SMS task
- `POST /api/tasks/analytics` - Enqueue analytics task
- `POST /api/tasks/ml-training` - Enqueue ML training task (high priority)
- `POST /api/schedule/email` - Schedule email task with delay

### Administration

- `GET /api/config` - Get current configuration
- `POST /api/config/reload` - Reload configuration from files
- `POST /api/shutdown` - Initiate graceful shutdown
- `GET /tasks/registry` - Task registry information
- `GET /tasks/alerts` - Active alerts from metrics system
- `GET /tasks/sla` - SLA status and violations
- `GET /tasks/diagnostics` - Comprehensive system diagnostics
- `GET /tasks/uptime` - System uptime and runtime information

## Task Examples

### Basic Email Task

```bash
curl -X POST http://127.0.0.1:3000/api/tasks/email \
  -H "Content-Type: application/json" \
  -d '{
    "to": "user@example.com",
    "subject": "Welcome!",
    "body": "Welcome to our service!",
    "priority": "high"
  }'
```

Response:

```json
{
  "task_id": "550e8400-e29b-41d4-a716-446655440000",
  "queue": "default",
  "enqueued_at": "2024-01-01T12:00:00Z",
  "estimated_execution_time": null
}
```

### Scheduled Email Task

```bash
curl -X POST http://127.0.0.1:3000/api/schedule/email \
  -H "Content-Type: application/json" \
  -d '{
    "email": {
      "to": "user@example.com",
      "subject": "Reminder",
      "body": "This is your scheduled reminder!"
    },
    "delay_seconds": 300
  }'
```

### ML Training Task (High Priority)

```bash
curl -X POST http://127.0.0.1:3000/api/tasks/ml-training \
  -H "Content-Type: application/json" \
  -d '{
    "model_name": "sentiment_classifier",
    "dataset_path": "/data/reviews.csv",
    "algorithm": "transformer",
    "hyperparameters": {
      "learning_rate": 0.001,
      "batch_size": 32
    },
    "training_config": {
      "epochs": 50,
      "batch_size": 32,
      "learning_rate": 0.001,
      "validation_split": 0.2,
      "early_stopping": true
    }
  }'
```

### Analytics Event

```bash
curl -X POST http://127.0.0.1:3000/api/tasks/analytics \
  -H "Content-Type: application/json" \
  -d '{
    "event_type": "user_signup",
    "user_id": "user_12345",
    "properties": {
      "source": "web",
      "campaign": "summer_2024",
      "referrer": "google"
    },
    "session_id": "sess_67890"
  }'
```

## Configuration

### Environment Variables

```bash
# Redis Configuration
export REDIS_URL="redis://localhost:6379"

# Worker Configuration  
export WORKER_COUNT=5
export ENABLE_AUTOSCALER=true

# Axum Configuration
export AXUM_ENABLE_CORS=true
export AXUM_ENABLE_TRACING=true
export AXUM_ROUTE_PREFIX="/api/v1/tasks"
```

### Configuration File (task-queue.toml)

The example includes a comprehensive TOML configuration file demonstrating all available options:

```toml
[redis]
url = "redis://127.0.0.1:6379"
pool_size = 10

[workers]
initial_count = 3
max_concurrent_tasks = 10

[autoscaler]
enabled = true
min_workers = 1
max_workers = 10

[axum]
auto_configure_routes = true
route_prefix = "/api/v1/tasks"
enable_metrics = true
enable_health_check = true
enable_cors = true
enable_tracing = true
```

## Task Types

### Basic Tasks

- **EmailTask** - Send emails with full configuration
- **NotificationTask** - Multi-channel notifications
- **DataProcessingTask** - Process datasets with parameters

### Communication Tasks

- **SlackNotificationTask** - Rich Slack messages with attachments
- **SmsTask** - SMS with different message types (transactional, marketing, alerts)

### Processing Tasks

- **AnalyticsTask** - Process analytics events with properties
- **MLTrainingTask** - Machine learning model training with hyperparameters

## Worker Binaries

### Comprehensive Worker (`task-worker`)

Full-featured worker with manual task registration:

```bash
cargo run --bin task-worker
```

Environment variables:

- `REDIS_URL` - Redis connection URL
- `WORKER_COUNT` - Number of workers to start (default: 3)
- `ENABLE_AUTOSCALER` - Enable auto-scaler (default: false)

### Environment-Only Worker (`task-worker-env-only`)

Minimal worker using auto-configuration:

```bash
cargo run --bin task-worker-env-only
```

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run with features
cargo test --features "full"
```

### Docker Support

```bash
# Start Redis
docker run -d --name redis -p 6379:6379 redis:7-alpine

# Run the example
cargo run
```

### Monitoring and Debugging

1. **Health Check**: GET `/tasks/health`
2. **System Metrics**: GET `/tasks/metrics/system`
3. **Worker Status**: GET `/tasks/metrics/workers`
4. **Queue Status**: GET `/tasks/metrics/queues`
5. **Performance**: GET `/tasks/metrics/performance`

## Performance Features

### Auto-Scaling

- **Dynamic worker scaling** based on queue pressure
- **Configurable thresholds** for scale up/down decisions
- **Cooldown periods** to prevent thrashing
- **Utilization monitoring** for optimal resource usage

### Metrics Collection

- **Real-time metrics** for all system components
- **Performance tracking** with SLA monitoring
- **Memory usage** monitoring and leak detection
- **Task execution** statistics and timing

### Error Handling

- **Comprehensive error responses** with proper HTTP status codes
- **Request ID tracking** for debugging
- **Graceful degradation** when components fail
- **Retry logic** with exponential backoff

## Production Considerations

### Security

- **Request validation** and sanitization
- **Error message** sanitization
- **Rate limiting** ready (add tower-http rate limiting)

### Observability

- **Structured logging** with tracing
- **Metrics collection** for monitoring
- **Health checks** for load balancers
- **Graceful shutdown** for zero-downtime deployments

### Scalability

- **Horizontal worker scaling** with auto-scaler
- **Queue-based architecture** for distributed processing
- **Redis clustering** support for high availability
- **Resource monitoring** for capacity planning

This example provides a production-ready foundation for building scalable task processing systems with Axum and
rust-task-queue. 