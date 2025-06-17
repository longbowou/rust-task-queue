use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rust_task_queue::prelude::*;
use rust_task_queue::{TaskPriority, TaskResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct FastTask {
    id: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct CpuIntensiveTask {
    id: u32,
    iterations: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct MemoryIntensiveTask {
    id: u32,
    size_mb: u32,
}

#[async_trait::async_trait]
impl Task for FastTask {
    async fn execute(&self) -> TaskResult {
        // Minimal processing - just return the ID
        Ok(rmp_serde::to_vec(&self.id)?)
    }

    fn name(&self) -> &str {
        "fast_task"
    }
    fn timeout_seconds(&self) -> u64 {
        1
    }
}

#[async_trait::async_trait]
impl Task for CpuIntensiveTask {
    async fn execute(&self) -> TaskResult {
        // CPU-intensive work: calculate prime numbers
        let mut primes = Vec::new();
        for n in 2..self.iterations {
            let mut is_prime = true;
            for i in 2..((n as f64).sqrt() as u32 + 1) {
                if n % i == 0 {
                    is_prime = false;
                    break;
                }
            }
            if is_prime {
                primes.push(n);
            }
        }
        Ok(rmp_serde::to_vec(&primes.len())?)
    }

    fn name(&self) -> &str {
        "cpu_intensive_task"
    }
    fn timeout_seconds(&self) -> u64 {
        60
    }
}

#[async_trait::async_trait]
impl Task for MemoryIntensiveTask {
    async fn execute(&self) -> TaskResult {
        // Memory-intensive work: allocate and process large vectors
        let size = (self.size_mb as usize * 1024 * 1024) / std::mem::size_of::<u8>();
        let mut data = vec![0u8; size];

        // Do some work with the data to prevent optimization
        for (i, byte) in data.iter_mut().enumerate() {
            *byte = (i % 256) as u8;
        }

        let checksum: u64 = data.iter().map(|&b| b as u64).sum();
        Ok(rmp_serde::to_vec(&checksum)?)
    }

    fn name(&self) -> &str {
        "memory_intensive_task"
    }
    fn timeout_seconds(&self) -> u64 {
        120
    }
}

fn bench_task_execution_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("task_execution_types");

    group.bench_function("fast_task_creation", |b| {
        b.iter(|| {
            let task = FastTask { id: 1 };
            black_box(task);
        })
    });

    group.bench_function("cpu_intensive_task_creation", |b| {
        b.iter(|| {
            let task = CpuIntensiveTask {
                id: 1,
                iterations: 100,
            };
            black_box(task);
        })
    });

    group.bench_function("memory_intensive_task_creation", |b| {
        b.iter(|| {
            let task = MemoryIntensiveTask { id: 1, size_mb: 1 };
            black_box(task);
        })
    });

    group.finish();
}

fn bench_task_serialization_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("task_serialization_performance");

    let fast_task = FastTask { id: 1 };
    let cpu_task = CpuIntensiveTask {
        id: 1,
        iterations: 1000,
    };
    let memory_task = MemoryIntensiveTask { id: 1, size_mb: 1 };

    group.bench_function("fast_task_serialization", |b| {
        b.iter(|| {
            let serialized = rmp_serde::to_vec(&fast_task).unwrap();
            black_box(serialized);
        })
    });

    group.bench_function("cpu_task_serialization", |b| {
        b.iter(|| {
            let serialized = rmp_serde::to_vec(&cpu_task).unwrap();
            black_box(serialized);
        })
    });

    group.bench_function("memory_task_serialization", |b| {
        b.iter(|| {
            let serialized = rmp_serde::to_vec(&memory_task).unwrap();
            black_box(serialized);
        })
    });

    group.finish();
}

fn bench_task_registry_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("task_registry_operations");

    group.bench_function("task_registry_creation", |b| {
        b.iter(|| {
            let registry = TaskRegistry::new();
            black_box(registry);
        })
    });

    group.bench_function("task_registration", |b| {
        b.iter(|| {
            let registry = TaskRegistry::new();
            let _ = registry.register_with_name::<FastTask>("fast_task");
            let _ = registry.register_with_name::<CpuIntensiveTask>("cpu_intensive_task");
            let _ = registry.register_with_name::<MemoryIntensiveTask>("memory_intensive_task");
            black_box(registry);
        })
    });

    group.bench_function("task_name_lookup", |b| {
        let registry = TaskRegistry::new();
        let _ = registry.register_with_name::<FastTask>("fast_task");
        let _ = registry.register_with_name::<CpuIntensiveTask>("cpu_intensive_task");
        let _ = registry.register_with_name::<MemoryIntensiveTask>("memory_intensive_task");

        b.iter(|| {
            let tasks = registry.registered_tasks();
            let contains_fast = tasks.contains(&"fast_task".to_string());
            let contains_cpu = tasks.contains(&"cpu_intensive_task".to_string());
            let contains_memory = tasks.contains(&"memory_intensive_task".to_string());
            black_box((contains_fast, contains_cpu, contains_memory));
        })
    });

    group.finish();
}

fn bench_concurrent_task_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_task_creation");

    // Test different batch sizes
    for batch_size in [10, 50, 100, 500].iter() {
        group.throughput(Throughput::Elements(*batch_size as u64));

        group.bench_with_input(
            BenchmarkId::new("fast_task_batch", batch_size),
            batch_size,
            |b, &batch_size| {
                b.iter(|| {
                    let tasks: Vec<FastTask> =
                        (0..batch_size).map(|i| FastTask { id: i as u32 }).collect();
                    black_box(tasks);
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("cpu_task_batch", batch_size),
            batch_size,
            |b, &batch_size| {
                b.iter(|| {
                    let tasks: Vec<CpuIntensiveTask> = (0..batch_size)
                        .map(|i| CpuIntensiveTask {
                            id: i as u32,
                            iterations: 100,
                        })
                        .collect();
                    black_box(tasks);
                })
            },
        );
    }

    group.finish();
}

fn bench_task_metadata_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("task_metadata_operations");

    group.bench_function("task_metadata_creation", |b| {
        b.iter(|| {
            let metadata = TaskMetadata::default();
            black_box(metadata);
        })
    });

    group.bench_function("task_wrapper_creation", |b| {
        let task = FastTask { id: 1 };
        let task_data = rmp_serde::to_vec(&task).unwrap();

        b.iter(|| {
            let wrapper = TaskWrapper {
                metadata: TaskMetadata::default(),
                payload: task_data.clone(),
            };
            black_box(wrapper);
        })
    });

    group.bench_function("task_wrapper_serialization", |b| {
        let task = FastTask { id: 1 };
        let task_data = rmp_serde::to_vec(&task).unwrap();
        let wrapper = TaskWrapper {
            metadata: TaskMetadata::default(),
            payload: task_data,
        };

        b.iter(|| {
            let serialized = rmp_serde::to_vec(&wrapper).unwrap();
            black_box(serialized);
        })
    });

    group.finish();
}

fn bench_task_priority_handling(c: &mut Criterion) {
    let mut group = c.benchmark_group("task_priority_handling");

    #[derive(Debug, Serialize, Deserialize, Clone, Default)]
    struct PriorityTask {
        id: u32,
        priority_level: u8,
    }

    #[async_trait::async_trait]
    impl Task for PriorityTask {
        async fn execute(&self) -> TaskResult {
            Ok(rmp_serde::to_vec(&self.id)?)
        }

        fn name(&self) -> &str {
            "priority_task"
        }

        fn priority(&self) -> TaskPriority {
            match self.priority_level {
                0 => TaskPriority::Low,
                1 => TaskPriority::Normal,
                2 => TaskPriority::High,
                _ => TaskPriority::Critical,
            }
        }
    }

    group.bench_function("priority_task_creation", |b| {
        b.iter(|| {
            let tasks: Vec<PriorityTask> = (0..100)
                .map(|i| PriorityTask {
                    id: i as u32,
                    priority_level: (i % 4) as u8,
                })
                .collect();
            black_box(tasks);
        })
    });

    group.bench_function("priority_sorting", |b| {
        let mut tasks: Vec<PriorityTask> = (0..100)
            .map(|i| PriorityTask {
                id: i as u32,
                priority_level: (i % 4) as u8,
            })
            .collect();

        b.iter(|| {
            tasks.sort_by_key(|task| task.priority());
            black_box(&tasks);
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_task_execution_types,
    bench_task_serialization_performance,
    bench_task_registry_operations,
    bench_concurrent_task_creation,
    bench_task_metadata_operations,
    bench_task_priority_handling
);
criterion_main!(benches);
