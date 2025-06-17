use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use rust_task_queue::prelude::*;
use rust_task_queue::{TaskResult, TaskPriority};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

// Test task for Redis operations simulation
#[derive(Debug, Serialize, Deserialize, Clone)]
struct RedisBenchTask {
    id: u32,
    data: String,
    priority: u8,
}

#[async_trait::async_trait]
impl Task for RedisBenchTask {
    async fn execute(&self) -> TaskResult {
        Ok(rmp_serde::to_vec(&format!("processed_{}", self.id))?)
    }
    
    fn name(&self) -> &str { "redis_bench_task" }
    
    fn priority(&self) -> TaskPriority {
        match self.priority {
            0..=2 => TaskPriority::Low,
            3..=5 => TaskPriority::Normal,
            6..=8 => TaskPriority::High,
            _ => TaskPriority::Critical,
        }
    }
}

fn create_test_task(id: u32, size: usize, priority: u8) -> RedisBenchTask {
    RedisBenchTask {
        id,
        data: "x".repeat(size),
        priority,
    }
}

// Simulate queue operations without Redis
struct MockQueue {
    queues: HashMap<String, VecDeque<Vec<u8>>>,
}

impl MockQueue {
    fn new() -> Self {
        Self {
            queues: HashMap::new(),
        }
    }
    
    fn enqueue(&mut self, queue_name: &str, data: Vec<u8>) {
        self.queues.entry(queue_name.to_string())
            .or_default()
            .push_back(data);
    }
    
    fn dequeue(&mut self, queue_name: &str) -> Option<Vec<u8>> {
        self.queues.get_mut(queue_name)?.pop_front()
    }
    
    fn queue_length(&self, queue_name: &str) -> usize {
        self.queues.get(queue_name).map(|q| q.len()).unwrap_or(0)
    }
    
    fn clear_queue(&mut self, queue_name: &str) {
        if let Some(queue) = self.queues.get_mut(queue_name) {
            queue.clear();
        }
    }
}

fn bench_message_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_serialization");
    
    // Test different message sizes
    for size in [100, 1024, 10240].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("task_serialization", size),
            size,
            |b, &size| {
                let task = create_test_task(1, size, 5);
                b.iter(|| {
                    let serialized = rmp_serde::to_vec(&task).unwrap();
                    black_box(serialized);
                })
            }
        );
        
        group.bench_with_input(
            BenchmarkId::new("task_deserialization", size),
            size,
            |b, &size| {
                let task = create_test_task(1, size, 5);
                let serialized = rmp_serde::to_vec(&task).unwrap();
                b.iter(|| {
                    let deserialized: RedisBenchTask = rmp_serde::from_slice(&serialized).unwrap();
                    black_box(deserialized);
                })
            }
        );
    }
    
    group.finish();
}

fn bench_queue_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue_operations");
    
    group.bench_function("queue_creation", |b| {
        b.iter(|| {
            let queue = MockQueue::new();
            black_box(queue);
        })
    });
    
    group.bench_function("single_enqueue", |b| {
        let mut queue = MockQueue::new();
        let task = create_test_task(1, 100, 5);
        let task_data = rmp_serde::to_vec(&task).unwrap();
        
        b.iter(|| {
            queue.enqueue("bench_queue", task_data.clone());
        })
    });
    
    group.bench_function("single_dequeue", |b| {
        let mut queue = MockQueue::new();
        let task = create_test_task(1, 100, 5);
        let task_data = rmp_serde::to_vec(&task).unwrap();
        
        // Pre-populate queue
        for _ in 0..1000 {
            queue.enqueue("bench_queue", task_data.clone());
        }
        
        b.iter(|| {
            let result = queue.dequeue("bench_queue");
            black_box(result);
        })
    });
    
    group.bench_function("queue_length_check", |b| {
        let mut queue = MockQueue::new();
        let task = create_test_task(1, 100, 5);
        let task_data = rmp_serde::to_vec(&task).unwrap();
        
        // Pre-populate queue
        for _ in 0..100 {
            queue.enqueue("bench_queue", task_data.clone());
        }
        
        b.iter(|| {
            let length = queue.queue_length("bench_queue");
            black_box(length);
        })
    });
    
    group.finish();
}

fn bench_batch_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_operations");
    
    // Test different batch sizes
    for batch_size in [10, 50, 100, 500].iter() {
        group.throughput(Throughput::Elements(*batch_size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("batch_enqueue", batch_size),
            batch_size,
            |b, &batch_size| {
                b.iter(|| {
                    let mut queue = MockQueue::new();
                    for i in 0..batch_size {
                        let task = create_test_task(i as u32, 100, 5);
                        let task_data = rmp_serde::to_vec(&task).unwrap();
                        queue.enqueue("bench_queue", task_data);
                    }
                    black_box(queue);
                })
            }
        );
        
        group.bench_with_input(
            BenchmarkId::new("batch_dequeue", batch_size),
            batch_size,
            |b, &batch_size| {
                b.iter(|| {
                    let mut queue = MockQueue::new();
                    
                    // Pre-populate queue
                    for i in 0..batch_size {
                        let task = create_test_task(i as u32, 100, 5);
                        let task_data = rmp_serde::to_vec(&task).unwrap();
                        queue.enqueue("bench_queue", task_data);
                    }
                    
                    // Dequeue all items
                    let mut results = Vec::new();
                    for _ in 0..batch_size {
                        if let Some(data) = queue.dequeue("bench_queue") {
                            results.push(data);
                        }
                    }
                    black_box(results);
                })
            }
        );
    }
    
    group.finish();
}

fn bench_priority_queue_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("priority_queue_simulation");
    
    group.bench_function("priority_task_creation", |b| {
        b.iter(|| {
            let tasks: Vec<RedisBenchTask> = (0..100)
                .map(|i| create_test_task(i, 100, (i % 10) as u8))
                .collect();
            black_box(tasks);
        })
    });
    
    group.bench_function("priority_task_sorting", |b| {
        let mut tasks: Vec<RedisBenchTask> = (0..100)
            .map(|i| create_test_task(i, 100, (i % 10) as u8))
            .collect();
        
        b.iter(|| {
            tasks.sort_by_key(|task| task.priority());
            black_box(&tasks);
        })
    });
    
    group.finish();
}

fn bench_concurrent_queue_access_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_queue_access_simulation");
    
    // Simulate concurrent access by creating multiple queue operations
    for concurrency in [2, 4, 8, 16].iter() {
        group.throughput(Throughput::Elements(*concurrency as u64));
        
        group.bench_with_input(
            BenchmarkId::new("concurrent_operations", concurrency),
            concurrency,
            |b, &concurrency| {
                b.iter(|| {
                    let mut queues: Vec<MockQueue> = (0..concurrency)
                        .map(|_| MockQueue::new())
                        .collect();
                    
                    // Simulate concurrent enqueue operations
                    for (i, queue) in queues.iter_mut().enumerate() {
                        let task = create_test_task(i as u32, 100, 5);
                        let task_data = rmp_serde::to_vec(&task).unwrap();
                        queue.enqueue(&format!("queue_{}", i), task_data);
                    }
                    
                    black_box(queues);
                })
            }
        );
    }
    
    group.finish();
}

fn bench_queue_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue_memory_usage");
    
    group.bench_function("large_queue_creation", |b| {
        b.iter(|| {
            let mut queue = MockQueue::new();
            
            // Add many small tasks
            for i in 0..1000 {
                let task = create_test_task(i, 50, 5);
                let task_data = rmp_serde::to_vec(&task).unwrap();
                queue.enqueue("large_queue", task_data);
            }
            
            black_box(queue);
        })
    });
    
    group.bench_function("queue_cleanup", |b| {
        b.iter(|| {
            let mut queue = MockQueue::new();
            
            // Add tasks
            for i in 0..100 {
                let task = create_test_task(i, 100, 5);
                let task_data = rmp_serde::to_vec(&task).unwrap();
                queue.enqueue("cleanup_queue", task_data);
            }
            
            // Clear queue
            queue.clear_queue("cleanup_queue");
            black_box(queue);
        })
    });
    
    group.finish();
}

fn bench_task_wrapper_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("task_wrapper_operations");
    
    group.bench_function("task_wrapper_creation", |b| {
        let task = create_test_task(1, 100, 5);
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
        let task = create_test_task(1, 100, 5);
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

criterion_group!(
    benches,
    bench_message_serialization,
    bench_queue_operations,
    bench_batch_operations,
    bench_priority_queue_simulation,
    bench_concurrent_queue_access_simulation,
    bench_queue_memory_usage,
    bench_task_wrapper_operations
);
criterion_main!(benches); 