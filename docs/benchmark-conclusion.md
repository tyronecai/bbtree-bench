# bbtree 性能与内存对比结论

测试日期：2026-11-09（Asia/Shanghai）

## 结论

在当前测试范围内，即 1,000,000 条 `u64 -> u64`、随机顺序、单线程、逐条
API 调用，bbtree 没有达到官网 README 中“单线程比 Rust BTreeMap 快 5 倍”
的普遍效果，也没有表现出比 Rust BTreeMap 低约 30% 的内存占用。

- bbtree 五阶段总耗时为 1163.56 ms，Rust BTreeMap 为 1318.01 ms。
  bbtree 快约 12%，而不是 5 倍。
- Abseil `btree_map` 五阶段总耗时为 1118.33 ms，比 bbtree 低约 4%。
- bbtree 的命中读取和更新略快于 Abseil；Abseil 的写入和删除更快。
- C++ `std::map` 的未命中读取最快，但其余阶段明显慢于三个 B-tree。
- 独立 map 内存测试中，bbtree 的 RSS 增量约 51.4 MiB，Rust BTreeMap
  约 27.9 MiB，Abseil 约 21.8 MiB，C++ `std::map` 约 61.2 MiB。

官网数字并非完全没有依据。bbtree 官方最新基准中，部分特定 workload 确实
达到 5 倍左右，例如 1M 顺序写入约 `5.28x`，1k map 的未命中查询约
`6.52x`。但这些数字不能直接推广到全部 key 类型、数据规模和操作组合。

完整数据：

- [性能与内存报告](../results/latest.md)
- [四实现原始性能数据](../results/latest.tsv)
- [独立内存测试数据](../results/memory-latest.tsv)
- [Rust fat LTO 补充测试](../results/rust-fat-lto.tsv)

## 测试规模与操作

主测试使用相同数据生成器和阶段边界：

1. 按确定性伪随机顺序写入 1,000,000 个唯一 key；
2. 按相同顺序读取全部命中项；
3. 读取 1,000,000 个不存在的 key；
4. 更新全部 value；
5. 删除随机顺序中的前 500,000 个 key。

key 和 value 类型均为 `u64`。key 数组在计时开始前生成。每个阶段单独计时，
进程默认重复 3 次，表中取中位数。计时区外校验 map 长度、命中/未命中数量、
checksum、更新后的 value 和删除后的完整 key/value 集合。

内存测试使用独立进程：

1. 先生成并打乱 key 数组；
2. 读取 `/proc/self/status` 的 `VmRSS`；
3. 创建并填满 map；
4. 再次读取 `VmRSS`；
5. 报告两次 RSS 的差值。

因此 map RSS 增量排除了预生成 key 数组，但包含 allocator 元数据、页级
取整和驻留内存碎片。报告同时给出性能进程的 `VmHWM`，即整个进程峰值 RSS。

完整构建和测试入口：

```bash
./scripts/build_remote.sh
./scripts/run_benchmark.sh --count 1000000 --repeats 3
```

`build_remote.sh` 将 Rust 源码同步到 `192.168.1.8`，在远端执行
`cargo test --release` 和 `cargo build --release --bins`，再取回两个 Rust
可执行文件。两个 C++ 可执行文件则在当前机器通过 CMake/Ninja 构建。
`run_benchmark.sh` 依次运行四个性能测试进程，再额外启动四个只负责插入的
内存测试进程。

## 编译环境

### Rust：远端编译，当前机器运行

编译主机为 `192.168.1.8`：

| 项目 | 配置 |
|---|---|
| 操作系统 | CentOS Linux 7 (Core)，kernel `3.10.0-1160.108.1.el7.x86_64` |
| CPU | 2 × Intel Xeon E5-2620 v3，共 24 vCPU；测试程序单线程运行 |
| 内存 | 31 GiB |
| Rust | `rustc 1.98.1 (48a229cea 2026-09-01)` |
| Cargo | `cargo 1.98.1 (797e8a9bc 2026-08-05)` |
| LLVM | `22.1.8` |
| target | `x86_64-unknown-linux-gnu` |
| 构建系统 glibc | 2.17 |
| `bbtree` | 0.2.0 |

远端执行：

```bash
cargo test --release
cargo build --release --bins
```

主测试的 release 参数：

| 参数 | 值 |
|---|---|
| `opt-level` | 3（release 默认） |
| `lto` | `thin` |
| `codegen-units` | 1 |
| `debug-assertions` | false |
| `overflow-checks` | false |
| `panic` | `unwind` |
| `RUSTFLAGS` | 未设置 |
| target CPU | 通用 `x86_64`，未使用 `target-cpu=native` |
| allocator | 系统 allocator |

`bbtree 0.2.0` 是测试时远端 Cargo 镜像提供的版本。官网最新版本为
`0.2.1`；两版源码的热路径没有性能相关变化，0.2.1 主要增加 serde 支持及
集合迭代器 `Send/Sync` 实现，因此本次差距不能归因于版本升级。

### C++：当前机器本地编译和运行

| 项目 | 配置 |
|---|---|
| 操作系统 | CentOS Stream 8，kernel `4.18.0-517.el8.x86_64` |
| CPU | Intel Xeon Skylake，4 vCPU；报告为 4 socket / 1 core per socket |
| 内存 | 15 GiB |
| glibc | 2.28 |
| 编译器 | GCC 13.1.1（`gcc-toolset-13`） |
| C++ 标准 | C++17 |
| CMake | 3.24.0 |
| Ninja | 1.8.2 |
| Abseil | `abseil-cpp-20260817.0` |
| allocator | 系统 allocator |

C++ 编译参数：

```text
-O3 -DNDEBUG -flto
-Wall -Wextra -Wpedantic -Wconversion -Werror
-static-libstdc++ -static-libgcc
```

`std::map` 与 `absl::btree_map` 复用同一个模板基准
[`cpp/ordered_map_bench.h`](../cpp/ordered_map_bench.h)。Abseil 运行时依赖
由目录中的源码通过 CMake 构建。两个 C++ 可执行文件均未启用
`-march=native`。

### 当前测试机器

所有性能和内存数据都在当前机器运行：

| 项目 | 配置 |
|---|---|
| 主机名 | `centos6-cai` |
| 操作系统 | CentOS Stream 8 |
| CPU | Intel Xeon Skylake，4 vCPU |
| 可见指令集 | AVX2、AVX-512F/DQ/CD/BW/VL、AVX512-VNNI 等 |
| 内存 | 15 GiB |
| 运行模式 | 四个程序依次运行，单线程核心循环 |
| CPU 绑定 | 未设置 `taskset` 或 CPU affinity |
| 频率控制 | 未固定 governor 或 turbo 状态 |

Rust 程序在远端按通用 x86-64 编译，因此虽然测试 CPU 支持 AVX-512，主测试
二进制不会利用这些指令。C++ 二进制也未启用 `-march=native`。

## 性能结果

| 操作 | bbtree ns/op | Rust BTreeMap | Abseil btree_map | C++ std::map |
|---|---:|---:|---:|---:|
| 写入全部 | 335.16 | 317.58 | **288.42** | 1073.65 |
| 命中读取 | **291.36** | 359.58 | 297.82 | 1142.82 |
| 未命中读取 | 69.33 | 45.90 | 60.23 | **16.60** |
| 更新全部 | **281.55** | 362.55 | 305.13 | 1145.83 |
| 删除一半 | 372.32 | 464.81 | **333.45** | 1466.28 |
| 五阶段总计 | 1163.56 ms | 1318.01 ms | **1118.33 ms** | 4112.05 ms |

相对 bbtree：

- Rust BTreeMap 总耗时是 bbtree 的约 `1.13x`；
- Abseil 总耗时是 bbtree 的约 `0.96x`；
- C++ `std::map` 总耗时是 bbtree 的约 `3.53x`。

## 内存结果

| 实现 | map RSS 增量 | 每 entry | 整个进程峰值 RSS |
|---|---:|---:|---:|
| bbtree | 51.4 MiB | 53.9 B | 61.2 MiB |
| Rust BTreeMap | 27.9 MiB | 29.3 B | 37.2 MiB |
| Abseil btree_map | **21.8 MiB** | **22.8 B** | **31.7 MiB** |
| C++ std::map | 61.2 MiB | 64.2 B | 70.9 MiB |

在当前 `u64 -> u64` 场景中：

- bbtree 的 map RSS 增量约为 Rust BTreeMap 的 `1.84x`；
- bbtree 的 map RSS 增量约为 Abseil 的 `2.36x`；
- bbtree 只比 C++ `std::map` 低约 16%。

这与官网“30% lower memory usage”不能直接对应。官网内存测试使用 mimalloc
和计数 allocator 测量 live heap；本测试使用系统 allocator 和 RSS，二者
口径不同。但就进程实际驻留内存而言，bbtree 在该 workload 下没有优势。

## 官网基准差异

bbtree 官方仓库的基准配置明显更激进：

- 官方 benchmark profile 使用 `lto = "fat"`、`codegen-units = 1`；
- 官方 `.cargo/config.toml` 设置 `-C target-cpu=native`；
- 官方 timing binary 使用 mimalloc；
- 官方测试在专用 self-hosted 机器运行，源码注释指定 Ryzen 9 9900X；
- 官方 workload 同时覆盖 1k、1M、10M、不同 key/value、顺序/随机访问、
  range、iterator、bulk build 和并发场景。

当前项目另外执行了 `fat LTO + x86-64-v3` 的 Rust 补充测试，结果保存在
[results/rust-fat-lto.tsv](../results/rust-fat-lto.tsv)：

| 操作 | bbtree ns/op | Rust BTreeMap ns/op |
|---|---:|---:|
| 写入全部 | 327.90 | **313.40** |
| 命中读取 | **322.32** | 363.45 |
| 未命中读取 | 72.88 | **46.16** |
| 更新全部 | **292.15** | 363.48 |
| 删除一半 | **351.06** | 477.23 |
| 五阶段总计 | **1190.78 ms** | 1325.11 ms |

补充构建命令：

```bash
RUSTFLAGS="-C target-cpu=x86-64-v3" \
CARGO_PROFILE_RELEASE_LTO=fat \
cargo build --release --bin bbtree_bench --bin std_btree_bench
```

换成 fat LTO 和 `x86-64-v3` 后，bbtree 总耗时仍只比 Rust BTreeMap 低约
10%，没有出现数量级变化。这个补充测试仍未使用官方的 mimalloc 和完全相同的
测试机，因此不是官网基准的逐项复现，但足以说明当前差距不能仅由 thin LTO
或通用 target CPU 解释。

bbtree 官方 `v0.2.1` 自己发布的结果也表明优势随 workload 变化。其
`u64/u64` 数据中，1M 随机写入约 `1.31x`、命中读取约 `2.00x`、未命中读取
约 `2.59x`、随机删除约 `1.36x`、upsert 约 `1.48x`。5 倍左右的数字主要
来自 1M 顺序写入约 `5.28x`、1k map 未命中查询约 `6.52x` 等特定 cell。

因此更准确的表述是：

> bbtree 在部分特定 benchmark workload 上可以达到甚至超过官网宣传的
> 倍数，但它并不是对任意有序 map workload 都稳定快 5 倍，也不保证所有
> 数据类型下都节省内存。README 中的 headline 省略了这些适用条件。

## 验证

- Rust `cargo test --release` 通过，包含 50,000 条字符串 key/value 的
  写入、读取、更新、删除和有序性检查。
- Rust `cargo fmt --check` 和
  `cargo clippy --release --all-targets -- -D warnings` 通过。
- C++ 使用 `-Wall -Wextra -Wpedantic -Wconversion -Werror` 编译通过。
- 100,000 条烟测和 1,000,000 条正式测试均通过。
- 四个实现的五阶段 checksum 完全一致。
- 内存模式会在输出前校验 map 长度，并确保 map 在第二次 RSS 采样期间仍然存活。

## 适用边界

本次结论适用于：

- 单线程；
- `u64` key 和 `u64` value；
- 1,000,000 条数据；
- 随机顺序的逐条 insert/get/update/remove；
- 系统 allocator；
- 当前 Xeon Skylake 4 vCPU 测试机。

它不能直接外推到：

- String、长共同前缀、真实业务 key；
- 批量 `insert_many`/`get_many`；
- range scan、iterator、顺序 bulk build；
- 10M 以上、明显 DRAM-bound 的数据；
- 并发读写；
- mimalloc、大页或其他 allocator；
- `target-cpu=native` 和官方专用测试机。

对于当前 workload，Abseil 的综合性能和内存表现最好；bbtree 与 Rust
BTreeMap 的性能接近且各有优势，但 bbtree 的内存占用更高。是否采用
bbtree 应使用项目的真实 key/value、数据规模和访问模式重新测试。
