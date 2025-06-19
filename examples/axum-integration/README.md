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

### Advanced Features

- **Configuration management** (TOML, YAML, ENV)
- **SLA monitoring** and alerts
- **Performance metrics** and diagnostics
- **Batch operations** and bulk processing
- **Task registry** information and introspection
- **Auto-scaler metrics** and recommendations

### Axum-Specific Features

- **All metrics endpoints** with JSON responses
- **Health checks** and system status
- **Request tracing** with structured logging
- **Typed extractors** for task enqueueing
- **Error responses** with proper HTTP status codes
- **State management** with Arc<TaskQueue>

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
export AXUM_ROUTE_PREFIX="/api/v1/tasks"
```

### Configuration File (task-queue.toml)

The example includes a comprehensive TOML configuration file [**task-queue.toml**](task-queue.toml) demonstrating all
available options:

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