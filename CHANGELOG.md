# Changelog

All notable changes to the Rust Task Queue project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2025-01-26

### Changed

#### Publishing and Compatibility Improvements

- **Changed Rust edition** from 2024 to 2021 for broader compatibility
- **Added minimum supported Rust version (MSRV)** of 1.70.0 
- **Improved crate accessibility** by supporting older Rust versions
- **Made auto-register functionality conditional** to support publishing without macros dependency

#### Publishing Configuration

- **Fixed workspace publishing issue** by making macros dependency optional
- **Removed direct dependency** on unpublished `rust-task-queue-macros` crate
- **Added conditional compilation** for all auto-register features
- **Updated feature flags** to handle missing auto-register gracefully

#### Developer Experience

- **Added comprehensive publishing guide** (`PUBLISHING.md`) with step-by-step instructions
- **Improved CLI handling** for environments without auto-register feature
- **Enhanced conditional compilation** across all modules (CLI, Actix integration, core library)
- **Fixed compiler warnings** and import issues for cleaner builds

### Fixed

#### Packaging and Publishing

- **Resolved cargo publish error** caused by unpublished workspace dependency
- **Fixed conditional feature compilation** in CLI and library code  
- **Corrected import paths** and removed unused imports
- **Enabled successful packaging** without macros crate dependency

#### Code Quality

- **Fixed clippy warnings** and unused variable warnings
- **Improved error handling** for missing auto-register functionality
- **Enhanced feature gate consistency** across all modules
- **Cleaned up conditional compilation directives**

### Technical Details

#### Compatibility Matrix

| Feature | With auto-register | Without auto-register |
|---------|-------------------|----------------------|
| Core task queue | ✅ | ✅ |
| Manual registration | ✅ | ✅ |
| Derive macros | ✅ | ❌ |
| CLI workers | ✅ | ✅ |
| Actix integration | ✅ | ✅ |

#### Publishing Strategy

- **Current release (0.1.1)**: Main crate without auto-register macros
- **Future releases**: Will include published macros crate for full functionality
- **Development**: Full workspace functionality maintained for contributors

#### Migration Guide

Users upgrading from 0.1.0 development versions should note:
- Auto-register macros temporarily unavailable in published version
- Manual task registration works identically
- All core functionality preserved
- CLI and Actix integration fully functional

## [0.1.0] - 2025-06-17 https://github.com/longbowou/rust-task-queue/releases/tag/0.1.0

### Initial Release

This is the first major release of Rust Task Queue, a high-performance, Redis-backed task queue framework with enhanced
auto-scaling capabilities for async Rust applications.

### Major Features Added

#### Core Task Queue Framework

- **Redis-backed broker** with optimized connection pooling and deadpool-redis integration
- **High-performance task execution** with MessagePack serialization (~39ns serialization, ~31ns deserialization)
- **Multiple queue priorities** with predefined queue constants and priority management
- **Task scheduling** with delay support and persistent Redis-based scheduling
- **Retry logic** with exponential backoff and configurable maximum attempts
- **Task timeouts** and comprehensive failure handling with graceful error recovery
- **Graceful shutdown** with active task tracking and proper cleanup

#### Enhanced Multi-Dimensional Auto-Scaling

- **5-metric analysis system** for intelligent scaling decisions:
    - Queue Pressure Score with weighted queue depth
    - Worker Utilization real-time monitoring
    - Task Complexity Factor recognition
    - Error Rate Monitoring for system health
    - Memory Pressure per-worker analysis
- **Adaptive threshold learning** with SLA-driven optimization
- **Stability controls** with hysteresis and independent cooldown periods
- **Consecutive signal requirements** to prevent scaling oscillations
- **Production-ready scaling triggers** with configurable thresholds

#### Actix Web Integration

- **15+ comprehensive metrics endpoints** for monitoring and diagnostics
- **Health check endpoints** with detailed component status
- **Auto-configurable routes** with customizable prefixes
- **Built-in task enqueue/status endpoints** for web integration
- **Production-ready middleware** with error handling and logging
- **SLA monitoring and alerts** through HTTP endpoints

####Enterprise-Grade Observability

- **Comprehensive structured logging** with tracing integration
- **Complete task lifecycle tracking** from enqueue to completion
- **Performance monitoring** with execution timing and throughput analysis
- **Error chain analysis** with deep context for debugging
- **Worker activity monitoring** with real-time status updates
- **Distributed tracing** with async instrumentation and span correlation
- **Multiple output formats** (JSON, compact, pretty) for different environments
- **Environment-based configuration** for production deployments

#### Production-Ready Features

- **Robust error handling** with custom TaskQueueError types
- **Type-safe task registration** with automatic discovery via macros
- **Advanced async task spawning** with intelligent backpressure management
- **Smart resource allocation** with semaphore-based concurrency control
- **Memory-efficient operations** with optimized data structures
- **Security enhancements** with input validation and injection protection

#### Performance Optimizations

- **Sub-40ns serialization/deserialization** using MessagePack
- **Connection pooling** with Redis cluster support
- **Concurrent task processing** with configurable worker limits
- **Memory-optimized data structures** using DashMap and Arc
- **Optimized queue operations** with batch processing capabilities

#### Comprehensive Testing Infrastructure

- **220+ total tests** across multiple test categories:
    - 122 unit tests covering core functionality
    - 9 integration tests for end-to-end workflows
    - 22 Actix integration tests for web endpoints
    - 6 performance tests for load handling
    - 11 security tests for injection protection
    - 9 error scenario tests for edge cases
    - 5 comprehensive benchmarks
- **Automated testing scripts** with Redis container management
- **CI/CD integration** with strict linting and quality gates
- **Performance regression testing** with detailed benchmarks

#### Developer Experience

- **CLI tools** for standalone workers with process separation
- **Configuration management** with TOML/YAML support
- **Automatic task registration** using procedural macros
- **Feature-gated modules** for flexible dependency management
- **Comprehensive documentation** with examples and best practices
- **Development scripts** for testing and benchmarking

### Development Infrastructure

#### Build System & CI/CD

- **Strict Clippy compliance** with zero warnings policy
- **Automated testing pipeline** with Redis container orchestration
- **Benchmark suite** with performance validation and reports
- **Multi-platform testing** with comprehensive error scenarios
- **Code quality gates** with formatting and linting checks

#### Configuration Management

- **External configuration files** (task-queue.toml/yaml)
- **Environment-based overrides** for production deployments
- **Feature-based compilation** with optional dependencies
- **Default configurations** for quick setup and development

#### Documentation & Examples

- **Comprehensive README** with feature matrix and benchmarks
- **Real-world examples** including Actix Web integration
- **Performance benchmarks** with detailed timing analysis
- **API documentation** with usage examples and best practices

### Package Features

#### Default Features (`default`)

- `tracing`: Enterprise-grade structured logging
- `auto-register`: Automatic task discovery
- `config-support`: External configuration files
- `cli`: Standalone worker binaries

#### Optional Features

- `actix-integration`: Actix Web framework integration
- `full`: All features enabled for maximum functionality

#### Feature Combinations

- **Web applications**: `tracing` + `auto-register` + `actix-integration`
- **Standalone workers**: `tracing` + `auto-register` + `cli`
- **Minimal deployment**: Core functionality only
- **Development**: `full` feature set

### Commit History

This release includes the complete development history from project inception:

#### June 17, 2025

- `daf9e40` feat(tracing): implement comprehensive observability system with structured logging
- `e5d5e32` feat(actix-integration): comprehensive example covering all framework features
- `553faca` feat(actix,docs): add comprehensive metrics API with 15+ endpoints and full test coverage
- `5d6a363` feat(actix): add comprehensive metrics endpoints with enhanced monitoring
- `4720026` feat(actix): add comprehensive metrics endpoints with enhanced monitoring
- `bed57d6` fix: update actix integration task-queue.toml
- `1cd00c9` refactor: reformat code & optimize import
- `7867c1c` refactor: use centralize queue names for tests and improve code quality
- `89c38cb` feat: Add comprehensive benchmark suite with automation and performance validation
- `0e976fc` feat: Add comprehensive benchmark suite with automation and performance validation
- `f27855b` feat: comprehensive production-ready improvements across security, performance, and reliability
- `b46da2b` feat: add comprehensive cursor rules with automated testing guidelines
- `ee1b6f8` feat: enhance test infrastructure and fix compilation issues
- `274d589` feat: enhance test infrastructure and fix compilation issues

#### June 16, 2025

- `e26b5c8` feat: overhaul worker async task spawning with intelligent backpressure management
- `fcb74a1` docs(readme): recommend single task-queue.toml config for Actix Web integration
- `b0c2310` feat: implement high-priority code improvements and safety enhancements
- `5e4f51b` fix: rewrite integration tests with proper isolation and reliability
- `c696638` feat: comprehensive crate improvements and integration test fixes
- `140f7bb` feat: comprehensive codebase improvements and development infrastructure
- `3d57517` refactor: main.rs
- `3943486` feat: improve worker task result logging readability
- `80a455c` rust-task-queue is a high-performance, Redis-backed task queue framework with auto-scaling capabilities for
  async Rust applications

### Performance Benchmarks

| Operation                  | Time      | Status      |
|----------------------------|-----------|-------------|
| Task Serialization         | ~39.15 ns | Excellent   |
| Task Deserialization       | ~31.51 ns | Excellent   |
| Queue Config Lookup        | ~39.76 ns | Excellent   |
| Queue Management           | ~1.38 µs  | Very Good   |
| Enhanced AutoScaler Config | ~617 ps   | Outstanding |

### Use Cases

This release enables the following production use cases:

- **High-throughput web applications** with separate worker processes
- **Microservices architectures** with task-based communication
- **Background job processing** with priority queues and retry logic
- **Scheduled task execution** with persistent scheduling
- **Auto-scaling worker fleets** with intelligent resource management
- **Monitoring and observability** with comprehensive metrics APIs
- **Development and testing** with full local Redis integration

### Dependencies

#### Core Dependencies

- **redis** 0.24 with tokio and connection-manager features
- **deadpool-redis** 0.14 for advanced connection pooling
- **tokio** 1.0 with full async runtime
- **serde** 1.0 with derive macros for serialization
- **rmp-serde** 1.1 for high-performance MessagePack serialization
- **uuid** 1.0 for unique task identification
- **async-trait** 0.1 for async trait support

#### Optional Dependencies

- **tracing** 0.1 + **tracing-subscriber** 0.3 for structured logging
- **actix-web** 4.4 for web framework integration
- **clap** 4.0 for CLI interface
- **config** 0.14 + **toml** 0.8 + **serde_yaml** 0.9 for configuration

### Migration Guide

This is the initial release, so no migration is required. For new projects:

1. Add dependency: `rust-task-queue = "0.1"`
2. Choose appropriate features for your use case
3. Follow the Quick Start guide in README.md
4. Use provided examples for common integration patterns

### Acknowledgments

This release represents a comprehensive task queue solution built with:

- **Performance** as a first-class concern (sub-40ns operations)
- **Production readiness** with extensive testing and monitoring
- **Developer experience** with automatic registration and clear APIs
- **Flexibility** through feature gates and configuration options
- **Reliability** with robust error handling and graceful shutdown

