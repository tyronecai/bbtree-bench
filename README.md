# bbtree、Rust `BTreeMap`、Abseil `btree_map` 与 C++ `std::map` 性能对比

项目在 Rust 中依赖 [`bbtree`](https://crates.io/crates/bbtree) 0.2.0，使用
`bbtree::unsync::BBTreeMap`；另外加入 Rust
`std::collections::BTreeMap`、Abseil `absl::btree_map` 和 C++
`std::map`。四者执行相同的单线程工作负载：

- 写入 1,000,000 个唯一的 `u64 -> u64` key-value；
- 按同一确定性随机顺序读取全部命中项；
- 读取 1,000,000 个不存在的 key；
- 更新全部 value；
- 删除随机顺序下的一半 key。

插入顺序、每阶段的操作数量和 value 计算方式都相同。Rust 的两个 map 使用
同一个泛型基准实现；C++ 的 `std::map` 和 Abseil 也使用同一个模板基准。
每个阶段在计时结束后校验长度、命中/未命中数、value checksum 和删除后的
完整 key/value 集合。每个进程默认重复 3 次，报告中取各阶段耗时的中位数。

测试结论、完整编译和测试环境以及官网 benchmark 差异分析见
[docs/benchmark-conclusion.md](docs/benchmark-conclusion.md)。

## 依赖

依赖已通过下面的等价命令加入：

```bash
cargo add bbtree
```

## 编译和运行

`target/` 中不放源码构建产物，避免把本机平台的对象文件带到远端：

```bash
./scripts/build_remote.sh
./scripts/run_benchmark.sh --count 1000000 --repeats 3
```

脚本在 `192.168.1.8` 上运行 `cargo test --release` 并构建两个 Rust
程序，然后把它们复制回当前机器。两个 C++ 程序直接在当前机器上通过
CMake/Ninja 和 GCC 13 的 `g++ -O3 -flto` 编译，Abseil 使用目录中的
`abseil-cpp-20260817.0`。结果写入：

- `results/latest.tsv`：完整原始数据；
- `results/latest.md`：便于阅读的对比表。
- `results/memory-latest.tsv`：独立插入测试测得的 map RSS 增量。
- `results/rust-fat-lto.tsv`：Rust fat LTO 补充测试数据。

可以通过环境变量覆盖远端：

```bash
REMOTE_HOST=192.168.1.8 REMOTE_DIR=/tmp/bbtree-kv-bench-build \
  ./scripts/build_remote.sh
```

## 数据规模

默认规模即一百万条。快速验证可以使用：

```bash
./scripts/run_benchmark.sh --count 100000 --repeats 1
```

正式结果使用默认的 1,000,000 条和至少 3 次重复。所有性能测试都在当前
机器运行；比较时应保持相同 CPU 频率和系统负载。内存报告给出两个指标：

- map RSS 增量：单独进程中先预生成 key，再记录 `VmRSS`，填充 map 后再次
  记录 RSS，报告两者的差值；
- 整个进程峰值 RSS：性能测试进程的 `VmHWM`，包含预生成 key 数组、map、
  运行时和分配器。
