# List available recipes
default:
    @just --list

# Run all tests
test:
    cargo test

# Run clippy
lint:
    cargo clippy -- -D warnings

# Build the project
build:
    cargo build

# Run all checks (test + lint)
check: test lint

# Show coverage summary table
coverage:
    cargo llvm-cov report --summary-only

# Generate HTML coverage report and open in browser
coverage-html:
    mkdir -p coverage/html
    cargo llvm-cov --html --output-dir coverage/html
    open coverage/html/index.html 2>/dev/null || true

# Generate LCOV output for CI/editors
coverage-lcov:
    mkdir -p coverage
    cargo llvm-cov --lcov --output-path coverage/lcov.info

# Update golden test files
bless:
    BLESS=1 cargo test

# Build docs
doc:
    cargo doc --no-deps --open
