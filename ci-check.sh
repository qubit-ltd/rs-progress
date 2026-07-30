#!/bin/bash
set -euo pipefail

PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
# Rust's coverage instrumentation expands the scoped background-loop closures
# into synthetic regions. Keep the project threshold below those non-source
# regions while retaining the shared CI defaults for every other metric.
exec env \
    COVERAGE_EXTRA_EXCLUDE_REGEX='(/src/(auto_reporter|event|metric)\.rs|/src/reporter/json_lines_reporter\.rs)$' \
    MIN_REGION_COVERAGE=94 \
    RS_CI_PROJECT_ROOT="$PROJECT_ROOT" \
    "$PROJECT_ROOT/.rs-ci/ci-check.sh" "$@"
