# bbtree vs Rust BTreeMap vs Abseil btree_map vs C++ std::map

Dataset: 1,000,000 `u64 -> u64` entries; 3 repeats; single-thread median.

Host: `centos6-cai`

## Latency

| Phase | bbtree ns/op | Rust BTreeMap | Abseil btree_map | C++ std::map |
|---|---:|---:|---:|---:|
| `insert_all` | 335.16 | 317.58 | 288.42 | 1073.65 |
| `read_hit_all` | 291.36 | 359.58 | 297.82 | 1142.82 |
| `read_miss_all` | 69.33 | 45.90 | 60.23 | 16.60 |
| `update_all` | 281.55 | 362.55 | 305.13 | 1145.83 |
| `remove_half` | 372.32 | 464.81 | 333.45 | 1466.28 |
| **sum of phase medians** | **1163.56 ms** | **1318.01 ms** | **1118.33 ms** | **4112.05 ms** |

## Throughput

| Phase | bbtree Mops/s | Rust BTreeMap | Abseil btree_map | C++ std::map |
|---|---:|---:|---:|---:|
| `insert_all` | 2.98 | 3.15 | 3.47 | 0.93 |
| `read_hit_all` | 3.43 | 2.78 | 3.36 | 0.88 |
| `read_miss_all` | 14.42 | 21.79 | 16.60 | 60.23 |
| `update_all` | 3.55 | 2.76 | 3.28 | 0.87 |
| `remove_half` | 2.69 | 2.15 | 3.00 | 0.68 |

## Relative to bbtree

| Phase | vs Rust BTreeMap | vs Abseil btree_map | vs C++ std::map |
|---|---:|---:|---:|
| `insert_all` | 0.95x | 0.86x | 3.20x |
| `read_hit_all` | 1.23x | 1.02x | 3.92x |
| `read_miss_all` | 0.66x | 0.87x | 0.24x |
| `update_all` | 1.29x | 1.08x | 4.07x |
| `remove_half` | 1.25x | 0.90x | 3.94x |
| **sum of phase medians** | **1.13x** | **0.96x** | **3.53x** |

A value above `1.00x` means bbtree is faster; below `1.00x` means the competitor is faster.

## Memory

| Implementation | Map RSS delta | Bytes/entry | Whole-process peak RSS |
|---|---:|---:|---:|
| bbtree | 51.4 MiB | 53.9 | 61.2 MiB |
| Rust BTreeMap | 27.9 MiB | 29.3 | 37.2 MiB |
| Abseil btree_map | 21.8 MiB | 22.8 | 31.7 MiB |
| C++ std::map | 61.2 MiB | 64.2 | 70.9 MiB |

`Map RSS delta` comes from a separate insertion-only process: keys are generated first, resident memory is sampled, the map is filled, and resident memory is sampled again. It includes allocator metadata and resident fragmentation. `Whole-process peak RSS` comes from the full performance run.

Every phase validates its result after timing: collection size, hit and miss counts, update checksum, removed values, and the final retained key/value set are checked.
