#!/bin/bash

# Dedicated benchmark runner for rust-task-queue with Redis container
# This script starts a Redis container, runs all benchmarks, and cleans up
# Extracted from run-tests.sh for dedicated benchmark execution

set -e

# Configuration
CONTAINER_NAME="rust-task-queue-bench-redis"
REDIS_PORT="6379"
REDIS_IMAGE="redis:7-alpine"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}$1${NC}"
}

print_success() {
    echo -e "${GREEN}$1${NC}"
}

print_warning() {
    echo -e "${YELLOW}$1${NC}"
}

print_error() {
    echo -e "${RED}$1${NC}"
}

# Cleanup function that will be called on script exit
cleanup() {
    local exit_code=$?
    print_status ""
    print_status "Cleaning up Redis container..."
    
    # Stop and remove container if it exists, suppress errors
    if docker ps -q --filter "name=${CONTAINER_NAME}" | grep -q .; then
        print_status "Stopping Redis container..."
        docker stop ${CONTAINER_NAME} > /dev/null 2>&1 || true
    fi
    
    if docker ps -aq --filter "name=${CONTAINER_NAME}" | grep -q .; then
        print_status "Removing Redis container..."
        docker rm ${CONTAINER_NAME} > /dev/null 2>&1 || true
    fi
    
    if [ $exit_code -eq 0 ]; then
        print_success "Cleanup completed successfully!"
    else
        print_warning "Cleanup completed (script exited with code $exit_code)"
    fi
}

# Set up trap to ensure cleanup happens on script exit (success, failure, or interruption)
trap cleanup EXIT

# Function to check if port is available
check_port() {
    if lsof -Pi :${REDIS_PORT} -sTCP:LISTEN -t >/dev/null 2>&1; then
        print_warning "Port ${REDIS_PORT} is already in use"
        print_status "Checking if it's our Redis container..."
        
        if docker ps --filter "name=${CONTAINER_NAME}" --filter "status=running" | grep -q ${CONTAINER_NAME}; then
            print_warning "Found existing Redis container. Stopping it first..."
            docker stop ${CONTAINER_NAME} > /dev/null 2>&1 || true
            docker rm ${CONTAINER_NAME} > /dev/null 2>&1 || true
            sleep 2
        else
            print_error "Port ${REDIS_PORT} is occupied by another process"
            print_status "Please stop the process using port ${REDIS_PORT} or change REDIS_PORT in this script"
            exit 1
        fi
    fi
}

# Function to wait for Redis to be ready with timeout
wait_for_redis() {
    local max_attempts=30
    local attempt=1
    
    print_status "Waiting for Redis to be ready..."
    
    while [ $attempt -le $max_attempts ]; do
        if docker exec ${CONTAINER_NAME} redis-cli ping > /dev/null 2>&1; then
            print_success "Redis is ready!"
            return 0
        fi
        
        echo -n "."
        sleep 1
        attempt=$((attempt + 1))
    done
    
    print_error "Redis failed to start within ${max_attempts} seconds"
    return 1
}

# Function to run a benchmark command with error handling
run_benchmark() {
    local bench_name="$1"
    local bench_command="$2"
    
    print_status ""
    print_status "Running ${bench_name}..."
    
    if eval "$bench_command"; then
        print_success "${bench_name}: COMPLETED"
        return 0
    else
        print_error "${bench_name}: FAILED"
        return 1
    fi
}

# Main execution starts here
print_status "Starting benchmark suite for rust-task-queue"
print_status "Container: ${CONTAINER_NAME} | Port: ${REDIS_PORT} | Image: ${REDIS_IMAGE}"
print_status ""

# Check if Docker is available
if ! command -v docker &> /dev/null; then
    print_error "Docker is not installed or not in PATH"
    exit 1
fi

# Check if port is available and clean up any existing containers
check_port

# Remove any existing container with the same name (in case of previous failures)
if docker ps -aq --filter "name=${CONTAINER_NAME}" | grep -q .; then
    print_warning "Removing existing container with name ${CONTAINER_NAME}"
    docker rm -f ${CONTAINER_NAME} > /dev/null 2>&1 || true
fi

# Start Redis container
print_status "Starting Redis container..."
if docker run -d --name ${CONTAINER_NAME} -p ${REDIS_PORT}:6379 ${REDIS_IMAGE} > /dev/null; then
    print_success "Redis container started successfully"
else
    print_error "Failed to start Redis container"
    exit 1
fi

# Wait for Redis to be ready
if ! wait_for_redis; then
    print_error "Redis startup failed"
    exit 1
fi

# Test Redis connection
print_status "Testing Redis connection..."
if docker exec ${CONTAINER_NAME} redis-cli ping | grep -q "PONG"; then
    print_success "Redis connection verified!"
else
    print_error "Redis is not responding to ping"
    exit 1
fi

# Initialize benchmark results tracking
TOTAL_BENCHMARKS=0
PASSED_BENCHMARKS=0
FAILED_BENCHMARKS=0

# Run all benchmark suites
benchmark_suites=(
#    "Serialization benchmarks|cargo bench --bench serialization"
#    "Worker performance benchmarks|cargo bench --bench worker_performance"
#    "Redis broker benchmarks|cargo bench --bench redis_broker"
#    "End-to-end benchmarks|cargo bench --bench end_to_end"
#    "Autoscaler benchmarks|cargo bench --bench autoscaler"
#    "Queue operations benchmarks|cargo bench --bench queue_operations"
#    "Task processing benchmarks|cargo bench --bench task_processing"
    "All benchmarks|cargo bench"
)

for benchmark_suite in "${benchmark_suites[@]}"; do
    IFS='|' read -r bench_name bench_command <<< "$benchmark_suite"
    TOTAL_BENCHMARKS=$((TOTAL_BENCHMARKS + 1))
    
    if run_benchmark "$bench_name" "$bench_command"; then
        PASSED_BENCHMARKS=$((PASSED_BENCHMARKS + 1))
    else
        FAILED_BENCHMARKS=$((FAILED_BENCHMARKS + 1))
        # Continue with other benchmarks instead of exiting immediately
        print_warning "Continuing with remaining benchmarks..."
    fi
done

# Print comprehensive summary
print_status ""
print_status "Benchmark Summary:"
print_status "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ $FAILED_BENCHMARKS -eq 0 ]; then
    print_success "All benchmarks completed successfully!"
    print_success "   Total benchmark suites: $TOTAL_BENCHMARKS"
    print_success "   Completed: $PASSED_BENCHMARKS"
    print_success "   Failed: $FAILED_BENCHMARKS"
else
    print_warning "Some benchmarks failed!"
    print_status "   Total benchmark suites: $TOTAL_BENCHMARKS"
    print_success "   Completed: $PASSED_BENCHMARKS"
    print_error "   Failed: $FAILED_BENCHMARKS"
fi

print_status ""
print_status "Benchmark Coverage Areas:"
print_status "   • Serialization performance (MessagePack vs JSON)"
print_status "   • Worker task execution and concurrency"
print_status "   • Redis broker operations and connection pooling"
print_status "   • End-to-end task queue workflow performance"
print_status "   • Auto-scaling decision making and responsiveness"
print_status "   • Basic queue operations and management"
print_status "   • Task processing serialization/deserialization"

print_status ""
print_status "Individual Benchmark Commands:"
print_status "   Run specific benchmark suites:"
print_status "   • cargo bench --bench serialization      # Serialization performance"
print_status "   • cargo bench --bench worker_performance # Worker and task execution"
print_status "   • cargo bench --bench redis_broker       # Redis operations"
print_status "   • cargo bench --bench end_to_end         # Full workflow"
print_status "   • cargo bench --bench autoscaler         # Auto-scaling performance"
print_status "   • cargo bench --bench queue_operations   # Queue management"
print_status "   • cargo bench --bench task_processing    # Task serialization"
print_status "   • cargo bench                            # All benchmarks"

print_status ""
print_status "Benchmark results are saved in: target/criterion/"
print_status "Open target/criterion/report/index.html for detailed reports"

# Exit with appropriate code
if [ $FAILED_BENCHMARKS -eq 0 ]; then
    exit 0
else
    exit 1
fi 