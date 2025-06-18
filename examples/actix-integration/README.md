# Comprehensive Actix Web Integration Example

This example showcases **ALL** available features of the `rust-task-queue` crate through a comprehensive
production-ready Actix Web integration. It demonstrates every aspect of the framework from basic task queueing to
advanced auto-scaling and system management.

## Features Demonstrated

### Core Framework Features

- **TaskQueueBuilder** with all configuration options
- **Auto-registration** and manual task registry
- **All available metrics endpoints** and health monitoring
- **Advanced scheduling** (delays, future dates, recurring)
- **Batch task operations** for high-throughput scenarios
- **Graceful shutdown** handling with proper cleanup
- **Configuration management** (TOML/YAML/ENV variables)
- **Comprehensive error handling** with structured responses
- **Security features** and rate limiting
- **SLA monitoring** and alerting

### Task Types Showcase

#### Communication Tasks

- **Email**: Templates, attachments, priority levels, delivery tracking
- **Slack**: Rich attachments, threading, custom usernames/emojis
- **SMS**: International formatting, delivery receipts, priority routing
- **Discord**: Rich embeds, text-to-speech, custom webhooks
- **Webhooks**: HTTP methods, headers, signatures, SSL verification
- **Multi-channel**: Urgent notifications across multiple platforms

#### Processing Tasks

- **Data Processing**: Batch operations, filters, transformations, format conversion
- **Analytics**: Event tracking, session management, campaign attribution
- **ML Training**: Model types, hyperparameters, GPU support, validation
- **File Processing**: Format conversion, compression, validation, virus scanning
- **Image Processing**: Resize, crop, filters, watermarks, format optimization
- **Report Generation**: PDF/Excel/CSV, templates, data sources, distribution

#### Operations Tasks

- **Backups**: Full/incremental/differential, compression, encryption, retention
- **Database Maintenance**: Vacuum, reindex, analyze, scheduled windows
- **Cache Warming**: Strategies, TTL management, hit rate optimization
- **Batch Processing**: Parallel workers, error thresholds, item priorities

### API Architecture

#### RESTful Endpoints

```
/api/v1/tasks/          # Task operations
/api/v1/system/         # System management  
/api/v1/tasks/metrics/  # Comprehensive monitoring
```

#### Auto-configured Routes

The system automatically configures monitoring endpoints based on `task-queue.toml`:

- Health checks and system status
- Performance and auto-scaler metrics
- SLA monitoring and alerts
- Diagnostics and uptime tracking

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Actix Web Server                         │
├─────────────────────────────────────────────────────────────┤
│  Middleware: Logging, Compression, Headers, CORS           │
├─────────────────────────────────────────────────────────────┤
│                  Request Routing                           │
│  ┌───────────────┬────────────────┬───────────────────────┐ │
│  │ Task Routes   │ System Routes  │ Auto-configured       │ │
│  │ /api/v1/tasks │ /api/v1/system │ /api/v1/tasks/*       │ │
│  └───────────────┴────────────────┴───────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│                    TaskQueue Core                          │
│  ┌───────────────┬────────────────┬───────────────────────┐ │
│  │ RedisBroker   │ AutoScaler     │ TaskScheduler         │ │
│  │ Connection    │ Load-based     │ Delayed Execution     │ │
│  │ Pooling       │ Scaling        │ Persistent Storage    │ │
│  └───────────────┴────────────────┴───────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│                   Task Registry                            │
│  ┌───────────────────────────────────────────────────────┐ │
│  │ Auto-Registration (Inventory-based)                   │ │
│  │ - Communication Tasks (8 types)                       │ │
│  │ - Processing Tasks (10 types)                         │ │
│  │ - Operations Tasks (4 types)                          │ │
│  └───────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│                 Worker Management                          │
│  ┌───────────────┬────────────────┬───────────────────────┐ │
│  │ Dynamic       │ Health         │ Graceful              │ │
│  │ Scaling       │ Monitoring     │ Shutdown              │ │
│  └───────────────┴────────────────┴───────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## Quick Start

### 1. Prerequisites

```bash
# Redis server (required)
docker run -d --name redis -p 6379:6379 redis:7-alpine

# Or use docker-compose
docker-compose up -d redis
```

### 2. Start the Web Server

```bash
# From the actix-integration directory
cargo run --bin web-server

# The server will start at http://localhost:8000
# Configuration is loaded from task-queue.toml
```

### 3. Start Workers

```bash
# In a separate terminal
cargo run --bin task-worker

# Or start multiple workers
cargo run --bin task-worker &
cargo run --bin task-worker &
cargo run --bin task-worker &
```

### 4. Test the API

```bash
# Simple email task
curl -X POST http://localhost:8000/api/v1/tasks/email \
  -H 'Content-Type: application/json' \
  -d '{"to":"test@example.com","subject":"Hello World"}'

# Check system status
curl http://localhost:8000/api/v1/system/status

# View comprehensive metrics
curl http://localhost:8000/api/v1/tasks/metrics
```

## API Reference

### Task Management

| Endpoint                                   | Method | Description                               |
|--------------------------------------------|--------|-------------------------------------------|
| `/api/v1/tasks/email`                      | POST   | Send email (simple or with attachments)   |
| `/api/v1/tasks/email/schedule`             | POST   | Schedule email for future delivery        |
| `/api/v1/tasks/slack-notification`         | POST   | Send Slack message with rich formatting   |
| `/api/v1/tasks/sms`                        | POST   | Send SMS with delivery options            |
| `/api/v1/tasks/webhook`                    | POST   | Deliver webhook with retry logic          |
| `/api/v1/tasks/multi-channel-notification` | POST   | Send urgent notifications across channels |
| `/api/v1/tasks/discord-notification`       | POST   | Send Discord message with embeds          |
| `/api/v1/tasks/analytics`                  | POST   | Track events and user behavior            |
| `/api/v1/tasks/ml-training`                | POST   | Train ML models with hyperparameters      |
| `/api/v1/tasks/file-processing`            | POST   | Process files with validation             |
| `/api/v1/tasks/image-processing`           | POST   | Transform images with operations          |
| `/api/v1/tasks/report`                     | POST   | Generate reports in multiple formats      |
| `/api/v1/tasks/backup`                     | POST   | Create system backups                     |
| `/api/v1/tasks/database-maintenance`       | POST   | Perform database optimization             |
| `/api/v1/tasks/cache-warmup`               | POST   | Warm caches for performance               |

### Batch Operations

| Endpoint                           | Method | Description                         |
|------------------------------------|--------|-------------------------------------|
| `/api/v1/tasks/batch/email`        | POST   | Send multiple emails in batch       |
| `/api/v1/tasks/batch/notification` | POST   | Send multiple notifications         |
| `/api/v1/tasks/schedule/advanced`  | POST   | Schedule with specific datetime     |
| `/api/v1/tasks/batch-processing`   | POST   | Process items with parallel workers |

### System Management

| Endpoint                       | Method | Description                 |
|--------------------------------|--------|-----------------------------|
| `/api/v1/system/status`        | GET    | Comprehensive system status |
| `/api/v1/system/config`        | GET    | Current configuration       |
| `/api/v1/system/config/reload` | POST   | Reload configuration        |
| `/api/v1/system/workers/scale` | POST   | Manual worker scaling       |
| `/api/v1/system/shutdown`      | POST   | Graceful shutdown           |

### Monitoring & Metrics

| Endpoint                            | Method | Description            |
|-------------------------------------|--------|------------------------|
| `/api/v1/tasks/health`              | GET    | Health check           |
| `/api/v1/tasks/status`              | GET    | Detailed system status |
| `/api/v1/tasks/metrics`             | GET    | All metrics combined   |
| `/api/v1/tasks/metrics/performance` | GET    | Performance metrics    |
| `/api/v1/tasks/metrics/autoscaler`  | GET    | Auto-scaler metrics    |
| `/api/v1/tasks/metrics/queues`      | GET    | Queue-specific metrics |
| `/api/v1/tasks/metrics/workers`     | GET    | Worker metrics         |
| `/api/v1/tasks/metrics/memory`      | GET    | Memory usage           |
| `/api/v1/tasks/uptime`              | GET    | Uptime information     |
| `/api/v1/tasks/diagnostics`         | GET    | System diagnostics     |
| `/api/v1/tasks/alerts`              | GET    | Active alerts          |
| `/api/v1/tasks/sla`                 | GET    | SLA status             |
| `/api/v1/tasks/registry`            | GET    | Registered tasks       |

## 🔧 Configuration

The example uses a comprehensive configuration file (`task-queue.toml`) that demonstrates all available options:

### Redis Configuration

```toml
[redis]
url = "redis://redis:6379"
pool_size = 20
connection_timeout = 10
command_timeout = 5
```

### Auto-Scaling Configuration

```toml
[autoscaler]
enabled = true
min_workers = 5
max_workers = 50
scale_up_count = 2
scale_down_count = 1

# Multidimensional scaling triggers
[autoscaler.scaling_triggers]
queue_pressure_threshold = 1.2
worker_utilization_threshold = 0.85
task_complexity_threshold = 2.0
error_rate_threshold = 0.03
memory_pressure_threshold = 1024.0

# Adaptive learning
enable_adaptive_thresholds = true
learning_rate = 0.05
adaptation_window_minutes = 60

# SLA targets
[autoscaler.target_sla]
max_p95_latency_ms = 3000.0
min_success_rate = 0.99
max_queue_wait_time_ms = 5000.0
```

### Security & Monitoring

```toml
[security]
max_payload_size_mb = 16
enable_input_validation = true
rate_limiting = true
max_requests_per_minute = 1000

[metrics]
enabled = true
export_interval = 60
retention_days = 30
enable_alerts = true
collect_scaling_metrics = true
```

### Actix Web Integration

```toml
[actix]
auto_configure_routes = true
route_prefix = "/api/v1/tasks"
enable_metrics = true
enable_health_check = true
cors_enabled = true
rate_limit_requests_per_minute = 100
```

## Testing

### Comprehensive Test Suite

Use the provided `http.http` file that contains 50+ test cases covering:

- **System Management**: Health checks, metrics, configuration
- **Basic Tasks**: Email, notifications, data processing
- **Communication**: Slack, SMS, webhooks, multi-channel
- **Processing**: Analytics, ML training, file processing
- **Advanced Features**: Batch operations, scheduling
- **Error Scenarios**: Invalid requests, missing fields
- **Performance Testing**: High-volume operations

### Running Tests

1. Start the server: `cargo run --bin web-server`
2. Start workers: `cargo run --bin task-worker`
3. Open `http.http` in your IDE (VS Code, IntelliJ)
4. Execute requests individually or in sequence

### Automated Testing

```bash
# Run with curl
curl -X GET http://localhost:8000/api/v1/system/status | jq

# Or use the provided test scripts
./test-all-endpoints.sh  # If available
```

## Monitoring & Observability

### Real-time Metrics

- **Queue Metrics**: Pending, processed, failed tasks per queue
- **Worker Metrics**: Active count, utilization, load distribution
- **Performance**: Task execution times, throughput, error rates
- **Auto-scaler**: Scaling decisions, triggers, recommendations
- **System Health**: Memory usage, uptime, component status

### SLA Monitoring

- P95 latency tracking
- Success rate monitoring
- Queue wait time analysis
- Adaptive threshold learning
- Alert management

### Diagnostic Tools

- Comprehensive health checks
- Component status monitoring
- Active alert tracking
- System diagnostics
- Performance bottleneck identification

## Security Features

### Input Validation

- Payload size limits
- Schema validation
- Rate limiting per endpoint
- CORS configuration

### Error Handling

- Structured error responses
- Request ID tracking
- Comprehensive error types
- Graceful degradation

## Production Deployment

### Docker Support

```bash
# Build the application
docker build -t rust-task-queue-actix .

# Run with docker-compose
docker-compose up -d
```

### Environment Variables

```bash
REDIS_URL=redis://localhost:6379
RUST_LOG=info
WORKERS_COUNT=5
AUTO_SCALE_ENABLED=true
```

### Health Checks

```bash
# Kubernetes readiness probe
curl -f http://localhost:8000/api/v1/tasks/health

# Liveness probe  
curl -f http://localhost:8000/api/v1/system/status
```

## Performance Characteristics

### Benchmarks (Typical Results)

- **Throughput**: 10,000+ tasks/second
- **Latency**: P95 < 100ms for simple tasks
- **Memory**: ~50MB base + ~2MB per worker
- **Redis**: 1,000+ ops/second per connection

### Scaling

- **Horizontal**: Auto-scales 1-50 workers based on load
- **Vertical**: Efficient memory usage with connection pooling
- **Queue**: Multiple priority queues for workload separation

## Development

### Adding New Task Types

1. Create task struct with `#[derive(AutoRegisterTask)]`
2. Implement `Task` trait with execution logic
3. Add endpoint handler in `main.rs`
4. Update routing configuration
5. Add tests to `http.http`

### Custom Metrics

The framework automatically collects comprehensive metrics. Custom metrics can be added through the `MetricsCollector`
interface.

### Configuration Extensions

Add new configuration sections in `task-queue.toml` and update the `TaskQueueConfig` struct.

## Learn More

- [Main Documentation](../../README.md)
- [Task Development Guide](../../DEVELOPMENT.md)
- [API Reference](./API.md)
- [Configuration Reference](./CONFIGURATION.md)

## Contributing

This example serves as both a demonstration and a template for production use. Contributions welcome:

1. Fork the repository
2. Create a feature branch
3. Add comprehensive tests
4. Update documentation
5. Submit a pull request

## License

This example is part of the rust-task-queue project and follows the same license terms. 