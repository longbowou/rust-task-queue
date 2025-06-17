use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Comprehensive metrics collector for task queue operations
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    counters: Arc<RwLock<HashMap<String, AtomicU64>>>,
    gauges: Arc<RwLock<HashMap<String, AtomicU64>>>,
    histograms: Arc<RwLock<HashMap<String, TaskHistogram>>>,
    start_time: Instant,
    memory_tracker: Arc<MemoryTracker>,
    performance_tracker: Arc<PerformanceTracker>,
    alert_manager: Arc<AlertManager>,
}

/// Memory usage tracking
#[derive(Debug)]
pub struct MemoryTracker {
    allocated_bytes: AtomicUsize,
    peak_memory: AtomicUsize,
    active_tasks: AtomicUsize,
    total_allocations: AtomicU64,
}

/// Task execution time histogram
#[derive(Debug)]
pub struct TaskHistogram {
    samples: Vec<Duration>,
    total_count: AtomicU64,
    total_duration: AtomicU64,
}

/// Detailed system metrics snapshot
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemMetrics {
    pub timestamp: DateTime<Utc>,
    pub uptime_seconds: u64,
    pub memory: MemoryMetrics,
    pub performance: PerformanceMetrics,
    pub tasks: TaskMetrics,
    pub queues: Vec<QueueDetailedMetrics>,
    pub workers: WorkerMetrics,
}

/// Memory usage metrics
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MemoryMetrics {
    pub current_bytes: usize,
    pub peak_bytes: usize,
    pub total_allocations: u64,
    pub active_tasks: usize,
    pub memory_efficiency: f64, // bytes per active task
}

/// Performance metrics  
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PerformanceMetrics {
    pub tasks_per_second: f64,
    pub average_execution_time_ms: f64,
    pub p95_execution_time_ms: f64,
    pub p99_execution_time_ms: f64,
    pub success_rate: f64,
    pub error_rate: f64,
}

/// Task execution metrics
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskMetrics {
    pub total_executed: u64,
    pub total_succeeded: u64,
    pub total_failed: u64,
    pub total_retried: u64,
    pub total_timed_out: u64,
    pub active_tasks: u64,
}

/// Queue-specific metrics (enhanced version)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueueDetailedMetrics {
    pub queue_name: String,
    pub pending_tasks: i64,
    pub processed_tasks: i64,
    pub failed_tasks: i64,
    pub average_wait_time_ms: f64,
}

/// Worker pool metrics
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkerMetrics {
    pub active_workers: u64,
    pub idle_workers: u64,
    pub busy_workers: u64,
    pub worker_utilization: f64,
    pub tasks_per_worker: f64,
}

/// Enhanced performance tracking with SLA monitoring
#[derive(Debug)]
pub struct PerformanceTracker {
    task_execution_times: Arc<RwLock<HashMap<String, Vec<Duration>>>>,
    #[allow(dead_code)] // Reserved for future queue latency tracking
    queue_latencies: Arc<RwLock<HashMap<String, Vec<Duration>>>>,
    error_rates: Arc<RwLock<HashMap<String, ErrorRateTracker>>>,
    sla_violations: Arc<RwLock<Vec<SLAViolation>>>,
}

/// Alert management system
#[derive(Debug)]
pub struct AlertManager {
    active_alerts: Arc<RwLock<HashMap<String, Alert>>>,
    alert_thresholds: Arc<RwLock<AlertThresholds>>,
}

/// Configurable alert thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    pub max_queue_size: u64,
    pub max_error_rate: f64,
    pub max_task_duration_ms: u64,
    pub max_memory_usage_mb: u64,
    pub max_worker_idle_time_sec: u64,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            max_queue_size: 10000,
            max_error_rate: 0.05, // 5%
            max_task_duration_ms: 300000, // 5 minutes
            max_memory_usage_mb: 1024, // 1GB
            max_worker_idle_time_sec: 300, // 5 minutes
        }
    }
}

/// Alert representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub severity: AlertSeverity,
    pub message: String,
    pub timestamp: SystemTime,
    pub metric_name: String,
    pub current_value: f64,
    pub threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}

/// SLA violation tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SLAViolation {
    pub violation_type: SLAViolationType,
    pub timestamp: SystemTime,
    pub details: String,
    pub metric_value: f64,
    pub threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SLAViolationType {
    TaskTimeoutExceeded,
    QueueBacklogTooHigh,
    ErrorRateTooHigh,
    MemoryUsageTooHigh,
    WorkerUtilizationTooLow,
}

/// Error rate tracking with time windows
#[derive(Debug)]
pub struct ErrorRateTracker {
    errors: Vec<SystemTime>,
    total_executions: u64,
    window_size: Duration,
}

impl ErrorRateTracker {
    pub fn new(window_size: Duration) -> Self {
        Self {
            errors: Vec::new(),
            total_executions: 0,
            window_size,
        }
    }

    pub fn record_execution(&mut self, is_error: bool) {
        self.total_executions += 1;
        if is_error {
            self.errors.push(SystemTime::now());
        }
        self.cleanup_old_entries();
    }

    pub fn error_rate(&mut self) -> f64 {
        self.cleanup_old_entries();
        if self.total_executions == 0 {
            return 0.0;
        }
        self.errors.len() as f64 / self.total_executions as f64
    }

    fn cleanup_old_entries(&mut self) {
        let cutoff = SystemTime::now() - self.window_size;
        self.errors.retain(|&time| time > cutoff);
    }
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            counters: Arc::new(RwLock::new(HashMap::new())),
            gauges: Arc::new(RwLock::new(HashMap::new())),
            histograms: Arc::new(RwLock::new(HashMap::new())),
            start_time: Instant::now(),
            memory_tracker: Arc::new(MemoryTracker::new()),
            performance_tracker: Arc::new(PerformanceTracker::new()),
            alert_manager: Arc::new(AlertManager::new()),
        }
    }

    /// Increment a counter metric
    pub async fn increment_counter(&self, name: &str, value: u64) {
        let counters = self.counters.read().await;
        if let Some(counter) = counters.get(name) {
            counter.fetch_add(value, Ordering::Relaxed);
        } else {
            drop(counters);
            let mut counters = self.counters.write().await;
            counters.entry(name.to_string())
                .or_insert_with(|| AtomicU64::new(0))
                .fetch_add(value, Ordering::Relaxed);
        }
    }

    /// Set a gauge metric
    pub async fn set_gauge(&self, name: &str, value: u64) {
        let gauges = self.gauges.read().await;
        if let Some(gauge) = gauges.get(name) {
            gauge.store(value, Ordering::Relaxed);
        } else {
            drop(gauges);
            let mut gauges = self.gauges.write().await;
            gauges.entry(name.to_string())
                .or_insert_with(|| AtomicU64::new(0))
                .store(value, Ordering::Relaxed);
        }
    }

    /// Record a timing measurement
    pub async fn record_timing(&self, name: &str, duration: Duration) {
        let mut histograms = self.histograms.write().await;
        let histogram = histograms.entry(name.to_string())
            .or_insert_with(TaskHistogram::new);
        histogram.record(duration);
    }

    /// Track memory allocation
    pub fn track_allocation(&self, bytes: usize) {
        self.memory_tracker.track_allocation(bytes);
    }

    /// Track memory deallocation
    pub fn track_deallocation(&self, bytes: usize) {
        self.memory_tracker.track_deallocation(bytes);
    }

    /// Track task start
    pub fn track_task_start(&self) {
        self.memory_tracker.track_task_start();
    }

    /// Track task completion
    pub fn track_task_end(&self) {
        self.memory_tracker.track_task_end();
    }

    /// Record task execution time for performance analysis
    pub async fn record_task_execution(&self, task_name: &str, duration: Duration, success: bool) {
        // Update execution time histogram
        let mut histograms = self.histograms.write().await;
        let histogram = histograms.entry(format!("task_execution_time_{}", task_name))
            .or_insert_with(TaskHistogram::new);
        histogram.record(duration);

        // Track in performance tracker
        self.performance_tracker.record_execution(task_name, duration, success).await;

        // Update counters
        self.increment_counter(&format!("tasks_executed_{}", task_name), 1).await;
        if success {
            self.increment_counter(&format!("tasks_succeeded_{}", task_name), 1).await;
        } else {
            self.increment_counter(&format!("tasks_failed_{}", task_name), 1).await;
        }

        // Check for alerts
        self.alert_manager.check_task_performance_alerts(task_name, duration, success).await;
    }

    /// Get comprehensive metrics snapshot
    pub async fn get_system_metrics(&self) -> SystemMetrics {
        let uptime = self.start_time.elapsed().as_secs();
        
        // Collect counter values
        let counters = self.counters.read().await;
        let total_executed = counters.get("tasks_executed")
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0);
        let total_succeeded = counters.get("tasks_succeeded")
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0);
        let total_failed = counters.get("tasks_failed")
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0);
        let total_retried = counters.get("tasks_retried")
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0);
        let total_timed_out = counters.get("tasks_timed_out")
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0);

        // Collect gauge values
        let gauges = self.gauges.read().await;
        let active_tasks = gauges.get("active_tasks")
            .map(|g| g.load(Ordering::Relaxed))
            .unwrap_or(0);
        let active_workers = gauges.get("active_workers")
            .map(|g| g.load(Ordering::Relaxed))
            .unwrap_or(0);

        // Collect timing data
        let histograms = self.histograms.read().await;
        let execution_histogram = histograms.get("task_execution_time");
        
        let (avg_execution_ms, p95_ms, p99_ms) = if let Some(hist) = execution_histogram {
            (
                hist.average().as_millis() as f64,
                hist.percentile(0.95).as_millis() as f64,
                hist.percentile(0.99).as_millis() as f64,
            )
        } else {
            (0.0, 0.0, 0.0)
        };

        // Calculate rates
        let tasks_per_second = if uptime > 0 {
            total_executed as f64 / uptime as f64
        } else {
            0.0
        };

        let success_rate = if total_executed > 0 {
            total_succeeded as f64 / total_executed as f64
        } else {
            0.0
        };

        let error_rate = if total_executed > 0 {
            total_failed as f64 / total_executed as f64
        } else {
            0.0
        };

        // Memory metrics
        let memory_metrics = self.memory_tracker.get_metrics();

        SystemMetrics {
            timestamp: Utc::now(),
            uptime_seconds: uptime,
            memory: memory_metrics,
            performance: PerformanceMetrics {
                tasks_per_second,
                average_execution_time_ms: avg_execution_ms,
                p95_execution_time_ms: p95_ms,
                p99_execution_time_ms: p99_ms,
                success_rate,
                error_rate,
            },
            tasks: TaskMetrics {
                total_executed,
                total_succeeded,
                total_failed,
                total_retried,
                total_timed_out,
                active_tasks,
            },
            queues: Vec::new(), // Would be populated by broker
            workers: WorkerMetrics {
                active_workers,
                idle_workers: 0, // Would be calculated from worker status
                busy_workers: 0, // Would be calculated from worker status
                worker_utilization: 0.0,
                tasks_per_worker: if active_workers > 0 {
                    total_executed as f64 / active_workers as f64
                } else {
                    0.0
                },
            },
        }
    }

    /// Get a simple metrics summary for quick debugging
    pub async fn get_metrics_summary(&self) -> String {
        let metrics = self.get_system_metrics().await;
        format!(
            "TaskQueue Metrics Summary:\n\
             - Uptime: {}s\n\
             - Tasks: {} executed, {} succeeded, {} failed\n\
             - Memory: {} bytes current, {} bytes peak\n\
             - Performance: {:.2} tasks/sec, {:.2}ms avg execution\n\
             - Workers: {} active\n\
             - Success Rate: {:.1}%",
            metrics.uptime_seconds,
            metrics.tasks.total_executed,
            metrics.tasks.total_succeeded,
            metrics.tasks.total_failed,
            metrics.memory.current_bytes,
            metrics.memory.peak_bytes,
            metrics.performance.tasks_per_second,
            metrics.performance.average_execution_time_ms,
            metrics.workers.active_workers,
            metrics.performance.success_rate * 100.0
        )
    }

    /// Get comprehensive performance report
    pub async fn get_performance_report(&self) -> PerformanceReport {
        let histograms = self.histograms.read().await;
        let mut task_performance = HashMap::new();

        for (name, histogram) in histograms.iter() {
            if name.starts_with("task_execution_time_") {
                let task_name = name.strip_prefix("task_execution_time_").unwrap();
                task_performance.insert(task_name.to_string(), TaskPerformanceMetrics {
                    avg_duration_ms: histogram.average().as_millis() as f64,
                    p50_duration_ms: histogram.percentile(0.50).as_millis() as u64,
                    p95_duration_ms: histogram.percentile(0.95).as_millis() as u64,
                    p99_duration_ms: histogram.percentile(0.99).as_millis() as u64,
                    total_executions: histogram.count(),
                });
            }
        }

        PerformanceReport {
            uptime_seconds: self.start_time.elapsed().as_secs(),
            task_performance,
            active_alerts: self.alert_manager.get_active_alerts().await,
            sla_violations: self.performance_tracker.get_recent_violations().await,
        }
    }

    /// Get real-time system health status
    pub async fn get_health_status(&self) -> SystemHealthStatus {
        let memory_metrics = self.memory_tracker.get_metrics();
        let active_alerts = self.alert_manager.get_active_alerts().await;
        
        let status = if active_alerts.iter().any(|a| matches!(a.severity, AlertSeverity::Critical | AlertSeverity::Emergency)) {
            HealthStatus::Critical
        } else if !active_alerts.is_empty() {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        };

        SystemHealthStatus {
            status,
            memory_usage_mb: (memory_metrics.current_bytes / (1024 * 1024)) as u64,
            uptime_seconds: self.start_time.elapsed().as_secs(),
            active_alert_count: active_alerts.len() as u32,
            critical_alert_count: active_alerts.iter()
                .filter(|a| matches!(a.severity, AlertSeverity::Critical | AlertSeverity::Emergency))
                .count() as u32,
        }
    }
}

impl MemoryTracker {
    pub fn new() -> Self {
        Self {
            allocated_bytes: AtomicUsize::new(0),
            peak_memory: AtomicUsize::new(0),
            active_tasks: AtomicUsize::new(0),
            total_allocations: AtomicU64::new(0),
        }
    }

    pub fn track_allocation(&self, bytes: usize) {
        let current = self.allocated_bytes.fetch_add(bytes, Ordering::Relaxed) + bytes;
        
        // Update peak if necessary
        let mut peak = self.peak_memory.load(Ordering::Relaxed);
        while current > peak {
            match self.peak_memory.compare_exchange_weak(
                peak,
                current,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(new_peak) => peak = new_peak,
            }
        }
        
        self.total_allocations.fetch_add(1, Ordering::Relaxed);
    }

    pub fn track_deallocation(&self, bytes: usize) {
        self.allocated_bytes.fetch_sub(bytes, Ordering::Relaxed);
    }

    pub fn track_task_start(&self) {
        self.active_tasks.fetch_add(1, Ordering::Relaxed);
    }

    pub fn track_task_end(&self) {
        self.active_tasks.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn get_metrics(&self) -> MemoryMetrics {
        let current = self.allocated_bytes.load(Ordering::Relaxed);
        let peak = self.peak_memory.load(Ordering::Relaxed);
        let active = self.active_tasks.load(Ordering::Relaxed);
        let total_allocs = self.total_allocations.load(Ordering::Relaxed);

        let efficiency = if active > 0 {
            current as f64 / active as f64
        } else {
            0.0
        };

        MemoryMetrics {
            current_bytes: current,
            peak_bytes: peak,
            total_allocations: total_allocs,
            active_tasks: active,
            memory_efficiency: efficiency,
        }
    }
}

impl TaskHistogram {
    pub fn new() -> Self {
        Self {
            samples: Vec::new(),
            total_count: AtomicU64::new(0),
            total_duration: AtomicU64::new(0),
        }
    }

    pub fn record(&mut self, duration: Duration) {
        self.samples.push(duration);
        self.total_count.fetch_add(1, Ordering::Relaxed);
        self.total_duration.fetch_add(duration.as_millis() as u64, Ordering::Relaxed);
        
        // Keep only recent samples to prevent memory bloat
        if self.samples.len() > 10000 {
            self.samples.drain(..5000);
        }
    }

    pub fn average(&self) -> Duration {
        let count = self.total_count.load(Ordering::Relaxed);
        if count == 0 {
            return Duration::from_millis(0);
        }
        
        let total_ms = self.total_duration.load(Ordering::Relaxed);
        Duration::from_millis(total_ms / count)
    }

    pub fn percentile(&self, p: f64) -> Duration {
        if self.samples.is_empty() {
            return Duration::from_millis(0);
        }

        let mut sorted_samples = self.samples.clone();
        sorted_samples.sort();
        
        let index = (sorted_samples.len() as f64 * p).ceil() as usize - 1;
        sorted_samples[index.min(sorted_samples.len() - 1)]
    }

    pub fn count(&self) -> u64 {
        self.total_count.load(Ordering::Relaxed)
    }
}

impl PerformanceTracker {
    pub fn new() -> Self {
        Self {
            task_execution_times: Arc::new(RwLock::new(HashMap::new())),
            queue_latencies: Arc::new(RwLock::new(HashMap::new())),
            error_rates: Arc::new(RwLock::new(HashMap::new())),
            sla_violations: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn record_execution(&self, task_name: &str, duration: Duration, success: bool) {
        // Track execution time
        let mut times = self.task_execution_times.write().await;
        times.entry(task_name.to_string())
            .or_insert_with(Vec::new)
            .push(duration);

        // Track error rate
        let mut error_rates = self.error_rates.write().await;
        error_rates.entry(task_name.to_string())
            .or_insert_with(|| ErrorRateTracker::new(Duration::from_secs(300))) // 5-minute window
            .record_execution(!success);
    }

    pub async fn get_recent_violations(&self) -> Vec<SLAViolation> {
        let violations = self.sla_violations.read().await;
        violations.clone()
    }
}

impl AlertManager {
    pub fn new() -> Self {
        Self {
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_thresholds: Arc::new(RwLock::new(AlertThresholds::default())),
        }
    }

    pub async fn check_task_performance_alerts(&self, task_name: &str, duration: Duration, _success: bool) {
        let thresholds = self.alert_thresholds.read().await;
        
        // Check task duration
        if duration.as_millis() > thresholds.max_task_duration_ms as u128 {
            let alert = Alert {
                id: format!("task_duration_{}_{}", task_name, SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
                severity: AlertSeverity::Warning,
                message: format!("Task {} took {}ms (threshold: {}ms)", task_name, duration.as_millis(), thresholds.max_task_duration_ms),
                timestamp: SystemTime::now(),
                metric_name: "task_duration".to_string(),
                current_value: duration.as_millis() as f64,
                threshold: thresholds.max_task_duration_ms as f64,
            };
            
            let mut alerts = self.active_alerts.write().await;
            alerts.insert(alert.id.clone(), alert);
        }
    }

    pub async fn get_active_alerts(&self) -> Vec<Alert> {
        let alerts = self.active_alerts.read().await;
        alerts.values().cloned().collect()
    }
}

/// Enhanced performance report
#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub uptime_seconds: u64,
    pub task_performance: HashMap<String, TaskPerformanceMetrics>,
    pub active_alerts: Vec<Alert>,
    pub sla_violations: Vec<SLAViolation>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskPerformanceMetrics {
    pub avg_duration_ms: f64,
    pub p50_duration_ms: u64,
    pub p95_duration_ms: u64,
    pub p99_duration_ms: u64,
    pub total_executions: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemHealthStatus {
    pub status: HealthStatus,
    pub memory_usage_mb: u64,
    pub uptime_seconds: u64,
    pub active_alert_count: u32,
    pub critical_alert_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for MemoryTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for TaskHistogram {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for PerformanceTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;


    #[tokio::test]
    async fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new();
        let metrics = collector.get_system_metrics().await;
        
        assert_eq!(metrics.tasks.total_executed, 0);
        assert_eq!(metrics.memory.current_bytes, 0);
    }

    #[tokio::test]
    async fn test_counter_increment() {
        let collector = MetricsCollector::new();
        
        collector.increment_counter("test_counter", 5).await;
        collector.increment_counter("test_counter", 3).await;
        
        let counters = collector.counters.read().await;
        let value = counters.get("test_counter").unwrap().load(Ordering::Relaxed);
        assert_eq!(value, 8);
    }

    #[tokio::test]
    async fn test_gauge_setting() {
        let collector = MetricsCollector::new();
        
        collector.set_gauge("test_gauge", 42).await;
        collector.set_gauge("test_gauge", 100).await;
        
        let gauges = collector.gauges.read().await;
        let value = gauges.get("test_gauge").unwrap().load(Ordering::Relaxed);
        assert_eq!(value, 100);
    }

    #[tokio::test]
    async fn test_timing_recording() {
        let collector = MetricsCollector::new();
        
        collector.record_timing("test_timing", Duration::from_millis(100)).await;
        collector.record_timing("test_timing", Duration::from_millis(200)).await;
        
        let histograms = collector.histograms.read().await;
        let histogram = histograms.get("test_timing").unwrap();
        let avg = histogram.average();
        
        assert_eq!(avg, Duration::from_millis(150));
    }

    #[test]
    fn test_memory_tracker() {
        let tracker = MemoryTracker::new();
        
        tracker.track_allocation(1000);
        tracker.track_allocation(500);
        tracker.track_task_start();
        tracker.track_task_start();
        
        let metrics = tracker.get_metrics();
        assert_eq!(metrics.current_bytes, 1500);
        assert_eq!(metrics.peak_bytes, 1500);
        assert_eq!(metrics.active_tasks, 2);
        assert_eq!(metrics.memory_efficiency, 750.0);
        
        tracker.track_deallocation(300);
        tracker.track_task_end();
        
        let metrics = tracker.get_metrics();
        assert_eq!(metrics.current_bytes, 1200);
        assert_eq!(metrics.active_tasks, 1);
        assert_eq!(metrics.memory_efficiency, 1200.0);
    }

    #[test]
    fn test_histogram_percentiles() {
        let mut histogram = TaskHistogram::new();
        
        // Add samples: 10, 20, 30, ..., 100 ms
        for i in 1..=10 {
            histogram.record(Duration::from_millis(i * 10));
        }
        
        assert_eq!(histogram.average(), Duration::from_millis(55));
        assert_eq!(histogram.percentile(0.9), Duration::from_millis(90)); // 90th percentile
        assert_eq!(histogram.percentile(0.95), Duration::from_millis(100)); // 95th percentile
    }

    #[tokio::test]
    async fn test_metrics_summary() {
        let collector = MetricsCollector::new();
        
        collector.increment_counter("tasks_executed", 100).await;
        collector.increment_counter("tasks_succeeded", 95).await;
        collector.increment_counter("tasks_failed", 5).await;
        collector.set_gauge("active_workers", 3).await;
        
        let summary = collector.get_metrics_summary().await;
        
        assert!(summary.contains("100 executed"));
        assert!(summary.contains("95 succeeded"));
        assert!(summary.contains("5 failed"));
        assert!(summary.contains("3 active"));
        assert!(summary.contains("95.0%")); // Success rate
    }
} 