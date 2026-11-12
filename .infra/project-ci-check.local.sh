#!/usr/bin/env bash
set -euo pipefail

project_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
cd "$project_root"

printf '%s\n' 'project-check: async reporting regression checks'
cargo run --locked --example async_reporting

model_test_dir=$(mktemp -d)
trap 'rm -rf -- "$model_test_dir"' EXIT
model_test_bin="$model_test_dir/reference-model-tests.exe"
printf '%s\n' 'project-check: metric delta reference model tests'
rustc --edition=2024 --test fuzz/fuzz_targets/metric_delta/reference_model.rs -o "$model_test_bin"
"$model_test_bin"
