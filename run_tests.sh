#!/bin/bash

# Database Testing Script for api_proxy/crates/models
# This script sets up the test environment and runs comprehensive database tests

set -e

echo "🚀 Starting comprehensive database tests for api_proxy/crates/models"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if PostgreSQL is running
print_status "Checking PostgreSQL connection..."
if ! pg_isready -h localhost -p 5432 -U postgres > /dev/null 2>&1; then
    print_error "PostgreSQL is not running or not accessible"
    print_status "Please start PostgreSQL using: docker-compose up -d postgres"
    exit 1
fi
print_success "PostgreSQL is running"

# Create test database if it doesn't exist
print_status "Setting up test database..."
export PGPASSWORD=dev123
if ! psql -h localhost -p 5432 -U postgres -lqt | cut -d \| -f 1 | grep -qw api_proxy_test; then
    print_status "Creating test database 'api_proxy_test'..."
    createdb -h localhost -p 5432 -U postgres api_proxy_test
    print_success "Test database created"
else
    print_status "Test database 'api_proxy_test' already exists"
fi

# Load test environment variables
print_status "Loading test environment configuration..."
if [ -f .env.test ]; then
    export $(cat .env.test | grep -v '^#' | xargs)
    print_success "Test environment loaded"
else
    print_warning ".env.test not found, using default settings"
    export DATABASE_URL="postgresql://postgres:dev123@localhost:5432/api_proxy_test"
fi

# Run migrations on test database
print_status "Running database migrations..."
cd crates/migration
if cargo run -- up; then
    print_success "Migrations completed successfully"
else
    print_error "Migration failed"
    exit 1
fi
cd ../..

# Run all tests with detailed output
print_status "Running comprehensive test suite..."

echo ""
echo "=========================================="
echo "🧪 RUNNING DATABASE CONNECTION TESTS"
echo "=========================================="
if cargo test -p models db_tests --verbose -- --nocapture; then
    print_success "Database connection tests passed"
else
    print_error "Database connection tests failed"
    exit 1
fi

echo ""
echo "=========================================="
echo "🔄 RUNNING CRUD OPERATION TESTS"
echo "=========================================="
if cargo test -p models crud_tests --verbose -- --nocapture; then
    print_success "CRUD operation tests passed"
else
    print_error "CRUD operation tests failed"
    exit 1
fi

echo ""
echo "=========================================="
echo "💾 RUNNING TRANSACTION TESTS"
echo "=========================================="
if cargo test -p models transaction_tests --verbose -- --nocapture; then
    print_success "Transaction tests passed"
else
    print_error "Transaction tests failed"
    exit 1
fi

echo ""
echo "=========================================="
echo "⚡ RUNNING PERFORMANCE TESTS"
echo "=========================================="
if cargo test -p models performance_tests --verbose -- --nocapture; then
    print_success "Performance tests passed"
else
    print_error "Performance tests failed"
    exit 1
fi

echo ""
echo "=========================================="
echo "🔗 RUNNING INTEGRATION TESTS"
echo "=========================================="
if cargo test -p models integration_tests --verbose -- --nocapture; then
    print_success "Integration tests passed"
else
    print_error "Integration tests failed"
    exit 1
fi

echo ""
echo "=========================================="
echo "🏃 RUNNING ALL TESTS TOGETHER"
echo "=========================================="
if cargo test -p models --verbose -- --nocapture; then
    print_success "All tests passed successfully!"
else
    print_error "Some tests failed"
    exit 1
fi

# Generate test coverage report (if cargo-tarpaulin is installed)
print_status "Generating test coverage report..."
if command -v cargo-tarpaulin &> /dev/null; then
    print_status "Running coverage analysis..."
    cargo tarpaulin -p models --out Html --output-dir target/coverage
    print_success "Coverage report generated in target/coverage/tarpaulin-report.html"
else
    print_warning "cargo-tarpaulin not installed. Install with: cargo install cargo-tarpaulin"
fi

# Clean up test database (optional)
read -p "Do you want to clean up the test database? (y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    print_status "Cleaning up test database..."
    dropdb -h localhost -p 5432 -U postgres api_proxy_test
    print_success "Test database cleaned up"
fi

echo ""
print_success "🎉 All database tests completed successfully!"
print_status "Test results summary:"
echo "  ✅ Database connection tests: PASSED"
echo "  ✅ CRUD operation tests: PASSED"
echo "  ✅ Transaction handling tests: PASSED"
echo "  ✅ Performance tests: PASSED"
echo "  ✅ Integration tests: PASSED"
echo ""
print_status "Your database models are ready for production! 🚀"