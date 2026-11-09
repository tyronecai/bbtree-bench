#!/usr/bin/env bash
set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
remote_host="${REMOTE_HOST:-192.168.1.8}"
remote_dir="${REMOTE_DIR:-/tmp/bbtree-kv-bench-build}"

mkdir -p "${project_root}/target/bench"

rsync -az --delete \
  --exclude '.git' \
  --exclude 'target' \
  --exclude 'abseil-cpp-*' \
  --exclude 'results/*.tsv' \
  --exclude 'results/*.md' \
  "${project_root}/" "${remote_host}:${remote_dir}/"

ssh "${remote_host}" bash -s -- "${remote_dir}" <<'REMOTE_SCRIPT'
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"
cd "$1"

cargo test --release
cargo build --release --bins
REMOTE_SCRIPT

scp -q "${remote_host}:${remote_dir}/target/release/bbtree_bench" \
  "${project_root}/target/bench/bbtree_bench"
scp -q "${remote_host}:${remote_dir}/target/release/std_btree_bench" \
  "${project_root}/target/bench/std_btree_bench"

cpp_build_dir="${project_root}/target/cpp-build-gcc13"
cmake_command=(cmake)
if command -v scl >/dev/null 2>&1 &&
   scl -l 2>/dev/null | grep -qx 'gcc-toolset-13'; then
  cmake_command=(scl enable gcc-toolset-13 -- cmake)
else
  echo "GCC 10 or newer is required to build Abseil" >&2
  exit 1
fi
cmake_args=()
if command -v ninja >/dev/null 2>&1; then
  cmake_args=(-G Ninja)
fi
"${cmake_command[@]}" "${cmake_args[@]}" \
  -S "${project_root}/cpp" \
  -B "${cpp_build_dir}" \
  -DCMAKE_BUILD_TYPE=Release
"${cmake_command[@]}" --build "${cpp_build_dir}" --parallel "$(nproc)"
cmake -E copy_if_different \
  "${cpp_build_dir}/bin/std_map_bench" \
  "${project_root}/target/bench/std_map_bench"
cmake -E copy_if_different \
  "${cpp_build_dir}/bin/absl_btree_bench" \
  "${project_root}/target/bench/absl_btree_bench"

chmod +x \
  "${project_root}/target/bench/bbtree_bench" \
  "${project_root}/target/bench/std_btree_bench" \
  "${project_root}/target/bench/std_map_bench" \
  "${project_root}/target/bench/absl_btree_bench"

echo "Built Rust binaries remotely and C++ binaries locally into ${project_root}/target/bench"

