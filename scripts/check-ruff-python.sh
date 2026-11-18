#!/usr/bin/env bash
set -euo pipefail

export SIGIL_STITCH_RUFF_UVX=1
cargo test --test python ruff_compatibility -- --ignored --nocapture
