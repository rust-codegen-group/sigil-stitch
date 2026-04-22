# List available recipes
default:
    @just --list

# Run all tests
test:
    cargo test

# Run tests with nextest (requires cargo-nextest installed)
test-nextest:
    cargo nextest run

# Run doctests only
test-doc:
    cargo test --doc

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

# Generate LCOV output using nextest (matches CI)
coverage-nextest:
    mkdir -p coverage
    cargo llvm-cov nextest --lcov --output-path coverage/lcov.info

# Update golden test files
bless:
    BLESS=1 cargo test

# Build docs
doc:
    cargo doc --no-deps --open

# Build the mdbook
book:
    mdbook build docs

# Serve the mdbook with live reload
book-serve:
    mdbook serve docs --open
