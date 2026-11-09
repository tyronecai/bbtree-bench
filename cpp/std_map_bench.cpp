#include "ordered_map_bench.h"

#include <cstdint>
#include <map>

int main(int argc, char** argv) {
    using Map = std::map<std::uint64_t, std::uint64_t>;
    return bbtree_kv_bench::run_map_benchmark<Map>(
        argc, argv, "std_map", "std::map");
}
