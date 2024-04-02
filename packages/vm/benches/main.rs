use criterion::{black_box, criterion_group, criterion_main, Criterion};

use rand::Rng;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;

use cosmwasm_std::{coins, Empty};
use cosmwasm_vm::testing::{
    mock_backend, mock_env, mock_info, mock_instance_options, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_vm::{
    call_execute, call_instantiate, capabilities_from_csv, Cache, CacheOptions, Checksum, Instance,
    InstanceOptions, Size,
};

use uuid::Uuid;

// Instance
const DEFAULT_MEMORY_LIMIT: Size = Size::mebi(64);
const DEFAULT_GAS_LIMIT: u64 = 1_000_000_000_000; // ~1ms
const DEFAULT_INSTANCE_OPTIONS: InstanceOptions = InstanceOptions {
    gas_limit: DEFAULT_GAS_LIMIT,
    print_debug: false,
};
const HIGH_GAS_LIMIT: u64 = 40_000_000_000_000_000; // ~20s, allows many calls on one instance

// Cache
const MEMORY_CACHE_SIZE: Size = Size::mebi(200);

// Multi-threaded get_instance benchmark
const INSTANTIATION_THREADS: usize = 128;
const CONTRACTS: u64 = 10;

static CONTRACT: &[u8] = include_bytes!("../testdata/hackatom.wasm");
static BENCH_SHA1: &[u8] = include_bytes!("../testdata/bench_sha1.wasm");
static BENCH_UUID: &[u8] = include_bytes!("../testdata/bench_uuid.wasm");

fn bench_instance(c: &mut Criterion) {
    let mut group = c.benchmark_group("Instance");

    group.bench_function("compile and instantiate", |b| {
        b.iter(|| {
            let backend = mock_backend(&[]);
            let (instance_options, memory_limit) = mock_instance_options();
            let _instance =
                Instance::from_code(CONTRACT, backend, instance_options, memory_limit).unwrap();
        });
    });

    group.bench_function("execute init", |b| {
        let backend = mock_backend(&[]);
        let much_gas: InstanceOptions = InstanceOptions {
            gas_limit: HIGH_GAS_LIMIT,
            ..DEFAULT_INSTANCE_OPTIONS
        };
        let mut instance =
            Instance::from_code(CONTRACT, backend, much_gas, Some(DEFAULT_MEMORY_LIMIT)).unwrap();

        b.iter(|| {
            let info = mock_info("creator", &coins(1000, "earth"));
            let msg = br#"{"verifier": "verifies", "beneficiary": "benefits"}"#;
            let contract_result =
                call_instantiate::<_, _, _, Empty>(&mut instance, &mock_env(), &info, msg).unwrap();
            assert!(contract_result.into_result().is_ok());
        });
    });

    group.bench_function("execute execute (release)", |b| {
        let backend = mock_backend(&[]);
        let much_gas: InstanceOptions = InstanceOptions {
            gas_limit: HIGH_GAS_LIMIT,
            ..DEFAULT_INSTANCE_OPTIONS
        };
        let mut instance =
            Instance::from_code(CONTRACT, backend, much_gas, Some(DEFAULT_MEMORY_LIMIT)).unwrap();

        let info = mock_info("creator", &coins(1000, "earth"));
        let msg = br#"{"verifier": "verifies", "beneficiary": "benefits"}"#;
        let contract_result =
            call_instantiate::<_, _, _, Empty>(&mut instance, &mock_env(), &info, msg).unwrap();
        assert!(contract_result.into_result().is_ok());

        b.iter(|| {
            let info = mock_info("verifies", &coins(15, "earth"));
            let msg = br#"{"release":{}}"#;
            let contract_result =
                call_execute::<_, _, _, Empty>(&mut instance, &mock_env(), &info, msg).unwrap();
            assert!(contract_result.into_result().is_ok());
        });
    });

    group.bench_function("execute execute (argon2)", |b| {
        let backend = mock_backend(&[]);
        let much_gas: InstanceOptions = InstanceOptions {
            gas_limit: HIGH_GAS_LIMIT,
            ..DEFAULT_INSTANCE_OPTIONS
        };
        let mut instance =
            Instance::from_code(CONTRACT, backend, much_gas, Some(DEFAULT_MEMORY_LIMIT)).unwrap();

        let info = mock_info("creator", &coins(1000, "earth"));
        let msg = br#"{"verifier": "verifies", "beneficiary": "benefits"}"#;
        let contract_result =
            call_instantiate::<_, _, _, Empty>(&mut instance, &mock_env(), &info, msg).unwrap();
        assert!(contract_result.into_result().is_ok());

        let mut gas_used = 0;
        b.iter(|| {
            let gas_before = instance.get_gas_left();
            let info = mock_info("hasher", &[]);
            let msg = br#"{"argon2":{"mem_cost":256,"time_cost":3}}"#;
            let contract_result =
                call_execute::<_, _, _, Empty>(&mut instance, &mock_env(), &info, msg).unwrap();
            assert!(contract_result.into_result().is_ok());
            gas_used = gas_before - instance.get_gas_left();
        });
        println!("Gas used: {}", gas_used);
    });

    group.finish();
}

fn bench_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("Cache");

    let options = CacheOptions {
        base_dir: TempDir::new().unwrap().into_path(),
        available_capabilities: capabilities_from_csv("iterator,staking"),
        memory_cache_size: MEMORY_CACHE_SIZE,
        instance_memory_limit: DEFAULT_MEMORY_LIMIT,
    };

    group.bench_function("save wasm", |b| {
        let cache: Cache<MockApi, MockStorage, MockQuerier> =
            unsafe { Cache::new(options.clone()).unwrap() };

        b.iter(|| {
            let result = cache.save_wasm(CONTRACT);
            assert!(result.is_ok());
        });
    });

    group.bench_function("load wasm", |b| {
        let cache: Cache<MockApi, MockStorage, MockQuerier> =
            unsafe { Cache::new(options.clone()).unwrap() };
        let checksum = cache.save_wasm(CONTRACT).unwrap();

        b.iter(|| {
            let result = cache.load_wasm(&checksum);
            assert!(result.is_ok());
        });
    });

    group.bench_function("analyze", |b| {
        let cache: Cache<MockApi, MockStorage, MockQuerier> =
            unsafe { Cache::new(options.clone()).unwrap() };
        let checksum = cache.save_wasm(CONTRACT).unwrap();

        b.iter(|| {
            let result = cache.analyze(&checksum);
            assert!(result.is_ok());
        });
    });

    group.bench_function("instantiate from fs", |b| {
        let non_memcache = CacheOptions {
            base_dir: TempDir::new().unwrap().into_path(),
            available_capabilities: capabilities_from_csv("iterator,staking"),
            memory_cache_size: Size(0),
            instance_memory_limit: DEFAULT_MEMORY_LIMIT,
        };
        let cache: Cache<MockApi, MockStorage, MockQuerier> =
            unsafe { Cache::new(non_memcache).unwrap() };
        let checksum = cache.save_wasm(CONTRACT).unwrap();

        b.iter(|| {
            let _ = cache
                .get_instance(&checksum, mock_backend(&[]), DEFAULT_INSTANCE_OPTIONS)
                .unwrap();
            assert_eq!(cache.stats().hits_pinned_memory_cache, 0);
            assert_eq!(cache.stats().hits_memory_cache, 0);
            assert!(cache.stats().hits_fs_cache >= 1);
            assert_eq!(cache.stats().misses, 0);
        });
    });

    group.bench_function("instantiate from memory", |b| {
        let checksum = Checksum::generate(CONTRACT);
        let cache: Cache<MockApi, MockStorage, MockQuerier> =
            unsafe { Cache::new(options.clone()).unwrap() };
        // Load into memory
        cache
            .get_instance(&checksum, mock_backend(&[]), DEFAULT_INSTANCE_OPTIONS)
            .unwrap();

        b.iter(|| {
            let backend = mock_backend(&[]);
            let _ = cache
                .get_instance(&checksum, backend, DEFAULT_INSTANCE_OPTIONS)
                .unwrap();
            assert_eq!(cache.stats().hits_pinned_memory_cache, 0);
            assert!(cache.stats().hits_memory_cache >= 1);
            assert_eq!(cache.stats().hits_fs_cache, 1);
            assert_eq!(cache.stats().misses, 0);
        });
    });

    group.bench_function("instantiate from pinned memory", |b| {
        let checksum = Checksum::generate(CONTRACT);
        let cache: Cache<MockApi, MockStorage, MockQuerier> =
            unsafe { Cache::new(options.clone()).unwrap() };
        // Load into pinned memory
        cache.pin(&checksum).unwrap();

        b.iter(|| {
            let backend = mock_backend(&[]);
            let _ = cache
                .get_instance(&checksum, backend, DEFAULT_INSTANCE_OPTIONS)
                .unwrap();
            assert_eq!(cache.stats().hits_memory_cache, 0);
            assert!(cache.stats().hits_pinned_memory_cache >= 1);
            assert_eq!(cache.stats().hits_fs_cache, 1);
            assert_eq!(cache.stats().misses, 0);
        });
    });

    group.finish();
}

pub fn bench_instance_threads(c: &mut Criterion) {
    c.bench_function("multi-threaded get_instance", |b| {
        let options = CacheOptions {
            base_dir: TempDir::new().unwrap().into_path(),
            available_capabilities: capabilities_from_csv("iterator,staking"),
            memory_cache_size: MEMORY_CACHE_SIZE,
            instance_memory_limit: DEFAULT_MEMORY_LIMIT,
        };

        let cache: Cache<MockApi, MockStorage, MockQuerier> =
            unsafe { Cache::new(options).unwrap() };
        let cache = Arc::new(cache);

        // Find sub-sequence helper
        fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
            haystack
                .windows(needle.len())
                .position(|window| window == needle)
        }

        // Offset to the i32.const (0x41) 15731626 (0xf00baa) (unsigned leb128 encoded) instruction
        // data we want to replace
        let query_int_data = b"\x41\xaa\x97\xc0\x07";
        let offset = find_subsequence(CONTRACT, query_int_data).unwrap() + 1;

        let mut leb128_buf = [0; 4];
        let mut contract = CONTRACT.to_vec();

        let mut random_checksum = || {
            let mut writable = &mut leb128_buf[..];

            // Generates a random number in the range of a 4-byte unsigned leb128 encoded number
            let r = rand::thread_rng().gen_range(2097152..2097152 + CONTRACTS);

            leb128::write::unsigned(&mut writable, r).expect("Should write number");

            // Splice data in contract
            contract.splice(offset..offset + leb128_buf.len(), leb128_buf);

            cache.save_wasm(contract.as_slice()).unwrap()
            // let checksum = cache.save_wasm(contract.as_slice()).unwrap();
            // Preload into memory
            // cache
            //     .get_instance(&checksum, mock_backend(&[]), DEFAULT_INSTANCE_OPTIONS)
            //     .unwrap();
            // checksum
        };

        b.iter_custom(|iters| {
            let mut res = Duration::from_secs(0);
            for _ in 0..iters {
                let mut durations: Vec<_> = (0..INSTANTIATION_THREADS)
                    .map(|_id| {
                        let cache = Arc::clone(&cache);
                        let checksum = random_checksum();

                        thread::spawn(move || {
                            let checksum = checksum;
                            // Perform measurement internally
                            let t = SystemTime::now();
                            black_box(
                                cache
                                    .get_instance(
                                        &checksum,
                                        mock_backend(&[]),
                                        DEFAULT_INSTANCE_OPTIONS,
                                    )
                                    .unwrap(),
                            );
                            t.elapsed().unwrap()
                        })
                    })
                    .collect::<Vec<_>>()
                    .into_iter()
                    .map(|handle| handle.join().unwrap())
                    .collect(); // join threads, collect durations

                // Calculate median thread duration
                durations.sort_unstable();
                res += durations[durations.len() / 2];
            }
            res
        });
    });
}

fn insert_comma(i: u64) -> String {
    format!("{}", i)
        .chars()
        .rev()
        .collect::<Vec<char>>()
        .chunks(3)
        .map(|cs| cs.iter().collect::<String>())
        .collect::<Vec<String>>()
        .join(",")
        .chars()
        .rev()
        .collect::<String>()
}

fn bench_uuid(c: &mut Criterion) {
    let mut group = c.benchmark_group("uuid");

    let (_, memory_limit) = mock_instance_options();
    let info = mock_info("creator", &coins(1000, "earth"));
    let much_gas: InstanceOptions = InstanceOptions {
        gas_limit: HIGH_GAS_LIMIT,
        ..DEFAULT_INSTANCE_OPTIONS
    };

    let expected_uuid_original = Uuid::new_v5(&Uuid::NAMESPACE_OID, b"link14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9sgf2vn8 12345 0").as_bytes()[0..16].to_vec();
    let expected_uuid = Uuid::new_v5(&Uuid::NAMESPACE_OID, &[b"link14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9sgf2vn8".as_slice(), &12345u64.to_be_bytes(), &0u16.to_be_bytes()].concat()).as_bytes()[0..16].to_vec();

    let mut instance = Instance::from_code(
        BENCH_UUID,
        mock_backend(&[]),
        much_gas,
        memory_limit,
    ).unwrap();

    call_instantiate::<_, _, _, Empty>(&mut instance, &mock_env(), &info, b"{}").unwrap();

    let mut dummy: Vec<u8> = vec![];
    let mut uuid_original: Vec<u8> = vec![];
    let mut uuid_wasm: Vec<u8> = vec![];
    let mut uuid_wasm_concat: Vec<u8> = vec![];
    let mut uuid_api: Vec<u8> = vec![];
    let mut uuid_api_concat: Vec<u8> = vec![];
    let mut uuid_api_separate: Vec<u8> = vec![];
    let mut gas_used_dummy = 0;
    let mut gas_used_original = 0;
    let mut gas_used_wasm = 0;
    let mut gas_used_wasm_concat = 0;
    let mut gas_used_api = 0;
    let mut gas_used_api_separate = 0;
    let mut gas_used_api_concat = 0;

    group.bench_function(format!("without_uuid"), |b| {
        b.iter(|| {
            let gas_before = instance.get_gas_left();
            let result_ptr = instance.call_function1("make_deps_and_output", &[]).unwrap();
            gas_used_dummy = gas_before - instance.get_gas_left();

            dummy = instance.read_memory(result_ptr.unwrap_i32() as u32, 16).unwrap();
            instance.deallocate(result_ptr.unwrap_i32() as u32).unwrap();
        });
    });

    group.bench_function(format!("uuid_original"), |b| {
        b.iter(|| {
            let gas_before = instance.get_gas_left();
            let result_ptr = instance.call_function1("uuid_original", &[]).unwrap();
            gas_used_original = gas_before - instance.get_gas_left();

            if uuid_original.len() == 0 {
                uuid_original = instance.read_memory(result_ptr.unwrap_i32() as u32, 16).unwrap();
            }
            instance.deallocate(result_ptr.unwrap_i32() as u32).unwrap();
        });
    });

    let _ = instance.call_function0("do_init_seq", &[]);

    group.bench_function(format!("uuid_api"), |b| {
        b.iter(|| {
            let gas_before = instance.get_gas_left();
            let result_ptr = instance.call_function1("uuid_api", &[]).unwrap();
            gas_used_api = gas_before - instance.get_gas_left();

            if uuid_api.len() == 0 {
                uuid_api = instance.read_memory(result_ptr.unwrap_i32() as u32, 16).unwrap();
            }
            instance.deallocate(result_ptr.unwrap_i32() as u32).unwrap();
        });
    });

    let _ = instance.call_function0("do_init_seq", &[]);

    group.bench_function(format!("uuid_api_separate"), |b| {
        b.iter(|| {
            let gas_before = instance.get_gas_left();
            let result_ptr = instance.call_function1("uuid_api_separate", &[]).unwrap();
            gas_used_api_separate = gas_before - instance.get_gas_left();

            if uuid_api_separate.len() == 0 {
                uuid_api_separate = instance.read_memory(result_ptr.unwrap_i32() as u32, 16).unwrap();
            }
            instance.deallocate(result_ptr.unwrap_i32() as u32).unwrap();
        });
    });

    let _ = instance.call_function0("do_init_seq", &[]);

    group.bench_function(format!("uuid_api_concat"), |b| {
        b.iter(|| {
            let gas_before = instance.get_gas_left();
            let result_ptr = instance.call_function1("uuid_api_concat", &[]).unwrap();
            gas_used_api_concat = gas_before - instance.get_gas_left();

            if uuid_api_concat.len() == 0 {
                uuid_api_concat = instance.read_memory(result_ptr.unwrap_i32() as u32, 16).unwrap();
            }
            instance.deallocate(result_ptr.unwrap_i32() as u32).unwrap();
        });
    });

    let _ = instance.call_function0("do_init_seq", &[]);

    group.bench_function(format!("uuid_wasm"), |b| {
        b.iter(|| {
            let gas_before = instance.get_gas_left();
            let result_ptr = instance.call_function1("uuid_native", &[]).unwrap();
            gas_used_wasm = gas_before - instance.get_gas_left();

            if uuid_wasm.len() == 0 {
                uuid_wasm = instance.read_memory(result_ptr.unwrap_i32() as u32, 16).unwrap();
            }
            instance.deallocate(result_ptr.unwrap_i32() as u32).unwrap();
        });
    });

    let _ = instance.call_function0("do_init_seq", &[]);

    group.bench_function(format!("uuid_wasm_concat"), |b| {
        b.iter(|| {
            let gas_before = instance.get_gas_left();
            let result_ptr = instance.call_function1("uuid_native_concat", &[]).unwrap();
            gas_used_wasm_concat = gas_before - instance.get_gas_left();

            if uuid_wasm_concat.len() == 0 {
                uuid_wasm_concat = instance.read_memory(result_ptr.unwrap_i32() as u32, 16).unwrap();
            }
            instance.deallocate(result_ptr.unwrap_i32() as u32).unwrap();
        });
    });

    println!("Gas used for dummy: {}", insert_comma(gas_used_dummy));
    println!("Gas used for original: {}", insert_comma(gas_used_original));
    println!("Gas used for api: {}", insert_comma(gas_used_api));
    println!("Gas used for api separate: {}", insert_comma(gas_used_api_separate));
    println!("Gas used for api concat: {}", insert_comma(gas_used_api_concat));
    println!("Gas used for wasm: {}", insert_comma(gas_used_wasm));
    println!("Gas used for wasm concat: {}", insert_comma(gas_used_wasm_concat));

    assert_eq!(expected_uuid_original, uuid_original);
    assert_eq!(expected_uuid_original, uuid_api);
    assert_eq!(expected_uuid, uuid_wasm);
    assert_eq!(expected_uuid, uuid_api_separate);
    assert_eq!(expected_uuid, uuid_api_concat);
    assert_eq!(expected_uuid, uuid_wasm_concat);
    println!("asserts have done");
}

fn bench_sha1(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha-1");

    let (_, memory_limit) = mock_instance_options();
    let info = mock_info("creator", &coins(1000, "earth"));
    let much_gas: InstanceOptions = InstanceOptions {
        gas_limit: HIGH_GAS_LIMIT,
        ..DEFAULT_INSTANCE_OPTIONS
    };

    for i in 1..50 {
        let mut instance = Instance::from_code(
            BENCH_SHA1,
            mock_backend(&[]),
            much_gas,
            memory_limit,
        ).unwrap();

        let pre = "x".repeat(i);
        let msg = format!(r#"{{"pre":"{} "}}"#, pre);
        let bytes = b"12345678901234";
        let len = [format!("{} ", pre).as_bytes(), bytes].concat().len();

        call_instantiate::<_, _, _, Empty>(&mut instance, &mock_env(), &info, msg.as_bytes()).unwrap();

        let mut dummy: Vec<u8> = vec![];
        let mut wasm_sha1: Vec<u8> = vec![];
        let mut api_sha1: Vec<u8> = vec![];
        let mut gas_used_dummy = 0;
        let mut gas_used_wasm = 0;
        let mut gas_used_wasm_twice = 0;
        let mut gas_used_api = 0;
        let mut gas_used_api_twice = 0;

        group.bench_function(format!("{}_without_sha1", len), |b| {
            b.iter(|| {
                let ptr = instance.allocate(bytes.len()).unwrap();
                instance.write_memory(ptr, bytes).unwrap();

                let gas_before = instance.get_gas_left();
                let result_ptr = instance.call_function1("make_deps_read_input_write_output", &[ptr.into()]).unwrap();
                gas_used_dummy = gas_before - instance.get_gas_left();

                dummy = instance.read_memory(result_ptr.unwrap_i32() as u32, 16).unwrap();
                instance.deallocate(result_ptr.unwrap_i32() as u32).unwrap();
            });
        });

        group.bench_function(format!("{}_wasm_sha1", len), |b| {
            b.iter(|| {
                let ptr = instance.allocate(bytes.len()).unwrap();
                instance.write_memory(ptr, bytes).unwrap();

                let gas_before = instance.get_gas_left();
                let result_ptr = instance.call_function1("sha1_raw", &[ptr.into()]).unwrap();
                gas_used_wasm = gas_before - instance.get_gas_left();

                wasm_sha1 = instance.read_memory(result_ptr.unwrap_i32() as u32, 16).unwrap();
                instance.deallocate(result_ptr.unwrap_i32() as u32).unwrap();
            });
        });

        group.bench_function(format!("{}_wasm_sha1_twice", len), |b| {
            b.iter(|| {
                let ptr = instance.allocate(bytes.len()).unwrap();
                instance.write_memory(ptr, bytes).unwrap();

                let gas_before = instance.get_gas_left();
                let result_ptr = instance.call_function1("sha1_raw_twice", &[ptr.into()]).unwrap();
                gas_used_wasm_twice = gas_before - instance.get_gas_left();

                wasm_sha1 = instance.read_memory(result_ptr.unwrap_i32() as u32, 16).unwrap();
                instance.deallocate(result_ptr.unwrap_i32() as u32).unwrap();
            });
        });

        group.bench_function(format!("{}_api_sha1", len), |b| {
            b.iter(|| {
                let ptr = instance.allocate(bytes.len()).unwrap();
                instance.write_memory(ptr, bytes).unwrap();

                let gas_before = instance.get_gas_left();
                let result_ptr = instance.call_function1("sha1_api", &[ptr.into()]).unwrap();
                gas_used_api = gas_before - instance.get_gas_left();

                api_sha1 = instance.read_memory(result_ptr.unwrap_i32() as u32, 16).unwrap();
                instance.deallocate(result_ptr.unwrap_i32() as u32).unwrap();
            });
        });

        group.bench_function(format!("{}_api_sha1_twice", len), |b| {
            b.iter(|| {
                let ptr = instance.allocate(bytes.len()).unwrap();
                instance.write_memory(ptr, bytes).unwrap();

                let gas_before = instance.get_gas_left();
                let result_ptr = instance.call_function1("sha1_api_twice", &[ptr.into()]).unwrap();
                gas_used_api_twice = gas_before - instance.get_gas_left();

                api_sha1 = instance.read_memory(result_ptr.unwrap_i32() as u32, 16).unwrap();
                instance.deallocate(result_ptr.unwrap_i32() as u32).unwrap();
            });
        });

        println!("Gas used for {}bytes/dummy: {}", len, insert_comma(gas_used_dummy));
        println!("Gas used for {}bytes/wasm: {}", len, insert_comma(gas_used_wasm));
        println!("Gas used for {}bytes/wasm_twice: {}", len, insert_comma(gas_used_wasm_twice));
        println!("Gas used for {}bytes/api: {}", len, insert_comma(gas_used_api));
        println!("Gas used for {}bytes/api_twice: {}", len, insert_comma(gas_used_api_twice));

        assert_eq!(wasm_sha1, api_sha1);
    }
}

fn make_config() -> Criterion {
    Criterion::default()
        .without_plots()
        .measurement_time(Duration::new(10, 0))
        .sample_size(12)
        .configure_from_args()
}

criterion_group!(
    name = instance;
    config = make_config();
    targets = bench_instance
);
criterion_group!(
    name = cache;
    config = make_config();
    targets = bench_cache
);
criterion_group!(
    name = multi_threaded_instance;
    config = Criterion::default()
        .without_plots()
        .measurement_time(Duration::new(16, 0))
        .sample_size(10)
        .configure_from_args();
    targets = bench_instance_threads
);
criterion_group!(
    name = sha1;
    config = make_config();
    targets = bench_sha1
);
criterion_group!(
    name = uuid;
    config = make_config();
    targets = bench_uuid
);

criterion_main!(uuid);
