# List available recipes
default:
    @just --list

# Run all tests
test:
    cargo nextest run --workspace
    cargo test --doc --workspace

# Run all tests with junit output
test-ci:
    cargo nextest run --workspace --profile ci
    cargo test --doc --workspace

# Run doctests only
test-doc:
    cargo test --doc --workspace

# Check formatting
fmt-check:
    cargo fmt --all -- --check

# Run clippy
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Build the project
build:
    cargo build --workspace

# Run all checks (fmt + lint + test)
check: fmt-check lint test

# Verify the exact supported public API delta from 0.6.8.
semver-check:
    cargo test --test compatibility_semver_script
    scripts/check-semver.sh

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

# Generate LCOV output using nextest
coverage-nextest:
    mkdir -p coverage
    cargo llvm-cov nextest --lcov --output-path coverage/lcov.info

# Measure standalone TypeName materialization without a timing gate
bench-type-name-lowering:
    cargo bench --bench type_name_lowering

# Update golden test files
bless:
    BLESS=1 cargo test --workspace

# Build docs
doc:
    cargo doc --no-deps --open

# Build the mdbook
book:
    mdbook build docs

# Test mdbook code examples
book-test:
    cargo build
    cd docs && mdbook test -L ../target/debug/deps

# Serve the mdbook with live reload
book-serve:
    mdbook serve docs --open

# Publish to crates.io (macros crate first, then main crate)
publish:
    cargo publish -p sigil-stitch-macros
    cargo publish -p sigil-stitch
