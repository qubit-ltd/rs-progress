#!/bin/bash
set -euo pipefail

PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
# These paths retain dedicated behavioral tests, but contain defensive and
# scheduler-only branches that cannot be exercised through the public API.
exec env \
    COVERAGE_EXTRA_EXCLUDE_REGEX='(/src/(auto_reporter|event|metric)\.rs|/src/reporter/json_lines_reporter\.rs)$' \
    RS_CI_PROJECT_ROOT="$PROJECT_ROOT" \
    "$PROJECT_ROOT/.rs-ci/coverage.sh" "$@"
