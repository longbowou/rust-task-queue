use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rust_task_queue::prelude::*;

fn bench_queue_manager_operations(c: &mut Criterion) {
    let queue_manager = QueueManager::new();

    c.bench_function("queue_manager_get_queues", |b| {
        b.iter(|| {
            let queues = queue_manager.get_queues_by_priority();
            black_box(queues);
        })
    });

    c.bench_function("queue_manager_get_queue_config", |b| {
        b.iter(|| {
            let config = queue_manager.get_queue_config("default");
            black_box(config);
        })
    });
}

fn bench_autoscaler_config(c: &mut Criterion) {
    c.bench_function("autoscaler_config_creation", |b| {
        b.iter(|| {
            let config = AutoScalerConfig::default();
            black_box(config);
        })
    });
}

criterion_group!(benches, bench_queue_manager_operations, bench_autoscaler_config);
criterion_main!(benches); 