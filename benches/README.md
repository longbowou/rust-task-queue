# Rust Task Queue - Benchmark Suite

This directory contains a comprehensive benchmark suite for the Rust Task Queue project, designed to measure and monitor performance across all critical components and workflows.

## Overview

The benchmark suite consists of 7 specialized benchmark modules that test different aspects of the task queue system:

| Benchmark | Focus Area | Key Metrics |
|-----------|------------|-------------|
| **serialization** | MessagePack vs JSON performance | Serialization speed, throughput, payload sizes |
| **worker_performance** | Task execution & concurrency | Worker throughput, concurrent execution, registry ops |
| **redis_broker** | Redis operations & pooling | Enqueue/dequeue speed, batch ops, connection pooling |
| **end_to_end** | Complete workflow testing | End-to-end latency, scaling behavior, memory usage |
| **autoscaler** | Auto-scaling performance | Scaling decisions, response time, load patterns |
| **queue_operations** | Basic queue management (legacy) | Queue config operations |
| **task_processing** | Basic task operations (legacy) | Simple serialization benchmarks |

## Quick Start

### Run All Benchmarks
```bash
# Run the complete benchmark suite inside the terminal
./scripts/run-benchmarks.sh

# Run the complete benchmark suite to exports reports
./scripts/run-benchmarks-with-reports.sh

# Get help
./scripts/run-benchmarks-with-reports.sh --help
```

### Run Individual Benchmarks
```bash
# Start Redis first
docker run -d --name redis-bench -p 6379:6379 redis:7-alpine

# Run specific benchmark
export REDIS_URL="redis://localhost:6379"
cargo bench --bench serialization
cargo bench --bench worker_performance
cargo bench --bench redis_broker
cargo bench --bench end_to_end
cargo bench --bench autoscaler
```

## Benchmark Details

### 1. Serialization Benchmark (`serialization.rs`)

**Purpose**: Compare MessagePack vs JSON serialization performance across different payload sizes.

**Test Cases**:
- Small tasks (simple struct with ID and short string)
- Medium tasks (metadata, 1KB payload)
- Large tasks (complex nested data, 10KB payload)
- Batch serialization throughput (10-1000 items)

**Key Metrics**:
- Serialization time per operation
- Deserialization time per operation
- Throughput (items per second)
- Memory efficiency

**Expected Results**:
- MessagePack should be ~2.5x faster than JSON
- Memory usage should be ~30% lower for MessagePack
- Performance advantage should increase with payload size

### 2. Worker Performance Benchmark (`worker_performance.rs`)

**Purpose**: Test task execution performance, concurrency handling, and worker management efficiency.

**Test Cases**:
- Task execution types (fast, slow, CPU-intensive, memory-intensive)
- Concurrent task execution (1-16 workers)
- Batch processing (sequential vs parallel)
- Task registry operations
- Error handling and retry logic
- Timeout handling

**Key Metrics**:
- Task execution time by type
- Concurrent processing throughput
- Registry lookup performance
- Error handling overhead

**Expected Results**:
- Fast tasks: <1ms execution time
- Concurrent scaling should be near-linear up to CPU cores
- Registry lookups: <100ns
- Error handling should add minimal overhead

### 3. Redis Broker Benchmark (`redis_broker.rs`)

**Purpose**: Measure Redis operations performance, connection pooling efficiency, and message handling.

**Test Cases**:
- Basic operations (enqueue, dequeue, queue length)
- Batch operations (10-500 items)
- Concurrent operations (2-16 clients)
- Message sizes (100B to 100KB)
- Priority queue operations
- Connection pooling efficiency

**Key Metrics**:
- Single operation latency
- Batch operation throughput
- Concurrent access performance
- Connection establishment time

**Expected Results**:
- Single operations: <5ms
- Batch operations: >1000 ops/second
- Connection pooling should reduce latency by ~50%
- Concurrent access should scale linearly with connections

### 4. End-to-End Benchmark (`end_to_end.rs`)

**Purpose**: Test complete task queue workflows and real-world usage patterns.

**Test Cases**:
- Task queue creation (minimal vs full-featured)
- Single task workflows (enqueue to completion)
- Batch processing (10-500 tasks)
- Concurrent enqueueing
- Worker scaling (1-8 workers)
- Priority processing
- Latency measurement
- Memory usage patterns

**Key Metrics**:
- End-to-end task latency
- System throughput
- Memory footprint
- Scaling efficiency

**Expected Results**:
- Simple task latency: <100ms
- System throughput: >1000 tasks/second
- Memory usage should be predictable and bounded
- Scaling should improve throughput proportionally

### 5. Autoscaler Benchmark (`autoscaler.rs`)

**Purpose**: Test auto-scaling performance, decision accuracy, and response times.

**Test Cases**:
- Scaling decision logic (different load scenarios)
- Load pattern responses (burst, sustained, gradual)
- CPU vs IO task scaling
- Scaling response times
- Concurrent scaling decisions
- Threshold tuning
- Cooldown period logic

**Key Metrics**:
- Scaling decision time
- Response time to load changes
- Decision accuracy
- Resource efficiency

**Expected Results**:
- Scaling decisions: <100ms
- Scale-up response: <5 seconds
- Scale-down response: <60 seconds (with cooldown)
- Minimal resource overhead for monitoring

### 6. Queue Operations Benchmark (`queue_operations.rs`) - Legacy

**Purpose**: Basic queue management operations (maintained for compatibility).

**Test Cases**:
- Queue manager creation
- Queue configuration retrieval
- Autoscaler config creation

### 7. Task Processing Benchmark (`task_processing.rs`) - Legacy

**Purpose**: Basic task serialization (superseded by serialization benchmark).

**Test Cases**:
- Simple task serialization/deserialization

## Performance Targets

Based on project requirements and industry standards:

### Serialization Performance
- **MessagePack**: <50ns per operation for small payloads
- **JSON**: <150ns per operation for small payloads
- **Throughput**: >10,000 serializations per second
- **Memory**: MessagePack should use ~30% less memory

### Task Processing Performance
- **Fast Tasks**: <1ms execution time
- **Task Registry**: <100ns lookup time
- **Concurrent Processing**: Near-linear scaling to CPU cores
- **System Throughput**: >1,000 tasks/second for simple tasks

### Redis Operations Performance
- **Single Operations**: <10ms latency
- **Batch Operations**: >1,000 operations/second
- **Connection Pooling**: <1ms connection acquisition
- **Concurrent Access**: Linear scaling with connection count

### End-to-End Performance
- **Task Latency**: <100ms for simple tasks
- **System Throughput**: >1,000 tasks/second
- **Memory Usage**: <10MB baseline + predictable growth
- **Worker Scaling**: 90%+ efficiency improvement per worker

### Auto-scaling Performance
- **Decision Time**: <100ms for scaling decisions
- **Scale-up Response**: <5 seconds
- **Scale-down Response**: <60 seconds (with cooldown)
- **Monitoring Overhead**: <1% CPU usage

## 🔧 Configuration

### Environment Variables
- `REDIS_URL`: Redis connection string (default: `redis://localhost:6379`)
- `REDIS_PORT`: Redis port for benchmark script (default: `6380`)

### Benchmark Configuration
Each benchmark can be customized by modifying the respective source file:

- **Payload sizes**: Adjust test data sizes in benchmark functions
- **Concurrency levels**: Modify the concurrency arrays (e.g., `[1, 2, 4, 8, 16]`)
- **Batch sizes**: Change batch size arrays (e.g., `[10, 50, 100, 500]`)
- **Timeout values**: Adjust timeout durations for long-running tests

### Criterion Configuration
Benchmarks use [Criterion.rs](https://bheisler.github.io/criterion.rs/) with:
- HTML report generation enabled
- Statistical analysis and regression detection
- Configurable measurement time and sample sizes
- Throughput measurements where applicable

## Output and Reports

### Automated Reports
Running `./scripts/run-benchmarks-with-reports.sh` generates:

1. **HTML Reports**: `target/criterion/report/index.html`
   - Interactive charts and graphs
   - Statistical analysis
   - Performance regression detection
   - Detailed timing distributions

2. **Summary Report**: `target/benchmark_reports/benchmark_summary.md`
   - Executive summary of all benchmarks
   - Performance targets vs actual results
   - Key metrics and trends
   - Failure analysis

3. **Raw Logs**: `target/benchmark_reports/*_output.log`
   - Complete benchmark output
   - Error messages and warnings
   - Detailed timing information

### Manual Analysis
For detailed analysis, examine:
- `target/criterion/<benchmark_name>/`: Individual benchmark results
- Flamegraphs and profiling data (if enabled)
- Memory usage reports
- CPU utilization metrics

## Troubleshooting

### Common Issues

**Redis Connection Failed**
```bash
# Check if Redis is running
docker ps | grep redis

# Start Redis manually
docker run -d --name redis-bench -p 6379:6379 redis:7-alpine

# Check Redis connectivity
redis-cli -h localhost -p 6379 ping
```

**Benchmark Timeout**
```bash
# Increase timeout in run-benchmarks-with-reports.sh
# Or run individual benchmarks with more time
cargo bench --bench <benchmark_name> -- --measurement-time 60
```

**Memory Issues**
```bash
# Reduce batch sizes in benchmark source files
# Or run with limited memory
docker run --memory=4g --name redis-bench -p 6379:6379 redis:7-alpine
```

**Port Conflicts**
```bash
# Use different port
./scripts/run-benchmarks-with-reports.sh --redis-port 6381

# Or set environment variable
REDIS_PORT=6382 ./scripts/run-benchmarks-with-reports.sh
```

### Performance Investigation

**Slower than Expected Results**:
1. Check system load during benchmarking
2. Ensure Redis is running locally (not remote)
3. Verify no other applications are using significant resources
4. Consider running benchmarks on dedicated hardware
5. Check for debug builds (should use release builds)

**Inconsistent Results**:
1. Run benchmarks multiple times
2. Check for background processes
3. Ensure consistent system state
4. Consider using Criterion's statistical analysis features
5. Look for thermal throttling on mobile hardware

## Advanced Usage

### Custom Benchmark Development

To add a new benchmark:

1. Create `benches/your_benchmark.rs`:
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rust_task_queue::prelude::*;

fn bench_your_feature(c: &mut Criterion) {
    c.bench_function("your_test", |b| {
        b.iter(|| {
            // Your benchmark code here
            black_box(result);
        })
    });
}

criterion_group!(benches, bench_your_feature);
criterion_main!(benches);
```

2. Add to `Cargo.toml`:
```toml
[[bench]]
name = "your_benchmark"
harness = false
```

3. Update `scripts/run-benchmarks.sh` to include your benchmark.

### Profiling Integration

Enable profiling for detailed analysis:

```bash
# Install perf (Linux) or Instruments (macOS)
cargo bench --bench <benchmark> -- --profile-time=10

# Generate flamegraphs
cargo install cargo-flamegraph
cargo flamegraph --bench <benchmark>
```

### CI/CD Integration

For continuous performance monitoring:

```yaml
# .github/workflows/benchmarks.yml
- name: Run Benchmarks
  run: |
    ./scripts/run-benchmarks.sh
    # Upload results to performance tracking system
```

## References

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/)
- [Rust Performance Optimization](https://nnethercote.github.io/perf-book/)
- [Redis Benchmarking](https://redis.io/docs/management/optimization/benchmarks/)
- [MessagePack Specification](https://msgpack.org/)

## Contributing

When adding new benchmarks:

1. Follow the existing patterns and naming conventions
2. Include comprehensive test cases covering edge cases
3. Add appropriate documentation and comments
4. Update this README with new benchmark descriptions
5. Ensure benchmarks are deterministic and reproducible
6. Test on multiple platforms if possible

For performance regressions:
1. Investigate the root cause
2. Create targeted micro-benchmarks
3. Profile the problematic code paths
4. Document findings and optimizations 