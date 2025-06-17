use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use rust_task_queue::prelude::*;
use rust_task_queue::TaskResult;
use serde::{Deserialize, Serialize};

// Test task for autoscaler benchmarking
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct AutoscalerTestTask {
    id: u32,
    duration_ms: u64,
    cpu_intensive: bool,
}

#[async_trait::async_trait]
impl Task for AutoscalerTestTask {
    async fn execute(&self) -> TaskResult {
        if self.cpu_intensive {
            // CPU-intensive work simulation
            let mut sum = 0u64;
            for i in 0..100000 {
                sum = sum.wrapping_add(i * 2);
            }
            Ok(rmp_serde::to_vec(&sum)?)
        } else {
            // Simulate work without actual sleeping for benchmarks
            let cycles = self.duration_ms * 100;
            let mut result = 0u64;
            for i in 0..cycles {
                result = result.wrapping_add(i);
            }
            Ok(rmp_serde::to_vec(&format!("completed_{}_{}", self.id, result))?)
        }
    }
    
    fn name(&self) -> &str { "autoscaler_test_task" }
    fn timeout_seconds(&self) -> u64 { 30 }
}

fn bench_autoscaler_config_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("autoscaler_config_creation");
    
    group.bench_function("default_autoscaler_config", |b| {
        b.iter(|| {
            let config = AutoScalerConfig::default();
            black_box(config);
        })
    });
    
    group.bench_function("custom_autoscaler_config", |b| {
        b.iter(|| {
            let config = AutoScalerConfig {
                min_workers: 1,
                max_workers: 20,
                scale_up_threshold: 10.0,
                scale_down_threshold: 2.0,
                scale_up_count: 3,
                scale_down_count: 1,
            };
            black_box(config);
        })
    });
    
    group.bench_function("config_validation", |b| {
        b.iter(|| {
            let config = AutoScalerConfig::default();
            
            // Use the actual validate method
            let is_valid = config.validate().is_ok();
            
            black_box((config, is_valid));
        })
    });
    
    group.finish();
}

fn bench_scaling_decision_logic(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaling_decision_logic");
    
    // Create different load scenarios with f64 values
    let load_scenarios = vec![
        ("low_load", 2.0),      // 2.0 tasks per worker
        ("medium_load", 7.0),   // 7.0 tasks per worker  
        ("high_load", 15.0),    // 15.0 tasks per worker
        ("extreme_load", 50.0), // 50.0 tasks per worker
    ];
    
    for (scenario_name, tasks_per_worker) in load_scenarios {
        group.bench_function(&format!("scaling_decision_{}", scenario_name), |b| {
            b.iter(|| {
                let config = AutoScalerConfig::default();
                let current_workers = 2;
                
                // Simulate scaling decision logic using the actual field types
                let should_scale_up = tasks_per_worker > config.scale_up_threshold && current_workers < config.max_workers;
                let should_scale_down = tasks_per_worker < config.scale_down_threshold && current_workers > config.min_workers;
                
                let decision = if should_scale_up {
                    "scale_up"
                } else if should_scale_down {
                    "scale_down"
                } else {
                    "no_change"
                };
                
                black_box(decision);
            })
        });
    }
    
    group.finish();
}

fn bench_load_pattern_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("load_pattern_analysis");
    
    // Simulate different load patterns
    group.bench_function("burst_load_pattern", |b| {
        b.iter(|| {
            // Simulate burst load - many tasks at once
            let tasks: Vec<AutoscalerTestTask> = (0..20)
                .map(|i| AutoscalerTestTask {
                    id: i,
                    duration_ms: 50,
                    cpu_intensive: false,
                })
                .collect();
            black_box(tasks);
        })
    });
    
    group.bench_function("sustained_load_pattern", |b| {
        b.iter(|| {
            // Simulate sustained load - steady stream of tasks
            let tasks: Vec<AutoscalerTestTask> = (0..10)
                .map(|i| AutoscalerTestTask {
                    id: i,
                    duration_ms: 100,
                    cpu_intensive: false,
                })
                .collect();
            black_box(tasks);
        })
    });
    
    group.bench_function("gradual_load_pattern", |b| {
        b.iter(|| {
            // Simulate gradual load increase
            let tasks: Vec<AutoscalerTestTask> = (0..15)
                .map(|i| AutoscalerTestTask {
                    id: i,
                    duration_ms: 30,
                    cpu_intensive: false,
                })
                .collect();
            black_box(tasks);
        })
    });
    
    group.finish();
}

fn bench_cpu_vs_io_task_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("cpu_vs_io_task_analysis");
    
    group.bench_function("cpu_intensive_task_creation", |b| {
        b.iter(|| {
            let tasks: Vec<AutoscalerTestTask> = (0..10)
                .map(|i| AutoscalerTestTask {
                    id: i,
                    duration_ms: 0,
                    cpu_intensive: true,
                })
                .collect();
            black_box(tasks);
        })
    });
    
    group.bench_function("io_intensive_task_creation", |b| {
        b.iter(|| {
            let tasks: Vec<AutoscalerTestTask> = (0..10)
                .map(|i| AutoscalerTestTask {
                    id: i,
                    duration_ms: 100,
                    cpu_intensive: false,
                })
                .collect();
            black_box(tasks);
        })
    });
    
    group.bench_function("mixed_task_workload", |b| {
        b.iter(|| {
            let tasks: Vec<AutoscalerTestTask> = (0..10)
                .map(|i| AutoscalerTestTask {
                    id: i,
                    duration_ms: if i % 2 == 0 { 50 } else { 0 },
                    cpu_intensive: i % 2 != 0,
                })
                .collect();
            black_box(tasks);
        })
    });
    
    group.finish();
}

fn bench_scaling_thresholds_tuning(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaling_thresholds_tuning");
    
    // Test different threshold configurations with f64 values
    let threshold_configs = vec![
        ("conservative", 20.0, 5.0),  // High scale-up, low scale-down
        ("aggressive", 5.0, 1.0),     // Low scale-up, very low scale-down  
        ("balanced", 10.0, 3.0),      // Balanced thresholds
    ];
    
    for (config_name, scale_up_threshold, scale_down_threshold) in threshold_configs {
        group.bench_function(&format!("threshold_config_{}", config_name), |b| {
            b.iter(|| {
                let config = AutoScalerConfig {
                    min_workers: 1,
                    max_workers: 10,
                    scale_up_threshold,
                    scale_down_threshold,
                    scale_up_count: 2,
                    scale_down_count: 1,
                };
                
                // Simulate decision making with different tasks per worker ratios
                let tasks_per_worker_scenarios = [2.0, 4.0, 8.0, 12.0, 25.0];
                let decisions: Vec<_> = tasks_per_worker_scenarios.iter().map(|&tasks_per_worker| {
                    let current_workers = 3;
                    if tasks_per_worker > config.scale_up_threshold && current_workers < config.max_workers {
                        "scale_up"
                    } else if tasks_per_worker < config.scale_down_threshold && current_workers > config.min_workers {
                        "scale_down"
                    } else {
                        "no_change"
                    }
                }).collect();
                
                black_box((config, decisions));
            })
        });
    }
    
    group.finish();
}

fn bench_scale_count_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("scale_count_calculation");
    
    group.bench_function("scale_up_count_limiting", |b| {
        b.iter(|| {
            let config = AutoScalerConfig {
                min_workers: 1,
                max_workers: 10,
                scale_up_threshold: 5.0,
                scale_down_threshold: 1.0,
                scale_up_count: 5, // Want to add 5 workers
                scale_down_count: 1,
            };
            
            let current_workers = 8; // Only room for 2 more workers
            
            // Calculate actual scale count (limited by max_workers)
            let actual_scale_count = std::cmp::min(
                config.scale_up_count,
                config.max_workers - current_workers,
            );
            
            black_box((config, actual_scale_count));
        })
    });
    
    group.bench_function("scale_down_count_limiting", |b| {
        b.iter(|| {
            let config = AutoScalerConfig {
                min_workers: 2,
                max_workers: 10,
                scale_up_threshold: 5.0,
                scale_down_threshold: 1.0,
                scale_up_count: 2,
                scale_down_count: 3, // Want to remove 3 workers
            };
            
            let current_workers = 4; // Only room to remove 2 workers (to reach min of 2)
            
            // Calculate actual scale count (limited by min_workers)
            let actual_scale_count = std::cmp::min(
                config.scale_down_count,
                current_workers - config.min_workers,
            );
            
            black_box((config, actual_scale_count));
        })
    });
    
    group.finish();
}

fn bench_concurrent_scaling_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_scaling_simulation");
    
    // Test autoscaler performance under concurrent load simulation
    for concurrency in [2, 4, 8].iter() {
        group.throughput(Throughput::Elements(*concurrency as u64));
        
        group.bench_with_input(
            BenchmarkId::new("concurrent_task_submission", concurrency),
            concurrency,
            |b, &concurrency| {
                b.iter(|| {
                    // Simulate tasks from multiple "clients"
                    let all_tasks: Vec<Vec<AutoscalerTestTask>> = (0..concurrency)
                        .map(|client_id| {
                            (0..5).map(|i| AutoscalerTestTask {
                                id: (client_id * 5 + i) as u32,
                                duration_ms: 50,
                                cpu_intensive: false,
                            }).collect()
                        })
                        .collect();
                    
                    black_box(all_tasks);
                })
            }
        );
    }
    
    group.finish();
}

fn bench_worker_management_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("worker_management_simulation");
    
    // Simulate worker states and management
    #[derive(Debug, Clone)]
    struct WorkerState {
        id: String,
        status: String,
        current_task: Option<String>,
        started_at: std::time::Instant,
    }
    
    group.bench_function("worker_state_tracking", |b| {
        b.iter(|| {
            let workers: Vec<WorkerState> = (0..10)
                .map(|i| WorkerState {
                    id: format!("worker_{}", i),
                    status: if i % 3 == 0 { "idle" } else { "busy" }.to_string(),
                    current_task: if i % 3 == 0 { None } else { Some(format!("task_{}", i)) },
                    started_at: std::time::Instant::now(),
                })
                .collect();
            
            black_box(workers);
        })
    });
    
    group.bench_function("worker_count_calculation", |b| {
        b.iter(|| {
            let workers: Vec<WorkerState> = (0..10)
                .map(|i| WorkerState {
                    id: format!("worker_{}", i),
                    status: if i % 3 == 0 { "idle" } else { "busy" }.to_string(),
                    current_task: if i % 3 == 0 { None } else { Some(format!("task_{}", i)) },
                    started_at: std::time::Instant::now(),
                })
                .collect();
            
            let active_workers = workers.iter().filter(|w| w.status == "busy").count();
            let idle_workers = workers.iter().filter(|w| w.status == "idle").count();
            let total_workers = workers.len();
            
            black_box((active_workers, idle_workers, total_workers));
        })
    });
    
    group.finish();
}

fn bench_tasks_per_worker_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("tasks_per_worker_calculation");
    
    group.bench_function("tasks_per_worker_ratio", |b| {
        b.iter(|| {
            let scenarios = [
                (5, 10),   // 5 workers, 10 tasks = 2.0 tasks/worker
                (3, 15),   // 3 workers, 15 tasks = 5.0 tasks/worker
                (8, 4),    // 8 workers, 4 tasks = 0.5 tasks/worker
                (0, 10),   // 0 workers, 10 tasks = inf (special case)
            ];
            
            let ratios: Vec<f64> = scenarios.iter().map(|(workers, tasks)| {
                if *workers > 0 {
                    *tasks as f64 / *workers as f64
                } else {
                    *tasks as f64 // When no workers, use task count directly
                }
            }).collect();
            
            black_box(ratios);
        })
    });
    
    group.finish();
}

fn bench_memory_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_efficiency");
    
    group.bench_function("autoscaler_metadata_overhead", |b| {
        b.iter(|| {
            // Create scenario with many workers (simulated)
            let worker_configs: Vec<String> = (0..10)
                .map(|i| format!("worker_{}", i))
                .collect();
            
            // Simulate autoscaler tracking worker states
            let worker_states: std::collections::HashMap<String, String> = worker_configs
                .into_iter()
                .map(|id| (id, "active".to_string()))
                .collect();
            
            // Simulate task queue monitoring with various metrics
            let queue_stats = [
                ("high_priority", 5, 100, 2),    // name, pending, processed, failed
                ("normal_priority", 15, 300, 5),
                ("low_priority", 3, 150, 1),
            ];
            
            let queue_data: Vec<(String, usize, usize, usize)> = queue_stats.iter()
                .map(|(name, pending, processed, failed)| 
                    (name.to_string(), *pending, *processed, *failed))
                .collect();
            
            black_box((worker_states, queue_data));
        })
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_autoscaler_config_creation,
    bench_scaling_decision_logic,
    bench_load_pattern_analysis,
    bench_cpu_vs_io_task_analysis,
    bench_scaling_thresholds_tuning,
    bench_scale_count_calculation,
    bench_concurrent_scaling_simulation,
    bench_worker_management_simulation,
    bench_tasks_per_worker_calculation,
    bench_memory_efficiency
);
criterion_main!(benches); 