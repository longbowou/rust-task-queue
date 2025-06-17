#!/bin/bash

# Rust Task Queue - Benchmark Verification Script
# This script verifies that all benchmarks compile correctly without running them

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${CYAN}🔍 Rust Task Queue - Benchmark Verification${NC}"
echo -e "${CYAN}==========================================${NC}"
echo

# Function to print step info
print_step() {
    echo -e "${YELLOW}▶ $1${NC}"
}

# Function to print success
print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

# Function to print error
print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# List of all benchmarks
BENCHMARKS=(
    "serialization"
    "worker_performance" 
    "redis_broker"
    "end_to_end"
    "autoscaler"
    "queue_operations"
    "task_processing"
)

print_step "Checking benchmark files exist..."

# Check if all benchmark files exist
missing_files=()
for bench in "${BENCHMARKS[@]}"; do
    if [ ! -f "benches/${bench}.rs" ]; then
        missing_files+=("benches/${bench}.rs")
    fi
done

if [ ${#missing_files[@]} -gt 0 ]; then
    print_error "Missing benchmark files:"
    for file in "${missing_files[@]}"; do
        echo "  - $file"
    done
    exit 1
fi

print_success "All benchmark files found"

print_step "Verifying Cargo.toml bench configurations..."

# Check if benchmarks are configured in Cargo.toml
missing_configs=()
for bench in "${BENCHMARKS[@]}"; do
    if ! grep -q "name = \"$bench\"" Cargo.toml; then
        missing_configs+=("$bench")
    fi
done

if [ ${#missing_configs[@]} -gt 0 ]; then
    print_error "Missing Cargo.toml configurations for:"
    for config in "${missing_configs[@]}"; do
        echo "  - $config"
    done
    exit 1
fi

print_success "All benchmark configurations found in Cargo.toml"

print_step "Checking benchmark compilation..."

# Try to compile each benchmark
failed_compilations=()
for bench in "${BENCHMARKS[@]}"; do
    echo -n "  Compiling $bench... "
    if cargo check --bench "$bench" >/dev/null 2>&1; then
        echo -e "${GREEN}OK${NC}"
    else
        echo -e "${RED}FAILED${NC}"
        failed_compilations+=("$bench")
    fi
done

if [ ${#failed_compilations[@]} -gt 0 ]; then
    print_error "Failed to compile benchmarks:"
    for bench in "${failed_compilations[@]}"; do
        echo "  - $bench"
        echo "    Run 'cargo check --bench $bench' for details"
    done
    exit 1
fi

print_success "All benchmarks compile successfully"

print_step "Verifying script permissions..."

if [ ! -x "scripts/run-benchmarks.sh" ]; then
    print_error "run-benchmarks.sh is not executable"
    echo "Run: chmod +x scripts/run-benchmarks.sh"
    exit 1
fi

print_success "Script permissions are correct"

print_step "Checking dependencies..."

# Check if criterion is in dependencies
if ! grep -q "criterion" Cargo.toml; then
    print_error "Criterion dependency not found in Cargo.toml"
    exit 1
fi

# Check if futures is available (needed for async benchmarks)
if ! grep -q "futures" Cargo.toml; then
    print_error "Futures dependency not found in Cargo.toml"
    exit 1
fi

print_success "Required dependencies found"

print_step "Generating benchmark summary..."

echo -e "\n${BLUE}📊 Benchmark Suite Summary${NC}"
echo -e "${BLUE}==========================${NC}"
echo -e "Total benchmarks: ${#BENCHMARKS[@]}"
echo
echo -e "${CYAN}Comprehensive Benchmarks (New):${NC}"
echo "  1. serialization     - MessagePack vs JSON performance"
echo "  2. worker_performance - Task execution and concurrency"
echo "  3. redis_broker      - Redis operations and pooling"
echo "  4. end_to_end        - Complete workflow testing"
echo "  5. autoscaler        - Auto-scaling performance"
echo
echo -e "${CYAN}Legacy Benchmarks:${NC}"
echo "  6. queue_operations  - Basic queue management"
echo "  7. task_processing   - Basic task serialization"
echo

echo -e "${CYAN}📁 Files Created:${NC}"
echo "  benches/serialization.rs       - 🆕 Comprehensive serialization benchmarks"
echo "  benches/worker_performance.rs  - 🆕 Worker and task execution benchmarks"
echo "  benches/redis_broker.rs        - 🆕 Redis operations benchmarks"
echo "  benches/end_to_end.rs          - 🆕 End-to-end workflow benchmarks"  
echo "  benches/autoscaler.rs          - 🆕 Auto-scaling performance benchmarks"
echo "  benches/README.md              - 🆕 Comprehensive documentation"
echo "  scripts/run-benchmarks.sh      - 🆕 Automated benchmark runner"
echo "  scripts/verify-benchmarks.sh   - 🆕 This verification script"
echo

echo -e "${CYAN}🚀 Usage:${NC}"
echo "  # Run all benchmarks:"
echo "  ./scripts/run-benchmarks.sh"
echo
echo "  # Run individual benchmark:"
echo "  cargo bench --bench serialization"
echo
echo "  # Verify compilation:"
echo "  ./scripts/verify-benchmarks.sh"
echo

echo -e "${CYAN}📈 Expected Performance Improvements:${NC}"
echo "  • MessagePack ~2.5x faster than JSON serialization"
echo "  • >1,000 tasks/second throughput for simple tasks"
echo "  • <100ms end-to-end latency for basic workflows"
echo "  • <5s auto-scaling response times"
echo "  • Linear scaling efficiency with worker count"
echo

print_success "🎉 Benchmark verification completed successfully!"
print_success "All ${#BENCHMARKS[@]} benchmarks are ready to run"

echo -e "\n${GREEN}✨ Next Steps:${NC}"
echo "1. Run full benchmark suite: ${YELLOW}./scripts/run-benchmarks.sh${NC}"
echo "2. View HTML reports: ${YELLOW}target/criterion/report/index.html${NC}"
echo "3. Check performance against targets in benches/README.md"
echo 