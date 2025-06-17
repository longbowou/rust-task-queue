use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use rust_task_queue::prelude::*;
use rust_task_queue::{TaskResult, TaskPriority};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

// Test tasks for end-to-end benchmarking
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct BenchmarkTask {
    id: u32,
    data: String,
    processing_time_ms: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct ThroughputTask {
    id: u32,
    batch_id: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct LatencyTask {
    id: u32,
    enqueue_time: u64,
}

#[async_trait::async_trait]
impl Task for BenchmarkTask {
    async fn execute(&self) -> TaskResult {
        // Simulate processing time without actual sleeping for benchmarks
        let cycles = self.processing_time_ms * 1000; // Simple simulation
        let mut sum = 0u64;
        for i in 0..cycles {
            sum = sum.wrapping_add(i);
        }
        Ok(rmp_serde::to_vec(&format!("completed_{}_{}", self.id, sum))?)
    }
    
    fn name(&self) -> &str { "benchmark_task" }
    fn timeout_seconds(&self) -> u64 { 30 }
}

#[async_trait::async_trait]
impl Task for ThroughputTask {
    async fn execute(&self) -> TaskResult {
        // Minimal processing for throughput testing
        Ok(rmp_serde::to_vec(&(self.id, self.batch_id))?)
    }
    
    fn name(&self) -> &str { "throughput_task" }
    fn timeout_seconds(&self) -> u64 { 5 }
}

#[async_trait::async_trait]
impl Task for LatencyTask {
    async fn execute(&self) -> TaskResult {
        let process_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        let latency = process_time - self.enqueue_time;
        Ok(rmp_serde::to_vec(&latency)?)
    }
    
    fn name(&self) -> &str { "latency_task" }
    fn timeout_seconds(&self) -> u64 { 5 }
}

fn bench_task_creation_workflow(c: &mut Criterion) {
    let mut group = c.benchmark_group("task_creation_workflow");
    
    group.bench_function("simple_task_creation", |b| {
        b.iter(|| {
            let task = BenchmarkTask {
                id: 1,
                data: "simple task data".to_string(),
                processing_time_ms: 0,
            };
            black_box(task);
        })
    });
    
    group.bench_function("complex_task_creation", |b| {
        b.iter(|| {
            let task = BenchmarkTask {
                id: 1,
                data: "x".repeat(1000), // 1KB data
                processing_time_ms: 10,
            };
            black_box(task);
        })
    });
    
    group.bench_function("task_serialization_workflow", |b| {
        let task = BenchmarkTask {
            id: 1,
            data: "workflow task data".to_string(),
            processing_time_ms: 5,
        };
        
        b.iter(|| {
            let serialized = rmp_serde::to_vec(&task).unwrap();
            let _deserialized: BenchmarkTask = rmp_serde::from_slice(&serialized).unwrap();
            black_box(serialized);
        })
    });
    
    group.finish();
}

fn bench_batch_task_workflow(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_task_workflow");
    
    // Test different batch sizes
    for batch_size in [10, 50, 100, 500].iter() {
        group.throughput(Throughput::Elements(*batch_size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("batch_creation", batch_size),
            batch_size,
            |b, &batch_size| {
                b.iter(|| {
                    let tasks: Vec<ThroughputTask> = (0..batch_size)
                        .map(|i| ThroughputTask {
                            id: i as u32,
                            batch_id: 1,
                        })
                        .collect();
                    black_box(tasks);
                })
            }
        );
        
        group.bench_with_input(
            BenchmarkId::new("batch_serialization", batch_size),
            batch_size,
            |b, &batch_size| {
                let tasks: Vec<ThroughputTask> = (0..batch_size)
                    .map(|i| ThroughputTask {
                        id: i as u32,
                        batch_id: 1,
                    })
                    .collect();
                
                b.iter(|| {
                    let serialized: Vec<Vec<u8>> = tasks.iter()
                        .map(|task| rmp_serde::to_vec(task).unwrap())
                        .collect();
                    black_box(serialized);
                })
            }
        );
    }
    
    group.finish();
}

fn bench_task_registry_workflow(c: &mut Criterion) {
    let mut group = c.benchmark_group("task_registry_workflow");
    
    group.bench_function("registry_setup", |b| {
        b.iter(|| {
            let registry = TaskRegistry::new();
            let _ = registry.register_with_name::<BenchmarkTask>("benchmark_task");
            let _ = registry.register_with_name::<ThroughputTask>("throughput_task");
            let _ = registry.register_with_name::<LatencyTask>("latency_task");
            black_box(registry);
        })
    });
    
    group.bench_function("task_lookup_workflow", |b| {
        let registry = TaskRegistry::new();
        let _ = registry.register_with_name::<BenchmarkTask>("benchmark_task");
        let _ = registry.register_with_name::<ThroughputTask>("throughput_task");
        let _ = registry.register_with_name::<LatencyTask>("latency_task");
        
        b.iter(|| {
            let registered_tasks = registry.registered_tasks();
            let benchmark_exists = registered_tasks.contains(&"benchmark_task".to_string());
            let throughput_exists = registered_tasks.contains(&"throughput_task".to_string());
            let latency_exists = registered_tasks.contains(&"latency_task".to_string());
            black_box((benchmark_exists, throughput_exists, latency_exists));
        })
    });
    
    group.finish();
}

fn bench_latency_measurement_workflow(c: &mut Criterion) {
    let mut group = c.benchmark_group("latency_measurement_workflow");
    
    group.bench_function("task_timestamp_creation", |b| {
        b.iter(|| {
            let enqueue_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            
            let task = LatencyTask {
                id: 1,
                enqueue_time,
            };
            black_box(task);
        })
    });
    
    group.bench_function("latency_calculation", |b| {
        let start_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        b.iter(|| {
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            
            let latency = current_time - start_time;
            black_box(latency);
        })
    });
    
    group.finish();
}

fn bench_priority_workflow(c: &mut Criterion) {
    let mut group = c.benchmark_group("priority_workflow");
    
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
        
        fn name(&self) -> &str { "priority_task" }
        
        fn priority(&self) -> TaskPriority {
            match self.priority_level {
                0 => TaskPriority::Low,
                1 => TaskPriority::Normal,
                2 => TaskPriority::High,
                _ => TaskPriority::Critical,
            }
        }
    }
    
    group.bench_function("mixed_priority_creation", |b| {
        b.iter(|| {
            let tasks: Vec<PriorityTask> = (0..30)
                .map(|i| {
                    let priority = match i % 3 {
                        0 => 3, // Critical
                        1 => 1, // Normal
                        _ => 0, // Low
                    };
                    
                    PriorityTask {
                        id: i as u32,
                        priority_level: priority,
                    }
                })
                .collect();
            black_box(tasks);
        })
    });
    
    group.bench_function("priority_based_sorting", |b| {
        let mut tasks: Vec<PriorityTask> = (0..30)
            .map(|i| {
                let priority = match i % 3 {
                    0 => 3, // Critical
                    1 => 1, // Normal
                    _ => 0, // Low
                };
                
                PriorityTask {
                    id: i as u32,
                    priority_level: priority,
                }
            })
            .collect();
        
        b.iter(|| {
            tasks.sort_by_key(|task| std::cmp::Reverse(task.priority())); // Higher priority first
            black_box(&tasks);
        })
    });
    
    group.finish();
}

fn bench_memory_workflow(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_workflow");
    
    group.bench_function("large_task_workflow", |b| {
        b.iter(|| {
            // Create tasks with varying data sizes
            let tasks: Vec<BenchmarkTask> = (0..100)
                .map(|i| BenchmarkTask {
                    id: i,
                    data: "x".repeat(i as usize * 10), // Variable size data
                    processing_time_ms: 0,
                })
                .collect();
            
            // Serialize all tasks
            let serialized: Vec<Vec<u8>> = tasks.iter()
                .map(|task| rmp_serde::to_vec(task).unwrap())
                .collect();
            
            black_box((tasks, serialized));
        })
    });
    
    group.bench_function("memory_intensive_operations", |b| {
        b.iter(|| {
            // Create task metadata
            let metadata_list: Vec<TaskMetadata> = (0..50)
                .map(|_| TaskMetadata::default())
                .collect();
            
            // Create task wrappers
            let task = BenchmarkTask {
                id: 1,
                data: "memory test".to_string(),
                processing_time_ms: 0,
            };
            let task_data = rmp_serde::to_vec(&task).unwrap();
            
            let wrappers: Vec<TaskWrapper> = metadata_list.into_iter()
                .map(|metadata| TaskWrapper {
                    metadata,
                    payload: task_data.clone(),
                })
                .collect();
            
            black_box(wrappers);
        })
    });
    
    group.finish();
}

fn bench_error_handling_workflow(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_handling_workflow");
    
    #[derive(Debug, Serialize, Deserialize, Clone, Default)]
    struct FailingTask {
        id: u32,
        should_fail: bool,
    }
    
    #[async_trait::async_trait]
    impl Task for FailingTask {
        async fn execute(&self) -> TaskResult {
            if self.should_fail {
                Err("Simulated task failure".into())
            } else {
                Ok(rmp_serde::to_vec(&self.id)?)
            }
        }
        
        fn name(&self) -> &str { "failing_task" }
        fn max_retries(&self) -> u32 { 3 }
    }
    
    group.bench_function("success_task_workflow", |b| {
        b.iter(|| {
            let tasks: Vec<FailingTask> = (0..10)
                .map(|i| FailingTask {
                    id: i,
                    should_fail: false,
                })
                .collect();
            
            // Simulate processing (serialization represents the workflow)
            let serialized: Vec<Vec<u8>> = tasks.iter()
                .map(|task| rmp_serde::to_vec(task).unwrap())
                .collect();
            
            black_box(serialized);
        })
    });
    
    group.bench_function("error_handling_overhead", |b| {
        b.iter(|| {
            let tasks: Vec<FailingTask> = (0..10)
                .map(|i| FailingTask {
                    id: i,
                    should_fail: i % 3 == 0, // Some tasks will fail
                })
                .collect();
            
            // Simulate the retry logic overhead by creating multiple versions
            let task_attempts: Vec<Vec<FailingTask>> = tasks.iter()
                .map(|task| {
                    (0..task.max_retries())
                        .map(|_| task.clone())
                        .collect()
                })
                .collect();
            
            black_box(task_attempts);
        })
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_task_creation_workflow,
    bench_batch_task_workflow,
    bench_task_registry_workflow,
    bench_latency_measurement_workflow,
    bench_priority_workflow,
    bench_memory_workflow,
    bench_error_handling_workflow
);
criterion_main!(benches); 