#!/bin/bash
set -euo pipefail

PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
CARGO_LLVM_COV_VERSION="${CARGO_LLVM_COV_VERSION:-0.8.6}"
exec env RS_CI_PROJECT_ROOT="$PROJECT_ROOT" CARGO_LLVM_COV_VERSION="$CARGO_LLVM_COV_VERSION" "$PROJECT_ROOT/.rs-ci/ci-check.sh" "$@"
