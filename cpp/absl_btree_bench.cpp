#include "absl/container/btree_map.h"
#include "ordered_map_bench.h"

#include <cstdint>

int main(int argc, char** argv) {
    using Map = absl::btree_map<std::uint64_t, std::uint64_t>;
    return bbtree_kv_bench::run_map_benchmark<Map>(
        argc, argv, "absl_btree", "absl::btree_map");
}
