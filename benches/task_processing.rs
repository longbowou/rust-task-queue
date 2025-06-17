use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rust_task_queue::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct BenchTask {
    data: String,
}

#[async_trait::async_trait]
impl Task for BenchTask {
    async fn execute(&self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        // Simulate minimal work
        let result = format!("Processed: {}", self.data);
        Ok(result.into_bytes())
    }

    fn name(&self) -> &str {
        "bench_task"
    }
}

fn bench_task_serialization(c: &mut Criterion) {
    let task = BenchTask {
        data: "benchmark data".to_string(),
    };

    c.bench_function("task_serialization", |b| {
        b.iter(|| {
            let serialized = rmp_serde::to_vec(&task).unwrap();
            black_box(serialized);
        })
    });
}

fn bench_task_deserialization(c: &mut Criterion) {
    let task = BenchTask {
        data: "benchmark data".to_string(),
    };
    let serialized = rmp_serde::to_vec(&task).unwrap();

    c.bench_function("task_deserialization", |b| {
        b.iter(|| {
            let deserialized: BenchTask = rmp_serde::from_slice(&serialized).unwrap();
            black_box(deserialized);
        })
    });
}

criterion_group!(
    benches,
    bench_task_serialization,
    bench_task_deserialization
);
criterion_main!(benches);
