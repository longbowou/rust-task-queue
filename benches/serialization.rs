use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rust_task_queue::prelude::*;
use rust_task_queue::TaskResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SmallTask {
    id: String,
    data: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct MediumTask {
    id: String,
    data: String,
    metadata: HashMap<String, String>,
    payload: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct LargeTask {
    id: String,
    data: String,
    metadata: HashMap<String, String>,
    payload: Vec<u8>,
    complex_data: Vec<HashMap<String, Vec<String>>>,
}

#[async_trait::async_trait]
impl Task for SmallTask {
    async fn execute(&self) -> TaskResult {
        Ok(self.data.as_bytes().to_vec())
    }
    fn name(&self) -> &str {
        "small_task"
    }
}

#[async_trait::async_trait]
impl Task for MediumTask {
    async fn execute(&self) -> TaskResult {
        Ok(self.data.as_bytes().to_vec())
    }
    fn name(&self) -> &str {
        "medium_task"
    }
}

#[async_trait::async_trait]
impl Task for LargeTask {
    async fn execute(&self) -> TaskResult {
        Ok(self.data.as_bytes().to_vec())
    }
    fn name(&self) -> &str {
        "large_task"
    }
}

fn create_small_task() -> SmallTask {
    SmallTask {
        id: "bench_task_001".to_string(),
        data: "Simple benchmark data".to_string(),
    }
}

fn create_medium_task() -> MediumTask {
    let mut metadata = HashMap::new();
    metadata.insert("priority".to_string(), "high".to_string());
    metadata.insert("category".to_string(), "processing".to_string());
    metadata.insert("source".to_string(), "benchmark".to_string());

    MediumTask {
        id: "bench_task_002".to_string(),
        data: "Medium complexity benchmark data with additional fields".to_string(),
        metadata,
        payload: vec![0u8; 1024], // 1KB payload
    }
}

fn create_large_task() -> LargeTask {
    let mut metadata = HashMap::new();
    for i in 0..50 {
        metadata.insert(format!("key_{}", i), format!("value_{}", i));
    }

    let mut complex_data = Vec::new();
    for i in 0..10 {
        let mut inner_map = HashMap::new();
        inner_map.insert(
            format!("dataset_{}", i),
            (0..20).map(|j| format!("item_{}_{}", i, j)).collect(),
        );
        complex_data.push(inner_map);
    }

    LargeTask {
        id: "bench_task_003".to_string(),
        data: "Large complex benchmark data with nested structures and significant payload"
            .to_string(),
        metadata,
        payload: vec![0u8; 10240], // 10KB payload
        complex_data,
    }
}

fn bench_messagepack_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("messagepack_serialization");

    let small_task = create_small_task();
    let medium_task = create_medium_task();
    let large_task = create_large_task();

    group.bench_function("small_task", |b| {
        b.iter(|| {
            let serialized = rmp_serde::to_vec(&small_task).unwrap();
            black_box(serialized);
        })
    });

    group.bench_function("medium_task", |b| {
        b.iter(|| {
            let serialized = rmp_serde::to_vec(&medium_task).unwrap();
            black_box(serialized);
        })
    });

    group.bench_function("large_task", |b| {
        b.iter(|| {
            let serialized = rmp_serde::to_vec(&large_task).unwrap();
            black_box(serialized);
        })
    });

    group.finish();
}

fn bench_json_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_serialization");

    let small_task = create_small_task();
    let medium_task = create_medium_task();
    let large_task = create_large_task();

    group.bench_function("small_task", |b| {
        b.iter(|| {
            let serialized = serde_json::to_vec(&small_task).unwrap();
            black_box(serialized);
        })
    });

    group.bench_function("medium_task", |b| {
        b.iter(|| {
            let serialized = serde_json::to_vec(&medium_task).unwrap();
            black_box(serialized);
        })
    });

    group.bench_function("large_task", |b| {
        b.iter(|| {
            let serialized = serde_json::to_vec(&large_task).unwrap();
            black_box(serialized);
        })
    });

    group.finish();
}

fn bench_messagepack_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("messagepack_deserialization");

    let small_task = create_small_task();
    let medium_task = create_medium_task();
    let large_task = create_large_task();

    let small_serialized = rmp_serde::to_vec(&small_task).unwrap();
    let medium_serialized = rmp_serde::to_vec(&medium_task).unwrap();
    let large_serialized = rmp_serde::to_vec(&large_task).unwrap();

    group.bench_function("small_task", |b| {
        b.iter(|| {
            let deserialized: SmallTask = rmp_serde::from_slice(&small_serialized).unwrap();
            black_box(deserialized);
        })
    });

    group.bench_function("medium_task", |b| {
        b.iter(|| {
            let deserialized: MediumTask = rmp_serde::from_slice(&medium_serialized).unwrap();
            black_box(deserialized);
        })
    });

    group.bench_function("large_task", |b| {
        b.iter(|| {
            let deserialized: LargeTask = rmp_serde::from_slice(&large_serialized).unwrap();
            black_box(deserialized);
        })
    });

    group.finish();
}

fn bench_json_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_deserialization");

    let small_task = create_small_task();
    let medium_task = create_medium_task();
    let large_task = create_large_task();

    let small_serialized = serde_json::to_vec(&small_task).unwrap();
    let medium_serialized = serde_json::to_vec(&medium_task).unwrap();
    let large_serialized = serde_json::to_vec(&large_task).unwrap();

    group.bench_function("small_task", |b| {
        b.iter(|| {
            let deserialized: SmallTask = serde_json::from_slice(&small_serialized).unwrap();
            black_box(deserialized);
        })
    });

    group.bench_function("medium_task", |b| {
        b.iter(|| {
            let deserialized: MediumTask = serde_json::from_slice(&medium_serialized).unwrap();
            black_box(deserialized);
        })
    });

    group.bench_function("large_task", |b| {
        b.iter(|| {
            let deserialized: LargeTask = serde_json::from_slice(&large_serialized).unwrap();
            black_box(deserialized);
        })
    });

    group.finish();
}

fn bench_serialization_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization_throughput");

    // Test throughput with different batch sizes
    for batch_size in [10, 100, 1000].iter() {
        let tasks: Vec<MediumTask> = (0..*batch_size).map(|_| create_medium_task()).collect();

        group.throughput(Throughput::Elements(*batch_size as u64));

        group.bench_with_input(
            BenchmarkId::new("messagepack_batch", batch_size),
            batch_size,
            |b, _| {
                b.iter(|| {
                    for task in &tasks {
                        let serialized = rmp_serde::to_vec(task).unwrap();
                        black_box(serialized);
                    }
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("json_batch", batch_size),
            batch_size,
            |b, _| {
                b.iter(|| {
                    for task in &tasks {
                        let serialized = serde_json::to_vec(task).unwrap();
                        black_box(serialized);
                    }
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_messagepack_serialization,
    bench_json_serialization,
    bench_messagepack_deserialization,
    bench_json_deserialization,
    bench_serialization_throughput
);
criterion_main!(benches);
