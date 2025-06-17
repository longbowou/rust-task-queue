#!/bin/bash

# Cleanup script for Redis test containers
# Use this if run-tests.sh fails and leaves containers running

set -e

CONTAINER_NAME="rust-task-queue-bench-redis"
REDIS_PORT="6379"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

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

print_status "Redis Test Container Cleanup"
print_status "================================"

# Check if container exists and is running
if docker ps --filter "name=${CONTAINER_NAME}" --filter "status=running" | grep -q ${CONTAINER_NAME}; then
    print_warning "Found running Redis test container"
    print_status "Stopping container: ${CONTAINER_NAME}"
    
    if docker stop ${CONTAINER_NAME} > /dev/null 2>&1; then
        print_success "Container stopped successfully"
    else
        print_error "Failed to stop container"
    fi
fi

# Check if container exists (stopped)
if docker ps -a --filter "name=${CONTAINER_NAME}" | grep -q ${CONTAINER_NAME}; then
    print_status "Removing container: ${CONTAINER_NAME}"
    
    if docker rm ${CONTAINER_NAME} > /dev/null 2>&1; then
        print_success "Container removed successfully"
    else
        print_error "Failed to remove container"
    fi
fi

# Check if port is still occupied
if lsof -Pi :${REDIS_PORT} -sTCP:LISTEN -t >/dev/null 2>&1; then
    print_warning "Port ${REDIS_PORT} is still in use"
    print_status "Process using port ${REDIS_PORT}:"
    lsof -Pi :${REDIS_PORT} -sTCP:LISTEN
    print_status "You may need to manually stop the process using this port"
else
    print_success "Port ${REDIS_PORT} is now available"
fi

# Check for any other redis containers
OTHER_REDIS=$(docker ps -a --filter "ancestor=redis" --format "table {{.Names}}\t{{.Status}}" | grep -v "NAMES" | grep -v "${CONTAINER_NAME}" || true)

if [ ! -z "$OTHER_REDIS" ]; then
    print_status ""
    print_status "Other Redis containers found:"
    echo "$OTHER_REDIS"
    print_status ""
    print_status "To remove all Redis containers:"
    print_status "docker stop \$(docker ps -q --filter ancestor=redis) 2>/dev/null || true"
    print_status "docker rm \$(docker ps -aq --filter ancestor=redis) 2>/dev/null || true"
fi

print_status ""
print_success "Cleanup completed!"
print_status "You can now run ./scripts/run-tests.sh again" 