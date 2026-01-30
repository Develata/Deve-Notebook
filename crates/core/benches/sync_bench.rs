//! # Sync Module Benchmarks
//!
//! 量化 SmolStr 优化效果的基准测试。
//!
//! ## 测试场景
//!
//! 1. **Clone 性能**: String vs SmolStr 克隆开销
//! 2. **创建性能**: 从 &str 创建新实例
//! 3. **LedgerEntry 克隆**: 模拟真实同步场景
//! 4. **批量操作**: 模拟 P2P 同步时的批量处理

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use smol_str::SmolStr;

// ============================================================================
// Test Data Constants
// ============================================================================

/// 短字符串 (< 23 bytes) - SmolStr 内联存储阈值内
const SHORT_CONTENT: &str = "Hello, World!"; // 13 bytes

/// 中等字符串 - 典型的单词/短句插入
const MEDIUM_CONTENT: &str = "The quick brown fox jumps"; // 25 bytes

/// UUID 格式字符串 - PeerId 典型格式
const UUID_CONTENT: &str = "550e8400-e29b-41d4-a716-446655440000"; // 36 bytes

/// 长字符串 - 段落级别插入
const LONG_CONTENT: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
    Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua."; // 124 bytes

// ============================================================================
// Benchmark: Clone Performance
// ============================================================================

fn bench_clone(c: &mut Criterion) {
    let mut group = c.benchmark_group("clone");

    for (name, content) in [
        ("short_13B", SHORT_CONTENT),
        ("medium_25B", MEDIUM_CONTENT),
        ("uuid_36B", UUID_CONTENT),
        ("long_124B", LONG_CONTENT),
    ] {
        let string_val = content.to_string();
        let smol_val = SmolStr::new(content);

        group.throughput(Throughput::Elements(1));

        group.bench_with_input(BenchmarkId::new("String", name), &string_val, |b, s| {
            b.iter(|| black_box(s.clone()));
        });

        group.bench_with_input(BenchmarkId::new("SmolStr", name), &smol_val, |b, s| {
            b.iter(|| black_box(s.clone()));
        });
    }

    group.finish();
}

// ============================================================================
// Benchmark: Creation Performance
// ============================================================================

fn bench_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("creation");

    for (name, content) in [
        ("short_13B", SHORT_CONTENT),
        ("medium_25B", MEDIUM_CONTENT),
        ("uuid_36B", UUID_CONTENT),
        ("long_124B", LONG_CONTENT),
    ] {
        group.throughput(Throughput::Elements(1));

        group.bench_with_input(BenchmarkId::new("String", name), &content, |b, s| {
            b.iter(|| black_box(s.to_string()));
        });

        group.bench_with_input(BenchmarkId::new("SmolStr", name), &content, |b, s| {
            b.iter(|| black_box(SmolStr::new(*s)));
        });
    }

    group.finish();
}

// ============================================================================
// Benchmark: Batch Clone (Simulating P2P Sync)
// ============================================================================

fn bench_batch_clone(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_clone");

    // 模拟 P2P 同步: 1000 个操作的批量克隆
    const BATCH_SIZE: usize = 1000;

    for (name, content) in [("short_13B", SHORT_CONTENT), ("uuid_36B", UUID_CONTENT)] {
        let string_vec: Vec<String> = (0..BATCH_SIZE).map(|_| content.to_string()).collect();
        let smol_vec: Vec<SmolStr> = (0..BATCH_SIZE).map(|_| SmolStr::new(content)).collect();

        group.throughput(Throughput::Elements(BATCH_SIZE as u64));

        group.bench_with_input(BenchmarkId::new("String", name), &string_vec, |b, vec| {
            b.iter(|| {
                let cloned: Vec<String> = vec.iter().cloned().collect();
                black_box(cloned)
            });
        });

        group.bench_with_input(BenchmarkId::new("SmolStr", name), &smol_vec, |b, vec| {
            b.iter(|| {
                let cloned: Vec<SmolStr> = vec.iter().cloned().collect();
                black_box(cloned)
            });
        });
    }

    group.finish();
}

// ============================================================================
// Benchmark: HashMap Operations (PeerId lookup simulation)
// ============================================================================

fn bench_hashmap_ops(c: &mut Criterion) {
    use std::collections::HashMap;

    let mut group = c.benchmark_group("hashmap");

    const NUM_PEERS: usize = 100;

    // 生成测试数据
    let peer_ids: Vec<String> = (0..NUM_PEERS)
        .map(|i| format!("peer-{i:032}")) // 37 bytes each
        .collect();

    // String HashMap
    let string_map: HashMap<String, u64> = peer_ids
        .iter()
        .enumerate()
        .map(|(i, s)| (s.clone(), i as u64))
        .collect();

    // SmolStr HashMap
    let smol_map: HashMap<SmolStr, u64> = peer_ids
        .iter()
        .enumerate()
        .map(|(i, s)| (SmolStr::new(s), i as u64))
        .collect();

    group.throughput(Throughput::Elements(NUM_PEERS as u64));

    // Benchmark: Lookup all keys
    group.bench_function("String_lookup", |b| {
        b.iter(|| {
            let mut sum = 0u64;
            for key in &peer_ids {
                sum += string_map.get(key).unwrap_or(&0);
            }
            black_box(sum)
        });
    });

    group.bench_function("SmolStr_lookup", |b| {
        b.iter(|| {
            let mut sum = 0u64;
            for key in &peer_ids {
                sum += smol_map.get(key.as_str()).unwrap_or(&0);
            }
            black_box(sum)
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark: Memory Allocation Pattern
// ============================================================================

fn bench_allocation_pattern(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocation");

    // 模拟编辑器输入: 大量单字符/短词插入
    const NUM_OPS: usize = 10_000;

    let short_inputs: Vec<&str> = vec!["a"; NUM_OPS]; // 单字符输入
    let word_inputs: Vec<&str> = vec!["hello"; NUM_OPS]; // 短词输入

    group.throughput(Throughput::Elements(NUM_OPS as u64));

    // 单字符插入
    group.bench_function("String_single_char", |b| {
        b.iter(|| {
            let ops: Vec<String> = short_inputs.iter().map(|s| s.to_string()).collect();
            black_box(ops)
        });
    });

    group.bench_function("SmolStr_single_char", |b| {
        b.iter(|| {
            let ops: Vec<SmolStr> = short_inputs.iter().map(|s| SmolStr::new(*s)).collect();
            black_box(ops)
        });
    });

    // 短词输入
    group.bench_function("String_word", |b| {
        b.iter(|| {
            let ops: Vec<String> = word_inputs.iter().map(|s| s.to_string()).collect();
            black_box(ops)
        });
    });

    group.bench_function("SmolStr_word", |b| {
        b.iter(|| {
            let ops: Vec<SmolStr> = word_inputs.iter().map(|s| SmolStr::new(*s)).collect();
            black_box(ops)
        });
    });

    group.finish();
}

// ============================================================================
// Criterion Main
// ============================================================================

criterion_group!(
    benches,
    bench_clone,
    bench_creation,
    bench_batch_clone,
    bench_hashmap_ops,
    bench_allocation_pattern,
);

criterion_main!(benches);
