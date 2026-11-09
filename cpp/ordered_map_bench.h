#ifndef BBTREE_KV_BENCH_ORDERED_MAP_BENCH_H_
#define BBTREE_KV_BENCH_ORDERED_MAP_BENCH_H_

#include <algorithm>
#include <chrono>
#include <cstdint>
#include <cstdlib>
#include <fstream>
#include <iostream>
#include <stdexcept>
#include <string>
#include <string_view>
#include <vector>

namespace bbtree_kv_bench {
namespace internal {

constexpr std::uint64_t kDefaultCount = 1'000'000;
constexpr std::size_t kDefaultRepeats = 3;
constexpr std::size_t kPhaseCount = 5;

struct Config {
    std::uint64_t count = kDefaultCount;
    std::size_t repeats = kDefaultRepeats;
    bool memory_only = false;
};

inline std::uint64_t value_for(std::uint64_t key, std::uint64_t version) {
    return key * 0x9E3779B97F4A7C15ULL + version + 1;
}

inline std::uint64_t expected_sum(std::uint64_t count,
                                  std::uint64_t version) {
    std::uint64_t sum = 0;
    for (std::uint64_t key = 0; key < count; ++key) {
        sum += value_for(key, version);
    }
    return sum;
}

inline std::uint64_t parse_u64(const char* text, const char* option) {
    try {
        std::size_t consumed = 0;
        const std::string value(text);
        const auto parsed = std::stoull(value, &consumed);
        if (consumed != value.size()) {
            throw std::invalid_argument("trailing characters");
        }
        return parsed;
    } catch (const std::exception&) {
        throw std::runtime_error(std::string(option) + " must be an integer");
    }
}

inline void print_usage(const char* executable) {
    std::cerr << "usage: " << executable
              << " [--count N] [--repeats N] [--memory-only]\n";
}

inline Config parse_args(int argc, char** argv) {
    Config config;
    for (int index = 1; index < argc; ++index) {
        const std::string_view argument(argv[index]);
        if (argument == "--count") {
            if (++index >= argc) {
                throw std::runtime_error("--count needs a value");
            }
            config.count = parse_u64(argv[index], "--count");
        } else if (argument == "--repeats") {
            if (++index >= argc) {
                throw std::runtime_error("--repeats needs a value");
            }
            const auto repeats = parse_u64(argv[index], "--repeats");
            if (repeats > static_cast<std::uint64_t>(SIZE_MAX)) {
                throw std::runtime_error("--repeats is too large");
            }
            config.repeats = static_cast<std::size_t>(repeats);
        } else if (argument == "--memory-only") {
            config.memory_only = true;
        } else if (argument == "-h" || argument == "--help") {
            print_usage(argv[0]);
            std::exit(0);
        } else {
            throw std::runtime_error("unknown argument: " +
                                     std::string(argument));
        }
    }

    if (config.count < 2) {
        throw std::runtime_error("--count must be at least 2");
    }
    if (config.count > static_cast<std::uint64_t>(SIZE_MAX)) {
        throw std::runtime_error("--count is too large for this platform");
    }
    if (config.repeats == 0) {
        throw std::runtime_error("--repeats must be at least 1");
    }
    return config;
}

inline std::vector<std::uint64_t> shuffled_keys(std::uint64_t count) {
    std::vector<std::uint64_t> keys(static_cast<std::size_t>(count));
    for (std::uint64_t key = 0; key < count; ++key) {
        keys[static_cast<std::size_t>(key)] = key;
    }

    std::uint64_t state = 0xD1B54A32D192ED03ULL;
    for (std::size_t index = keys.size() - 1; index > 0; --index) {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        const auto random = state * 0x2545F4914F6CDD1DULL;
        const auto swap_index = static_cast<std::size_t>(
            random % (static_cast<std::uint64_t>(index) + 1));
        std::swap(keys[index], keys[swap_index]);
    }
    return keys;
}

inline std::uint64_t median(std::vector<std::uint64_t>& samples) {
    std::sort(samples.begin(), samples.end());
    const auto middle = samples.size() / 2;
    if (samples.size() % 2 == 0) {
        return (samples[middle - 1] + samples[middle]) / 2;
    }
    return samples[middle];
}

inline std::uint64_t status_rss_kb(std::string_view field) {
    std::ifstream status("/proc/self/status");
    std::string line;
    while (std::getline(status, line)) {
        if (line.rfind(field, 0) == 0) {
            std::size_t begin = field.size();
            while (begin < line.size() &&
                   (line[begin] == ' ' || line[begin] == '\t')) {
                ++begin;
            }
            try {
                return std::stoull(line.substr(begin));
            } catch (const std::exception&) {
                return 0;
            }
        }
    }
    return 0;
}

inline std::uint64_t peak_rss_kb() {
    return status_rss_kb("VmHWM:");
}

inline std::uint64_t current_rss_kb() {
    return status_rss_kb("VmRSS:");
}

inline void print_row(const Config& config,
                      std::string_view implementation,
                      std::string_view map_type,
                      std::string_view phase,
                      std::uint64_t operations,
                      std::uint64_t median_ns,
                      std::uint64_t checksum,
                      std::uint64_t peak_rss) {
    const double ns_per_operation =
        static_cast<double>(median_ns) / static_cast<double>(operations);
    const double million_operations_per_second =
        static_cast<double>(operations) * 1000.0 /
        static_cast<double>(std::max<std::uint64_t>(median_ns, 1));
    std::cout << implementation << '\t' << map_type << '\t' << config.count
              << '\t' << config.repeats << '\t' << phase << '\t'
              << operations << '\t' << median_ns << '\t' << ns_per_operation
              << '\t' << million_operations_per_second << '\t' << checksum
              << '\t' << peak_rss << '\n';
}

template <typename Map>
int run(const Config& config,
        std::string_view implementation,
        std::string_view map_type) {
    using Clock = std::chrono::steady_clock;

    const auto keys = shuffled_keys(config.count);
    std::vector<std::uint64_t> samples[kPhaseCount];
    std::uint64_t checksums[kPhaseCount] = {};

    for (std::size_t repeat = 0; repeat < config.repeats; ++repeat) {
        Map map;

        auto started = Clock::now();
        for (const auto key : keys) {
            map.insert({key, value_for(key, 0)});
        }
        auto elapsed = std::chrono::duration_cast<std::chrono::nanoseconds>(
                           Clock::now() - started)
                           .count();
        if (map.size() != config.count) {
            throw std::runtime_error("insert_all: length mismatch");
        }
        samples[0].push_back(static_cast<std::uint64_t>(elapsed));
        checksums[0] = static_cast<std::uint64_t>(map.size());

        started = Clock::now();
        std::uint64_t hit_sum = 0;
        for (const auto key : keys) {
            const auto iterator = map.find(key);
            if (iterator == map.end()) {
                throw std::runtime_error("read_hit_all: missing key");
            }
            hit_sum += iterator->second;
        }
        elapsed = std::chrono::duration_cast<std::chrono::nanoseconds>(
                      Clock::now() - started)
                      .count();
        if (hit_sum != expected_sum(config.count, 0)) {
            throw std::runtime_error("read_hit_all: checksum mismatch");
        }
        samples[1].push_back(static_cast<std::uint64_t>(elapsed));
        checksums[1] = hit_sum;

        started = Clock::now();
        std::uint64_t miss_count = 0;
        for (std::uint64_t offset = 0; offset < config.count; ++offset) {
            if (map.find(config.count + offset) == map.end()) {
                ++miss_count;
            }
        }
        elapsed = std::chrono::duration_cast<std::chrono::nanoseconds>(
                      Clock::now() - started)
                      .count();
        if (miss_count != config.count) {
            throw std::runtime_error("read_miss_all: unexpected hit");
        }
        samples[2].push_back(static_cast<std::uint64_t>(elapsed));
        checksums[2] = miss_count;

        started = Clock::now();
        std::uint64_t update_sum = 0;
        for (const auto key : keys) {
            const auto iterator = map.find(key);
            if (iterator == map.end()) {
                throw std::runtime_error("update_all: missing key");
            }
            iterator->second = value_for(key, 1);
            update_sum += iterator->second;
        }
        elapsed = std::chrono::duration_cast<std::chrono::nanoseconds>(
                      Clock::now() - started)
                      .count();
        if (update_sum != expected_sum(config.count, 1)) {
            throw std::runtime_error("update_all: checksum mismatch");
        }
        samples[3].push_back(static_cast<std::uint64_t>(elapsed));
        checksums[3] = update_sum;

        const auto remove_count = config.count / 2;
        started = Clock::now();
        std::uint64_t remove_sum = 0;
        for (std::uint64_t index = 0; index < remove_count; ++index) {
            const auto key = keys[static_cast<std::size_t>(index)];
            const auto iterator = map.find(key);
            if (iterator == map.end()) {
                throw std::runtime_error("remove_half: missing key");
            }
            remove_sum += iterator->second;
            map.erase(iterator);
        }
        elapsed = std::chrono::duration_cast<std::chrono::nanoseconds>(
                      Clock::now() - started)
                      .count();
        samples[4].push_back(static_cast<std::uint64_t>(elapsed));
        checksums[4] = remove_sum;

        if (map.size() != config.count - remove_count) {
            throw std::runtime_error("remove_half: length mismatch");
        }
        for (std::uint64_t index = 0; index < config.count; ++index) {
            const auto key = keys[static_cast<std::size_t>(index)];
            const auto iterator = map.find(key);
            if (index < remove_count) {
                if (iterator != map.end()) {
                    throw std::runtime_error(
                        "remove_half: removed key is still present");
                }
            } else if (iterator == map.end() ||
                       iterator->second != value_for(key, 1)) {
                throw std::runtime_error(
                    "remove_half: retained key or value is incorrect");
            }
        }
    }

    const auto peak_rss = peak_rss_kb();
    constexpr const char* phases[kPhaseCount] = {
        "insert_all", "read_hit_all", "read_miss_all", "update_all",
        "remove_half"};
    for (std::size_t index = 0; index < kPhaseCount; ++index) {
        const auto operations = index == 4 ? config.count / 2 : config.count;
        print_row(config, implementation, map_type, phases[index], operations,
                  median(samples[index]), checksums[index], peak_rss);
    }
    return 0;
}

template <typename Map>
int run_memory(const Config& config,
               std::string_view implementation,
               std::string_view map_type) {
    const auto keys = shuffled_keys(config.count);
    const auto rss_before_kb = current_rss_kb();
    Map map;
    for (const auto key : keys) {
        map.insert({key, value_for(key, 0)});
    }
    if (map.size() != config.count) {
        throw std::runtime_error("memory-only: length mismatch");
    }
    const auto rss_after_kb = current_rss_kb();
    asm volatile("" : : "r"(&map) : "memory");
    const auto map_rss_kb =
        rss_after_kb > rss_before_kb ? rss_after_kb - rss_before_kb : 0;
    std::cout << implementation << '\t' << map_type << '\t' << config.count
              << '\t' << map_rss_kb << '\t' << peak_rss_kb()
              << '\n';
    return 0;
}

}  // namespace internal

template <typename Map>
int run_map_benchmark(int argc,
                      char** argv,
                      std::string_view implementation,
                      std::string_view map_type) {
    try {
        const auto config = internal::parse_args(argc, argv);
        if (config.memory_only) {
            return internal::run_memory<Map>(config, implementation, map_type);
        }
        return internal::run<Map>(config, implementation, map_type);
    } catch (const std::exception& error) {
        std::cerr << "error: " << error.what() << '\n';
        internal::print_usage(argv[0]);
        return EXIT_FAILURE;
    }
}

}  // namespace bbtree_kv_bench

#endif  // BBTREE_KV_BENCH_ORDERED_MAP_BENCH_H_
