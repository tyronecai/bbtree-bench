#!/usr/bin/env python3
import csv
import socket
import sys
from pathlib import Path


PHASES = [
    "insert_all",
    "read_hit_all",
    "read_miss_all",
    "update_all",
    "remove_half",
]

IMPLEMENTATIONS = ["bbtree", "std_btree", "absl_btree", "std_map"]
LABELS = {
    "bbtree": "bbtree",
    "std_btree": "Rust BTreeMap",
    "absl_btree": "Abseil btree_map",
    "std_map": "C++ std::map",
}
COMPETITORS = ["std_btree", "absl_btree", "std_map"]


def main() -> None:
    path = Path(sys.argv[1])
    with path.open(newline="") as handle:
        rows = list(csv.DictReader(handle, delimiter="\t"))
    memory_path = Path(sys.argv[2])
    with memory_path.open(newline="") as handle:
        memory_rows = list(csv.DictReader(handle, delimiter="\t"))

    indexed = {(row["implementation"], row["phase"]): row for row in rows}
    memory_by_implementation = {
        row["implementation"]: int(row["map_rss_kb"]) for row in memory_rows
    }
    count = rows[0]["count"] if rows else "?"
    repeats = rows[0]["repeats"] if rows else "?"
    peak_rss = {
        implementation: max(
            (
                int(row["peak_rss_kb"])
                for row in rows
                if row["implementation"] == implementation
            ),
            default=0,
        )
        for implementation in IMPLEMENTATIONS
    }
    totals = {implementation: 0 for implementation in IMPLEMENTATIONS}

    print("# bbtree vs Rust BTreeMap vs Abseil btree_map vs C++ std::map")
    print()
    print(
        f"Dataset: {int(count):,} `u64 -> u64` entries; "
        f"{repeats} repeats; single-thread median."
    )
    print()
    print(f"Host: `{socket.gethostname()}`")
    print()

    print("## Latency")
    print()
    print("| Phase | bbtree ns/op | Rust BTreeMap | Abseil btree_map | C++ std::map |")
    print("|---|---:|---:|---:|---:|")
    for phase in PHASES:
        values = []
        for implementation in IMPLEMENTATIONS:
            row = indexed[(implementation, phase)]
            values.append(float(row["ns_per_op"]))
            totals[implementation] += int(row["median_ns"])
        print(
            f"| `{phase}` | {values[0]:.2f} | {values[1]:.2f} | "
            f"{values[2]:.2f} | {values[3]:.2f} |"
        )
    print(
        f"| **sum of phase medians** | "
        f"**{totals['bbtree'] / 1_000_000:.2f} ms** | "
        f"**{totals['std_btree'] / 1_000_000:.2f} ms** | "
        f"**{totals['absl_btree'] / 1_000_000:.2f} ms** | "
        f"**{totals['std_map'] / 1_000_000:.2f} ms** |"
    )

    print()
    print("## Throughput")
    print()
    print(
        "| Phase | bbtree Mops/s | Rust BTreeMap | "
        "Abseil btree_map | C++ std::map |"
    )
    print("|---|---:|---:|---:|---:|")
    for phase in PHASES:
        values = [
            float(indexed[(implementation, phase)]["mops"])
            for implementation in IMPLEMENTATIONS
        ]
        print(
            f"| `{phase}` | {values[0]:.2f} | {values[1]:.2f} | "
            f"{values[2]:.2f} | {values[3]:.2f} |"
        )

    print()
    print("## Relative to bbtree")
    print()
    print("| Phase | vs Rust BTreeMap | vs Abseil btree_map | vs C++ std::map |")
    print("|---|---:|---:|---:|")
    for phase in PHASES:
        bbtree_ns = int(indexed[("bbtree", phase)]["median_ns"])
        speedups = [
            int(indexed[(implementation, phase)]["median_ns"]) / bbtree_ns
            for implementation in COMPETITORS
        ]
        print(
            f"| `{phase}` | {speedups[0]:.2f}x | {speedups[1]:.2f}x | "
            f"{speedups[2]:.2f}x |"
        )
    total_speedups = [
        totals[implementation] / totals["bbtree"]
        for implementation in COMPETITORS
    ]
    print(
        f"| **sum of phase medians** | **{total_speedups[0]:.2f}x** | "
        f"**{total_speedups[1]:.2f}x** | **{total_speedups[2]:.2f}x** |"
    )
    print()
    print(
        "A value above `1.00x` means bbtree is faster; below `1.00x` "
        "means the competitor is faster."
    )
    print()
    print("## Memory")
    print()
    print(
        "| Implementation | Map RSS delta | Bytes/entry | "
        "Whole-process peak RSS |"
    )
    print("|---|---:|---:|---:|")
    for implementation in IMPLEMENTATIONS:
        map_rss_kb = memory_by_implementation[implementation]
        bytes_per_entry = map_rss_kb * 1024 / int(count)
        print(
            f"| {LABELS[implementation]} | {map_rss_kb / 1024:.1f} MiB | "
            f"{bytes_per_entry:.1f} | {peak_rss[implementation] / 1024:.1f} MiB |"
        )
    print()
    print(
        "`Map RSS delta` comes from a separate insertion-only process: keys "
        "are generated first, resident memory is sampled, the map is filled, "
        "and resident memory is sampled again. It includes allocator metadata "
        "and resident fragmentation. `Whole-process peak RSS` comes from the "
        "full performance run."
    )
    print()
    print(
        "Every phase validates its result after timing: collection size, "
        "hit and miss counts, update checksum, removed values, and the final "
        "retained key/value set are checked."
    )


if __name__ == "__main__":
    main()
