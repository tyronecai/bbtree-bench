#!/usr/bin/env bash
set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
count="${COUNT:-1000000}"
repeats="${REPEATS:-3}"

while (($#)); do
  case "$1" in
    --count)
      count="$2"
      shift 2
      ;;
    --repeats)
      repeats="$2"
      shift 2
      ;;
    *)
      echo "usage: $0 [--count N] [--repeats N]" >&2
      exit 2
      ;;
  esac
done

rust_binary="${project_root}/target/bench/bbtree_bench"
rust_std_binary="${project_root}/target/bench/std_btree_bench"
cpp_binary="${project_root}/target/bench/std_map_bench"
cpp_absl_binary="${project_root}/target/bench/absl_btree_bench"
if [[ ! -x "${rust_binary}" || ! -x "${rust_std_binary}" ||
      ! -x "${cpp_binary}" || ! -x "${cpp_absl_binary}" ]]; then
  echo "benchmark binaries are missing; run scripts/build_remote.sh first" >&2
  exit 1
fi

mkdir -p "${project_root}/results"
output_tsv="${project_root}/results/latest.tsv"
output_md="${project_root}/results/latest.md"
memory_tsv="${project_root}/results/memory-latest.tsv"
rust_output="$(mktemp)"
rust_std_output="$(mktemp)"
cpp_output="$(mktemp)"
cpp_absl_output="$(mktemp)"
memory_output="$(mktemp)"
trap 'rm -f "${rust_output}" "${rust_std_output}" "${cpp_output}" "${cpp_absl_output}" "${memory_output}"' EXIT

"${rust_binary}" --count "${count}" --repeats "${repeats}" >"${rust_output}"
"${rust_std_binary}" --count "${count}" --repeats "${repeats}" >"${rust_std_output}"
"${cpp_absl_binary}" --count "${count}" --repeats "${repeats}" >"${cpp_absl_output}"
"${cpp_binary}" --count "${count}" --repeats "${repeats}" >"${cpp_output}"

{
  printf 'implementation\tmap_type\tcount\trepeats\tphase\toperations\tmedian_ns\tns_per_op\tmops\tchecksum\tpeak_rss_kb\n'
  cat "${rust_output}" "${rust_std_output}" "${cpp_absl_output}" "${cpp_output}"
} >"${output_tsv}"

{
  printf 'implementation\tmap_type\tcount\tmap_rss_kb\tprocess_peak_rss_kb\n'
  "${rust_binary}" --count "${count}" --memory-only
  "${rust_std_binary}" --count "${count}" --memory-only
  "${cpp_absl_binary}" --count "${count}" --memory-only
  "${cpp_binary}" --count "${count}" --memory-only
} >"${memory_output}"
mv "${memory_output}" "${memory_tsv}"
chmod 644 "${memory_tsv}"

python3 "${project_root}/scripts/render_report.py" \
  "${output_tsv}" "${memory_tsv}" >"${output_md}"
cat "${output_tsv}"
echo
cat "${output_md}"

