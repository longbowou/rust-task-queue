#!/bin/bash

# Comprehensive test script for rust-task-queue with Redis container
# This script starts a Redis container, runs all tests, and cleans up
# Improved with robust cleanup and error handling

set -e

# Configuration
CONTAINER_NAME="redis-test"
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
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Cleanup function that will be called on script exit
cleanup() {
    local exit_code=$?
    print_status ""
    print_status "🧹 Cleaning up Redis container..."
    
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
    
    print_status "⏳ Waiting for Redis to be ready..."
    
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

# Function to run a test command with error handling
run_test() {
    local test_name="$1"
    local test_command="$2"
    
    print_status ""
    print_status "🧪 Running ${test_name}..."
    
    if eval "$test_command"; then
        print_success "${test_name}: PASSED"
        return 0
    else
        print_error "${test_name}: FAILED"
        return 1
    fi
}

# Main execution starts here
print_status "🚀 Starting comprehensive test suite for rust-task-queue"
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
print_status "🐳 Starting Redis container..."
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
print_status "🔍 Testing Redis connection..."
if docker exec ${CONTAINER_NAME} redis-cli ping | grep -q "PONG"; then
    print_success "Redis connection verified!"
else
    print_error "Redis is not responding to ping"
    exit 1
fi

# Initialize test results tracking
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Run all test suites
test_suites=(
    "Clippy checks|cargo clippy --all-targets --all-features -- -D warnings"
    "Unit tests (121)|cargo test --lib -- --test-threads=1"
    "Integration tests (9)|cargo test --test integration_tests"
    "Error scenario tests (9)|cargo test --test error_scenarios"
    "Performance tests (6)|cargo test --test performance_tests"
    "Security tests (7)|cargo test --test security_tests"
    "Build check|cargo build --all-targets --all-features"
    "Benchmarks (5)|cargo bench"
)

for test_suite in "${test_suites[@]}"; do
    IFS='|' read -r test_name test_command <<< "$test_suite"
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    if run_test "$test_name" "$test_command"; then
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        FAILED_TESTS=$((FAILED_TESTS + 1))
        # Continue with other tests instead of exiting immediately
        print_warning "Continuing with remaining tests..."
    fi
done

# Print comprehensive summary
print_status ""
print_status "📊 Comprehensive Test Summary:"
print_status "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ $FAILED_TESTS -eq 0 ]; then
    print_success "🎉 All tests completed successfully!"
    print_success "   ✅ Total test suites: $TOTAL_TESTS"
    print_success "   ✅ Passed: $PASSED_TESTS"
    print_success "   ✅ Failed: $FAILED_TESTS"
else
    print_warning "⚠️  Some tests failed!"
    print_status "   📊 Total test suites: $TOTAL_TESTS"
    print_success "   ✅ Passed: $PASSED_TESTS"
    print_error "   ❌ Failed: $FAILED_TESTS"
fi

print_status ""
print_status "🔍 Test Coverage Areas:"
print_status "   • Core functionality and task processing (121 unit tests)"
print_status "   • End-to-end workflows (9 integration tests)"
print_status "   • Error handling and edge cases (9 error scenario tests)"
print_status "   • Performance and load testing (6 performance tests)"
print_status "   • Security and injection protection (7 security tests)"
print_status "   • Performance benchmarking (5 benchmark tests)"
print_status "   • Auto-scaling and worker management"
print_status "   • Redis connection handling and recovery"
print_status "   • Memory management and resource leaks"
print_status "   • Concurrent access and race condition safety"
print_status "   • Configuration validation and compliance"

print_status ""
print_status "🛠️  Development Commands:"
print_status "   Run individual test suites:"
print_status "   • cargo test --lib                    # Unit tests"
print_status "   • cargo test --test integration_tests # Integration tests"
print_status "   • cargo test --test error_scenarios   # Error handling tests"
print_status "   • cargo test --test performance_tests # Performance tests"
print_status "   • cargo test --test security_tests    # Security tests"
print_status "   • cargo bench                         # Benchmarks"
print_status "   • cargo clippy --all-targets --all-features -- -D warnings"

# Exit with appropriate code
if [ $FAILED_TESTS -eq 0 ]; then
    exit 0
else
    exit 1
fi 