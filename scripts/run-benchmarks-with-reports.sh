#!/bin/bash

# Rust Task Queue - Comprehensive Benchmark Runner
# This script runs all benchmark suites with proper Redis setup and generates reports

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
REDIS_CONTAINER_NAME="rust-task-queue-bench-redis"
REDIS_PORT=${REDIS_PORT:-6379}
REDIS_URL="redis://localhost:${REDIS_PORT}"
BENCHMARK_OUTPUT_DIR="target/criterion"
BENCHMARK_REPORT_DIR="target/benchmark_reports"

# Create benchmark reports directory
mkdir -p "$BENCHMARK_REPORT_DIR"

echo -e "${CYAN}Rust Task Queue - Comprehensive Benchmark Suite${NC}"
echo -e "${CYAN}=================================================${NC}"
echo

# Function to print section headers
print_section() {
    echo -e "\n${BLUE}$1${NC}"
    echo -e "${BLUE}$(printf '=%.0s' $(seq 1 ${#1}))${NC}"
}

# Function to print step info
print_step() {
    echo -e "${YELLOW}▶ $1${NC}"
}

# Function to print success
print_success() {
    echo -e "${GREEN}$1${NC}"
}

# Function to print error
print_error() {
    echo -e "${RED}$1${NC}"
}

# Function to check if Redis container is running
is_redis_running() {
    docker ps --format "table {{.Names}}" | grep -q "^${REDIS_CONTAINER_NAME}$"
}

# Function to start Redis container
start_redis() {
    print_step "Starting Redis container on port $REDIS_PORT..."
    
    # Stop and remove existing container if it exists
    if docker ps -a --format "table {{.Names}}" | grep -q "^${REDIS_CONTAINER_NAME}$"; then
        docker stop "$REDIS_CONTAINER_NAME" >/dev/null 2>&1 || true
        docker rm "$REDIS_CONTAINER_NAME" >/dev/null 2>&1 || true
    fi
    
    # Start new Redis container
    docker run -d \
        --name "$REDIS_CONTAINER_NAME" \
        -p "$REDIS_PORT:6379" \
        redis:7-alpine \
        redis-server --save "" --appendonly no \
        >/dev/null
    
    # Wait for Redis to be ready
    print_step "Waiting for Redis to be ready..."
    local retries=30
    while [ $retries -gt 0 ]; do
        if docker exec "$REDIS_CONTAINER_NAME" redis-cli ping >/dev/null 2>&1; then
            break
        fi
        sleep 1
        retries=$((retries - 1))
    done
    
    if [ $retries -eq 0 ]; then
        print_error "Redis failed to start within 30 seconds"
        exit 1
    fi
    
    print_success "Redis is ready on port $REDIS_PORT"
}

# Function to stop Redis container
stop_redis() {
    print_step "Stopping Redis container..."
    if is_redis_running; then
        docker stop "$REDIS_CONTAINER_NAME" >/dev/null 2>&1
        docker rm "$REDIS_CONTAINER_NAME" >/dev/null 2>&1
    fi
    print_success "Redis container stopped"
}

# Function to run a single benchmark
run_benchmark() {
    local bench_name=$1
    local description=$2
    
    print_step "Running $bench_name benchmark: $description"
    
    # Set environment variables for the benchmark
    export REDIS_URL="$REDIS_URL"
    
    # Run the benchmark
    if cargo bench --bench "$bench_name" 2>&1 | tee "$BENCHMARK_REPORT_DIR/${bench_name}_output.log"; then
        print_success "$bench_name benchmark completed"
        return 0
    else
        print_error "$bench_name benchmark failed"
        return 1
    fi
}

# Function to generate benchmark summary
generate_summary() {
    local summary_file="$BENCHMARK_REPORT_DIR/benchmark_summary.md"
    
    print_step "Generating benchmark summary..."
    
    cat > "$summary_file" << EOF
# Rust Task Queue - Benchmark Results Summary

Generated on: $(date)
Redis URL: $REDIS_URL

## Benchmark Suites Executed

EOF

    # Add details for each benchmark
    for bench in serialization worker_performance redis_broker end_to_end autoscaler queue_operations task_processing; do
        if [ -f "$BENCHMARK_REPORT_DIR/${bench}_output.log" ]; then
            echo "### ${bench^} Benchmark" >> "$summary_file"
            echo "" >> "$summary_file"
            echo "Status: Completed" >> "$summary_file"
            echo "" >> "$summary_file"
            
            # Extract some key metrics if available
            if grep -q "time:" "$BENCHMARK_REPORT_DIR/${bench}_output.log"; then
                echo "Key Results:" >> "$summary_file"
                echo "\`\`\`" >> "$summary_file"
                grep "time:" "$BENCHMARK_REPORT_DIR/${bench}_output.log" | head -10 >> "$summary_file"
                echo "\`\`\`" >> "$summary_file"
            fi
            echo "" >> "$summary_file"
        else
            echo "### ${bench^} Benchmark" >> "$summary_file"
            echo "" >> "$summary_file"
            echo "Status: Failed or not run" >> "$summary_file"
            echo "" >> "$summary_file"
        fi
    done
    
    cat >> "$summary_file" << EOF

## Benchmark Descriptions

### 1. Serialization Benchmark
- **Purpose**: Compare MessagePack vs JSON serialization performance
- **Key Metrics**: Serialization/deserialization speed, throughput
- **Expected Results**: MessagePack should be ~2.5x faster than JSON

### 2. Worker Performance Benchmark  
- **Purpose**: Test task execution, concurrency, and worker management
- **Key Metrics**: Task execution time, concurrent processing, registry operations
- **Focus Areas**: CPU vs memory intensive tasks, error handling

### 3. Redis Broker Benchmark
- **Purpose**: Test Redis operations, connection pooling, message handling
- **Key Metrics**: Enqueue/dequeue performance, batch operations, concurrent access
- **Focus Areas**: Different message sizes, priority queues

### 4. End-to-End Benchmark
- **Purpose**: Complete task queue workflow performance
- **Key Metrics**: End-to-end latency, throughput, scaling behavior
- **Focus Areas**: Real-world usage patterns, memory efficiency

### 5. Autoscaler Benchmark
- **Purpose**: Test auto-scaling performance and decision making
- **Key Metrics**: Scaling response time, decision accuracy, resource efficiency
- **Focus Areas**: Different load patterns, threshold tuning

### 6. Queue Operations Benchmark (Legacy)
- **Purpose**: Basic queue manager operations
- **Key Metrics**: Queue configuration, basic operations
- **Status**: Legacy benchmark, superseded by newer suites

### 7. Task Processing Benchmark (Legacy)
- **Purpose**: Basic task serialization
- **Key Metrics**: Simple task serialization/deserialization
- **Status**: Legacy benchmark, superseded by serialization benchmark

## Performance Targets

Based on project requirements:
- **Serialization**: MessagePack <50ns, JSON <150ns
- **Task Throughput**: >1000 tasks/second for simple tasks
- **Redis Operations**: <10ms for single operations
- **End-to-End Latency**: <100ms for simple tasks
- **Scaling Response**: <5s for autoscaler decisions

## HTML Reports

Detailed HTML reports are available in:
\`target/criterion/\`

Open \`target/criterion/report/index.html\` in your browser for interactive results.

EOF

    print_success "Benchmark summary generated: $summary_file"
}

# Function to cleanup on exit
cleanup() {
    print_step "Cleaning up..."
    stop_redis
    print_success "Cleanup completed"
}

# Trap cleanup on script exit
trap cleanup EXIT INT TERM

# Main execution
main() {
    # Check prerequisites
    print_section "Prerequisites Check"
    
    if ! command -v docker &> /dev/null; then
        print_error "Docker is required but not installed"
        exit 1
    fi
    
    if ! command -v cargo &> /dev/null; then
        print_error "Cargo is required but not installed"
        exit 1
    fi
    
    print_success "Prerequisites check passed"
    
    # Start Redis
    print_section "Redis Setup"
    start_redis
    
    # Build project first
    print_section "Project Build"
    print_step "Building project with all features..."
    if cargo build --all-features --release; then
        print_success "Project build completed"
    else
        print_error "Project build failed"
        exit 1
    fi
    
    # Run benchmarks
    print_section "Benchmark Execution"
    
    local failed_benchmarks=()
    
    # Core performance benchmarks (new comprehensive suite)
    if ! run_benchmark "serialization" "MessagePack vs JSON serialization performance"; then
        failed_benchmarks+=("serialization")
    fi
    
    if ! run_benchmark "worker_performance" "Task execution and worker management"; then
        failed_benchmarks+=("worker_performance")
    fi
    
    if ! run_benchmark "redis_broker" "Redis operations and connection pooling"; then
        failed_benchmarks+=("redis_broker")
    fi
    
    if ! run_benchmark "end_to_end" "Complete task queue workflow performance"; then
        failed_benchmarks+=("end_to_end")
    fi
    
    if ! run_benchmark "autoscaler" "Auto-scaling performance and decisions"; then
        failed_benchmarks+=("autoscaler")
    fi
    
    # Legacy benchmarks (for compatibility)
    if ! run_benchmark "queue_operations" "Legacy queue manager operations"; then
        failed_benchmarks+=("queue_operations")
    fi
    
    if ! run_benchmark "task_processing" "Legacy task processing operations"; then
        failed_benchmarks+=("task_processing")
    fi
    
    # Generate summary
    print_section "Results Summary"
    generate_summary
    
    # Final status
    print_section "Benchmark Results"
    
    local total_benchmarks=7
    local successful_benchmarks=$((total_benchmarks - ${#failed_benchmarks[@]}))
    
    echo -e "${CYAN}Benchmark Execution Summary${NC}"
    echo -e "Total Benchmarks: $total_benchmarks"
    echo -e "Successful: ${GREEN}$successful_benchmarks${NC}"
    echo -e "Failed: ${RED}${#failed_benchmarks[@]}${NC}"
    
    if [ ${#failed_benchmarks[@]} -gt 0 ]; then
        echo -e "\n${RED}Failed Benchmarks:${NC}"
        for bench in "${failed_benchmarks[@]}"; do
            echo -e "  $bench"
        done
    fi
    
    echo -e "\n${CYAN}Reports Generated:${NC}"
    echo -e "  Summary: $BENCHMARK_REPORT_DIR/benchmark_summary.md"
    echo -e "  HTML Reports: target/criterion/report/index.html"
    echo -e "  Raw Logs: $BENCHMARK_REPORT_DIR/*_output.log"
    
    if [ ${#failed_benchmarks[@]} -eq 0 ]; then
        echo -e "\n${GREEN}All benchmarks completed successfully!${NC}"
        exit 0
    else
        echo -e "\n${YELLOW}Some benchmarks failed. Check the logs for details.${NC}"
        exit 1
    fi
}

# Show help if requested
if [[ "$1" == "--help" || "$1" == "-h" ]]; then
    cat << EOF
Rust Task Queue - Comprehensive Benchmark Runner

USAGE:
    $0 [OPTIONS]

OPTIONS:
    -h, --help          Show this help message
    --redis-port PORT   Use custom Redis port (default: 6380)

ENVIRONMENT VARIABLES:
    REDIS_PORT          Redis port to use (default: 6380)

EXAMPLES:
    $0                  Run all benchmarks with default settings
    $0 --redis-port 6381    Run benchmarks with Redis on port 6381
    REDIS_PORT=6382 $0      Run benchmarks with Redis on port 6382

BENCHMARK SUITES:
    1. serialization     - MessagePack vs JSON performance comparison
    2. worker_performance - Task execution and worker management
    3. redis_broker      - Redis operations and connection pooling  
    4. end_to_end        - Complete workflow performance testing
    5. autoscaler        - Auto-scaling performance and decisions
    6. queue_operations  - Legacy queue manager operations
    7. task_processing   - Legacy task processing operations

OUTPUT:
    - HTML reports: target/criterion/report/index.html
    - Summary: benchmark_reports/benchmark_summary.md
    - Raw logs: benchmark_reports/*_output.log

EOF
    exit 0
fi

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --redis-port)
            REDIS_PORT="$2"
            REDIS_URL="redis://localhost:${REDIS_PORT}"
            shift 2
            ;;
        *)
            print_error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Run main function
main 