use bbtree::unsync::BBTreeMap;
use std::collections::BTreeMap;
use std::env;
use std::hint::black_box;
use std::process::ExitCode;
use std::time::Instant;

const DEFAULT_COUNT: u64 = 1_000_000;
const DEFAULT_REPEATS: usize = 3;

const PHASES: [&str; 5] = [
    "insert_all",
    "read_hit_all",
    "read_miss_all",
    "update_all",
    "remove_half",
];

#[derive(Debug, Clone, Copy)]
struct Config {
    count: u64,
    repeats: usize,
    memory_only: bool,
}

trait BenchMap {
    fn empty() -> Self;
    fn put(&mut self, key: u64, value: u64);
    fn size(&self) -> usize;
    fn lookup(&self, key: &u64) -> Option<&u64>;
    fn lookup_mut(&mut self, key: &u64) -> Option<&mut u64>;
    fn delete(&mut self, key: &u64) -> Option<u64>;
}

impl BenchMap for BBTreeMap<u64, u64> {
    fn empty() -> Self {
        BBTreeMap::new()
    }

    fn put(&mut self, key: u64, value: u64) {
        let _ = BBTreeMap::insert(self, key, value);
    }

    fn size(&self) -> usize {
        BBTreeMap::len(self)
    }

    fn lookup(&self, key: &u64) -> Option<&u64> {
        BBTreeMap::get(self, key)
    }

    fn lookup_mut(&mut self, key: &u64) -> Option<&mut u64> {
        BBTreeMap::get_mut(self, key)
    }

    fn delete(&mut self, key: &u64) -> Option<u64> {
        BBTreeMap::remove(self, key)
    }
}

impl BenchMap for BTreeMap<u64, u64> {
    fn empty() -> Self {
        BTreeMap::new()
    }

    fn put(&mut self, key: u64, value: u64) {
        let _ = BTreeMap::insert(self, key, value);
    }

    fn size(&self) -> usize {
        BTreeMap::len(self)
    }

    fn lookup(&self, key: &u64) -> Option<&u64> {
        BTreeMap::get(self, key)
    }

    fn lookup_mut(&mut self, key: &u64) -> Option<&mut u64> {
        BTreeMap::get_mut(self, key)
    }

    fn delete(&mut self, key: &u64) -> Option<u64> {
        BTreeMap::remove(self, key)
    }
}

pub fn bbtree_entrypoint() -> ExitCode {
    run_entrypoint::<BBTreeMap<u64, u64>>("bbtree", "unsync::BBTreeMap")
}

pub fn std_btree_entrypoint() -> ExitCode {
    run_entrypoint::<BTreeMap<u64, u64>>("std_btree", "std::collections::BTreeMap")
}

fn run_entrypoint<M: BenchMap>(implementation: &str, map_type: &str) -> ExitCode {
    let executable = env::args().next().unwrap_or_else(|| "benchmark".to_owned());
    match parse_args() {
        Ok(config) => match if config.memory_only {
            run_memory::<M>(implementation, map_type, config)
        } else {
            run::<M>(implementation, map_type, config)
        } {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("error: {message}");
                ExitCode::FAILURE
            }
        },
        Err(message) => {
            eprintln!("error: {message}");
            eprintln!("usage: {executable} [--count N] [--repeats N] [--memory-only]");
            ExitCode::FAILURE
        }
    }
}

fn run<M: BenchMap>(implementation: &str, map_type: &str, config: Config) -> Result<(), String> {
    let keys = shuffled_keys(config.count);
    let mut samples = [
        Vec::<u128>::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ];
    let mut checksums = [0_u64; PHASES.len()];

    for repeat in 0..config.repeats {
        let mut map = M::empty();

        let started = Instant::now();
        for &key in &keys {
            map.put(key, value_for(key, 0));
        }
        let elapsed = started.elapsed();
        assert_eq!(
            map.size() as u64,
            config.count,
            "insert_all: length mismatch on repeat {repeat}"
        );
        samples[0].push(elapsed.as_nanos());
        checksums[0] = map.size() as u64;

        let started = Instant::now();
        let mut hit_sum = 0_u64;
        for &key in &keys {
            let value = *map
                .lookup(&key)
                .unwrap_or_else(|| panic!("read_hit_all: missing key {key}"));
            hit_sum = hit_sum.wrapping_add(value);
        }
        let elapsed = started.elapsed();
        assert_eq!(
            hit_sum,
            expected_sum(config.count, 0),
            "read_hit_all: checksum mismatch on repeat {repeat}"
        );
        samples[1].push(elapsed.as_nanos());
        checksums[1] = hit_sum;

        let started = Instant::now();
        let mut miss_count = 0_u64;
        for offset in 0..config.count {
            let missing_key = config.count + offset;
            if map.lookup(&missing_key).is_none() {
                miss_count += 1;
            }
        }
        let elapsed = started.elapsed();
        assert_eq!(
            miss_count, config.count,
            "read_miss_all: unexpected hit on repeat {repeat}"
        );
        samples[2].push(elapsed.as_nanos());
        checksums[2] = miss_count;

        let started = Instant::now();
        let mut update_sum = 0_u64;
        for &key in &keys {
            let value = map
                .lookup_mut(&key)
                .unwrap_or_else(|| panic!("update_all: missing key {key}"));
            *value = value_for(key, 1);
            update_sum = update_sum.wrapping_add(*value);
        }
        let elapsed = started.elapsed();
        assert_eq!(
            update_sum,
            expected_sum(config.count, 1),
            "update_all: checksum mismatch on repeat {repeat}"
        );
        samples[3].push(elapsed.as_nanos());
        checksums[3] = update_sum;

        let remove_count = config.count / 2;
        let started = Instant::now();
        let mut remove_sum = 0_u64;
        for &key in &keys[..remove_count as usize] {
            let value = map
                .delete(&key)
                .unwrap_or_else(|| panic!("remove_half: missing key {key}"));
            remove_sum = remove_sum.wrapping_add(value);
        }
        let elapsed = started.elapsed();
        samples[4].push(elapsed.as_nanos());
        checksums[4] = remove_sum;

        assert_eq!(
            map.size() as u64,
            config.count - remove_count,
            "remove_half: length mismatch on repeat {repeat}"
        );
        for (index, &key) in keys.iter().enumerate() {
            if (index as u64) < remove_count {
                assert!(
                    map.lookup(&key).is_none(),
                    "remove_half: key {key} was not removed on repeat {repeat}"
                );
            } else {
                assert_eq!(
                    map.lookup(&key).copied(),
                    Some(value_for(key, 1)),
                    "remove_half: retained key {key} has the wrong value on repeat {repeat}"
                );
            }
        }

        black_box(map.size());
    }

    let peak_rss_kb = peak_rss_kb();
    for (index, phase) in PHASES.iter().enumerate() {
        let median_ns = median(&mut samples[index]);
        let operations = match *phase {
            "remove_half" => config.count / 2,
            _ => config.count,
        };
        print_row(
            implementation,
            map_type,
            config,
            phase,
            operations,
            median_ns,
            checksums[index],
            peak_rss_kb,
        );
    }

    Ok(())
}

fn run_memory<M: BenchMap>(
    implementation: &str,
    map_type: &str,
    config: Config,
) -> Result<(), String> {
    let keys = shuffled_keys(config.count);
    let rss_before_kb = current_rss_kb();
    let mut map = M::empty();
    for &key in &keys {
        map.put(key, value_for(key, 0));
    }
    assert_eq!(map.size() as u64, config.count);
    let rss_after_kb = current_rss_kb();
    black_box(&map);
    let map_rss_kb = rss_after_kb.saturating_sub(rss_before_kb);
    println!(
        "{implementation}\t{map_type}\t{}\t{map_rss_kb}\t{}",
        config.count,
        peak_rss_kb()
    );
    Ok(())
}

fn parse_args() -> Result<Config, String> {
    let mut config = Config {
        count: DEFAULT_COUNT,
        repeats: DEFAULT_REPEATS,
        memory_only: false,
    };
    let mut args = env::args().skip(1);

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--count" => {
                config.count = args
                    .next()
                    .ok_or_else(|| "--count needs a value".to_owned())?
                    .parse()
                    .map_err(|_| "--count must be an integer".to_owned())?;
            }
            "--repeats" => {
                config.repeats = args
                    .next()
                    .ok_or_else(|| "--repeats needs a value".to_owned())?
                    .parse()
                    .map_err(|_| "--repeats must be an integer".to_owned())?;
            }
            "--memory-only" => {
                config.memory_only = true;
            }
            "-h" | "--help" => {
                println!("usage: benchmark [--count N] [--repeats N] [--memory-only]");
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    if config.count < 2 {
        return Err("--count must be at least 2".to_owned());
    }
    if usize::try_from(config.count).is_err() {
        return Err("--count is too large for this platform".to_owned());
    }
    if config.repeats == 0 {
        return Err("--repeats must be at least 1".to_owned());
    }

    Ok(config)
}

fn shuffled_keys(count: u64) -> Vec<u64> {
    let mut keys: Vec<u64> = (0..count).collect();
    let mut state = 0xD1B5_4A32_D192_ED03_u64;

    for index in (1..keys.len()).rev() {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        let random = state.wrapping_mul(0x2545_F491_4F6C_DD1D);
        let swap_index = (random % (index as u64 + 1)) as usize;
        keys.swap(index, swap_index);
    }
    keys
}

fn value_for(key: u64, version: u64) -> u64 {
    key.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(version + 1)
}

fn expected_sum(count: u64, version: u64) -> u64 {
    (0..count).fold(0_u64, |sum, key| sum.wrapping_add(value_for(key, version)))
}

fn median(samples: &mut [u128]) -> u128 {
    samples.sort_unstable();
    let middle = samples.len() / 2;
    if samples.len().is_multiple_of(2) {
        (samples[middle - 1] + samples[middle]) / 2
    } else {
        samples[middle]
    }
}

#[allow(clippy::too_many_arguments)]
fn print_row(
    implementation: &str,
    map_type: &str,
    config: Config,
    phase: &str,
    operations: u64,
    median_ns: u128,
    checksum: u64,
    peak_rss_kb: u64,
) {
    let nanoseconds_per_operation = median_ns as f64 / operations as f64;
    let million_operations_per_second = operations as f64 * 1_000.0 / median_ns.max(1) as f64;
    println!(
        "{implementation}\t{map_type}\t{}\t{}\t{phase}\t{operations}\t{median_ns}\t{nanoseconds_per_operation:.3}\t{million_operations_per_second:.3}\t{checksum}\t{peak_rss_kb}",
        config.count, config.repeats
    );
}

fn peak_rss_kb() -> u64 {
    proc_status_kb("VmHWM:").unwrap_or_default()
}

fn current_rss_kb() -> u64 {
    proc_status_kb("VmRSS:").unwrap_or_default()
}

fn proc_status_kb(field: &str) -> Option<u64> {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status.lines().find_map(|line| {
                line.strip_prefix(field)
                    .and_then(|value| value.split_whitespace().next())
                    .and_then(|value| value.parse().ok())
            })
        })
}
