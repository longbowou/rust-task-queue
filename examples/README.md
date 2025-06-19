# Rust Task Queue Examples

This directory contains comprehensive examples demonstrating all features of the `rust-task-queue` crate across different use cases and web frameworks. Each example is production-ready and showcases best practices for building scalable task processing systems.

## Available Examples

### Web Framework Integrations

#### 1. [Actix Web Integration](./actix-integration/)
**Most comprehensive example** - Full-featured production-ready web application

**Features:**
- **Task Types**: Email, Slack, SMS, Discord, webhooks, analytics, ML training, file processing, reports, backups
- **Complete API**: 40+ endpoints covering all task operations, system management, and monitoring
- **Advanced Monitoring**: Comprehensive metrics, SLA monitoring, auto-scaler insights, alerts
- **Configuration Management**: TOML/YAML/ENV support with hot reloading
- **Security Features**: Input validation, rate limiting, CORS support
- **Docker Support**: Full containerization with docker-compose

**Quick Start:**
```bash
cd actix-integration
docker compose up 
```

#### 2. [Axum Integration](./axum-integration/)
**Modern async-first framework** - Clean, lightweight web application

**Features:**
- **Core Task Types**: Email, notifications, data processing, ML training, analytics
- **Metrics & Monitoring**: Health checks, system status, performance metrics
- **Auto-scaling**: Dynamic worker management based on queue load
- **Configuration**: TOML/YAML support with environment variable overrides
- **Type Safety**: Strongly typed extractors and responses
- **Docker Support**: Full containerization with docker-compose

**Quick Start:**
```bash
cd axum-integration
docker compose up 
```

### Performance Testing

#### 3. [Performance Test](./performance_test.rs)
**Standalone performance benchmark** - Comprehensive load testing and metrics

**Features:**
- **Multi-complexity Tasks**: Light (10ms), medium (100ms), heavy (500ms) workloads
- **Priority Queues**: Task prioritization based on complexity
- **Real-time Metrics**: Queue depth, throughput, completion rates
- **Tracing Integration**: Detailed performance tracking and lifecycle events
- **Configurable Load**: Customizable task types and counts

**Quick Start:**
```bash
# Ensure Redis is running
docker run -d --name redis -p 6379:6379 redis:7-alpine

# Run performance test
cargo run --example performance_test --features "tracing"
```

## Quick Start Guide

### Environment Setup

```bash
# Optional: Set Redis URL (defaults to localhost:6379)
export REDIS_URL="redis://localhost:6379"

# Optional: Enable comprehensive logging
export RUST_LOG="rust_task_queue=info,debug"
```

## Further Reading

- **[Main Documentation](../README.md)**: Core concepts and API reference
- **[Development Guide](../DEVELOPMENT.md)**: Development workflow and best practices

## Contributing

These examples serve as both demonstrations and templates for production use. Contributions welcome:

1. **Fork** the repository
2. **Create** a feature branch
3. **Add** comprehensive tests
4. **Update** documentation
5. **Submit** a pull request

When contributing examples:
- Include **comprehensive error handling**
- Add **appropriate test coverage**
- Update **this README** with new examples
- Provide **clear documentation** and usage instructions

## License

All examples are part of the rust-task-queue project and follow the same license terms. 