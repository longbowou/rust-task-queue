use crate::{queue::queue_names, QueueMetrics, RedisBroker, TaskQueueError};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Enhanced autoscaler configuration with multi-dimensional scaling triggers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoScalerConfig {
    pub min_workers: usize,
    pub max_workers: usize,
    pub scale_up_count: usize,
    pub scale_down_count: usize,

    // Multi-dimensional thresholds
    pub scaling_triggers: ScalingTriggers,

    // Adaptive learning parameters
    pub enable_adaptive_thresholds: bool,
    pub learning_rate: f64,
    pub adaptation_window_minutes: u32,

    // Hysteresis and stability
    pub scale_up_cooldown_seconds: u64,
    pub scale_down_cooldown_seconds: u64,
    pub consecutive_signals_required: usize,

    // Performance targets for adaptive learning
    pub target_sla: SLATargets,
}

/// Multi-dimensional scaling triggers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingTriggers {
    pub queue_pressure_threshold: f64,     // Weighted queue depth score
    pub worker_utilization_threshold: f64, // Target worker utilization %
    pub task_complexity_threshold: f64,    // Complex task overload factor
    pub error_rate_threshold: f64,         // Maximum acceptable error rate
    pub memory_pressure_threshold: f64,    // Memory usage per worker (MB)
}

/// SLA targets for adaptive threshold learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SLATargets {
    pub max_p95_latency_ms: f64,
    pub min_success_rate: f64,
    pub max_queue_wait_time_ms: f64,
    pub target_worker_utilization: f64,
}

impl Default for AutoScalerConfig {
    fn default() -> Self {
        Self {
            min_workers: 1,
            max_workers: 20,
            scale_up_count: 2,
            scale_down_count: 1,

            scaling_triggers: ScalingTriggers {
                queue_pressure_threshold: 0.75,
                worker_utilization_threshold: 0.80,
                task_complexity_threshold: 1.5,
                error_rate_threshold: 0.05,
                memory_pressure_threshold: 512.0,
            },

            enable_adaptive_thresholds: true,
            learning_rate: 0.1,
            adaptation_window_minutes: 30,

            scale_up_cooldown_seconds: 60,
            scale_down_cooldown_seconds: 300,
            consecutive_signals_required: 2,

            target_sla: SLATargets {
                max_p95_latency_ms: 5000.0,
                min_success_rate: 0.95,
                max_queue_wait_time_ms: 10000.0,
                target_worker_utilization: 0.70,
            },
        }
    }
}

impl AutoScalerConfig {
    pub fn validate(&self) -> Result<(), TaskQueueError> {
        if self.min_workers == 0 {
            return Err(TaskQueueError::Configuration(
                "Minimum workers must be greater than 0".to_string(),
            ));
        }

        if self.max_workers < self.min_workers {
            return Err(TaskQueueError::Configuration(
                "Maximum workers must be greater than or equal to minimum workers".to_string(),
            ));
        }

        if self.max_workers > 1000 {
            return Err(TaskQueueError::Configuration(
                "Maximum workers cannot exceed 1000".to_string(),
            ));
        }

        if self.scale_up_count == 0 || self.scale_up_count > 50 {
            return Err(TaskQueueError::Configuration(
                "Scale up count must be between 1 and 50".to_string(),
            ));
        }

        if self.scale_down_count == 0 || self.scale_down_count > 50 {
            return Err(TaskQueueError::Configuration(
                "Scale down count must be between 1 and 50".to_string(),
            ));
        }

        // Validate scaling triggers
        let triggers = &self.scaling_triggers;
        if triggers.queue_pressure_threshold <= 0.0 || triggers.queue_pressure_threshold > 2.0 {
            return Err(TaskQueueError::Configuration(
                "Queue pressure threshold must be between 0.1 and 2.0".to_string(),
            ));
        }

        if triggers.worker_utilization_threshold <= 0.0
            || triggers.worker_utilization_threshold > 1.0
        {
            return Err(TaskQueueError::Configuration(
                "Worker utilization threshold must be between 0.1 and 1.0".to_string(),
            ));
        }

        if triggers.error_rate_threshold < 0.0 || triggers.error_rate_threshold > 1.0 {
            return Err(TaskQueueError::Configuration(
                "Error rate threshold must be between 0.0 and 1.0".to_string(),
            ));
        }

        if self.learning_rate <= 0.0 || self.learning_rate > 1.0 {
            return Err(TaskQueueError::Configuration(
                "Learning rate must be between 0.01 and 1.0".to_string(),
            ));
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum ScalingAction {
    ScaleUp(usize),
    ScaleDown(usize),
    NoAction,
}

/// Enhanced metrics with multi-dimensional analysis
#[derive(Debug)]
pub struct AutoScalerMetrics {
    pub active_workers: i64,
    pub total_pending_tasks: i64,
    pub queue_metrics: Vec<QueueMetrics>,

    // Enhanced metrics
    pub queue_pressure_score: f64,
    pub worker_utilization: f64,
    pub task_complexity_factor: f64,
    pub error_rate: f64,
    pub memory_pressure_mb: f64,
    pub avg_queue_wait_time_ms: f64,
    pub throughput_trend: f64,
}

/// Adaptive threshold controller that learns from system performance
#[derive(Debug)]
pub struct AdaptiveThresholdController {
    current_thresholds: ScalingTriggers,
    performance_history: Vec<PerformanceSnapshot>,
    last_adaptation: Instant,
    learning_rate: f64,
    target_sla: SLATargets,
    adaptation_window: Duration,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields reserved for future Phase 2/3 enhancements
struct PerformanceSnapshot {
    timestamp: Instant,
    latency_p95_ms: f64,
    success_rate: f64,
    queue_wait_time_ms: f64,
    worker_utilization: f64,
    scaling_action_taken: Option<ScalingAction>,
}

/// Hysteresis controller to prevent oscillations
#[derive(Debug)]
pub struct HysteresisController {
    scale_up_consecutive_signals: usize,
    scale_down_consecutive_signals: usize,
    required_consecutive_signals: usize,
    last_scaling_action: Option<(Instant, ScalingAction)>,
    scale_up_cooldown: Duration,
    scale_down_cooldown: Duration,
}

pub struct AutoScaler {
    broker: Arc<RedisBroker>,
    config: AutoScalerConfig,
    adaptive_controller: Option<AdaptiveThresholdController>,
    hysteresis_controller: HysteresisController,
    metrics_history: Vec<AutoScalerMetrics>,
    start_time: Instant,
}

impl AutoScaler {
    pub fn new(broker: Arc<RedisBroker>) -> Self {
        let config = AutoScalerConfig::default();
        Self::with_config(broker, config)
    }

    pub fn with_config(broker: Arc<RedisBroker>, config: AutoScalerConfig) -> Self {
        let adaptive_controller = if config.enable_adaptive_thresholds {
            Some(AdaptiveThresholdController::new(
                config.scaling_triggers.clone(),
                config.learning_rate,
                config.target_sla.clone(),
                Duration::from_secs(config.adaptation_window_minutes as u64 * 60),
            ))
        } else {
            None
        };

        let hysteresis_controller = HysteresisController::new(
            config.consecutive_signals_required,
            Duration::from_secs(config.scale_up_cooldown_seconds),
            Duration::from_secs(config.scale_down_cooldown_seconds),
        );

        Self {
            broker,
            config,
            adaptive_controller,
            hysteresis_controller,
            metrics_history: Vec::new(),
            start_time: Instant::now(),
        }
    }

    pub async fn collect_metrics(&self) -> Result<AutoScalerMetrics, TaskQueueError> {
        let active_workers = self.broker.get_active_workers().await?;

        let queues = [
            queue_names::DEFAULT,
            queue_names::HIGH_PRIORITY,
            queue_names::LOW_PRIORITY,
        ];
        let mut queue_metrics = Vec::new();
        let mut total_pending_tasks = 0;
        let mut total_processed_tasks = 0;
        let mut total_failed_tasks = 0;

        for queue in &queues {
            let metrics = self.broker.get_queue_metrics(queue).await?;
            total_pending_tasks += metrics.pending_tasks;
            total_processed_tasks += metrics.processed_tasks;
            total_failed_tasks += metrics.failed_tasks;
            queue_metrics.push(metrics);
        }

        // Calculate enhanced metrics
        let queue_pressure_score =
            self.calculate_queue_pressure_score(&queue_metrics, active_workers);
        let worker_utilization = self.calculate_worker_utilization(active_workers, &queue_metrics);
        let task_complexity_factor = self.calculate_task_complexity_factor(&queue_metrics);
        let error_rate = self.calculate_error_rate(total_processed_tasks, total_failed_tasks);
        let memory_pressure_mb = self.estimate_memory_pressure(active_workers);
        let avg_queue_wait_time_ms = self.calculate_avg_queue_wait_time(&queue_metrics);
        let throughput_trend = self.calculate_throughput_trend(&queue_metrics);

        Ok(AutoScalerMetrics {
            active_workers,
            total_pending_tasks,
            queue_metrics,
            queue_pressure_score,
            worker_utilization,
            task_complexity_factor,
            error_rate,
            memory_pressure_mb,
            avg_queue_wait_time_ms,
            throughput_trend,
        })
    }

    pub fn decide_scaling_action(
        &mut self,
        metrics: &AutoScalerMetrics,
    ) -> Result<ScalingAction, TaskQueueError> {
        let current_workers = metrics.active_workers as usize;

        // Get current thresholds (potentially adaptive)
        let thresholds = if let Some(ref adaptive) = self.adaptive_controller {
            adaptive.get_current_thresholds().clone()
        } else {
            self.config.scaling_triggers.clone()
        };

        // Multi-dimensional scaling decision
        let scale_up_signals = Self::count_scale_up_signals_static(metrics, &thresholds);
        let scale_down_signals = Self::count_scale_down_signals_static(metrics, &thresholds);

        let proposed_action = if scale_up_signals >= 2 && current_workers < self.config.max_workers
        {
            let scale_count = std::cmp::min(
                self.config.scale_up_count,
                self.config.max_workers - current_workers,
            );
            ScalingAction::ScaleUp(scale_count)
        } else if scale_down_signals >= 2 && current_workers > self.config.min_workers {
            let scale_count = std::cmp::min(
                self.config.scale_down_count,
                current_workers - self.config.min_workers,
            );
            ScalingAction::ScaleDown(scale_count)
        } else {
            ScalingAction::NoAction
        };

        // Apply hysteresis control
        let final_action = if self
            .hysteresis_controller
            .should_execute_scaling(&proposed_action)
        {
            #[cfg(feature = "tracing")]
            match &proposed_action {
                ScalingAction::ScaleUp(count) => {
                    tracing::info!(
                        "Enhanced auto-scaling up: queue_pressure={:.2}, utilization={:.2}, complexity={:.2}, adding {} workers",
                        metrics.queue_pressure_score,
                        metrics.worker_utilization,
                        metrics.task_complexity_factor,
                        count
                    );
                }
                ScalingAction::ScaleDown(count) => {
                    tracing::info!(
                        "Enhanced auto-scaling down: queue_pressure={:.2}, utilization={:.2}, removing {} workers",
                        metrics.queue_pressure_score,
                        metrics.worker_utilization,
                        count
                    );
                }
                _ => {}
            }

            proposed_action
        } else {
            ScalingAction::NoAction
        };

        // Record for adaptive learning
        if let Some(ref mut adaptive) = self.adaptive_controller {
            adaptive.record_performance_snapshot(metrics, &final_action);
        }

        // Store metrics history for trend analysis
        self.metrics_history.push(metrics.clone());
        if self.metrics_history.len() > 100 {
            self.metrics_history.remove(0);
        }

        Ok(final_action)
    }

    fn count_scale_up_signals_static(
        metrics: &AutoScalerMetrics,
        thresholds: &ScalingTriggers,
    ) -> usize {
        let mut signals = 0;

        if metrics.queue_pressure_score > thresholds.queue_pressure_threshold {
            signals += 1;
        }
        if metrics.worker_utilization > thresholds.worker_utilization_threshold {
            signals += 1;
        }
        if metrics.task_complexity_factor > thresholds.task_complexity_threshold {
            signals += 1;
        }
        if metrics.error_rate > thresholds.error_rate_threshold {
            signals += 1;
        }
        if metrics.memory_pressure_mb > thresholds.memory_pressure_threshold {
            signals += 1;
        }

        signals
    }

    fn count_scale_down_signals_static(
        metrics: &AutoScalerMetrics,
        thresholds: &ScalingTriggers,
    ) -> usize {
        let mut signals = 0;

        // Scale down when metrics are well below thresholds
        if metrics.queue_pressure_score < thresholds.queue_pressure_threshold * 0.3 {
            signals += 1;
        }
        if metrics.worker_utilization < thresholds.worker_utilization_threshold * 0.4 {
            signals += 1;
        }
        if metrics.task_complexity_factor < thresholds.task_complexity_threshold * 0.5 {
            signals += 1;
        }
        if metrics.error_rate < thresholds.error_rate_threshold * 0.2 {
            signals += 1;
        }
        if metrics.memory_pressure_mb < thresholds.memory_pressure_threshold * 0.5 {
            signals += 1;
        }

        signals
    }

    fn calculate_queue_pressure_score(
        &self,
        queue_metrics: &[QueueMetrics],
        active_workers: i64,
    ) -> f64 {
        let mut weighted_pressure = 0.0;
        let queue_weights = [
            (queue_names::HIGH_PRIORITY, 3.0),
            (queue_names::DEFAULT, 1.0),
            (queue_names::LOW_PRIORITY, 0.3),
        ];

        for metrics in queue_metrics {
            let weight = queue_weights
                .iter()
                .find(|(name, _)| *name == metrics.queue_name)
                .map(|(_, w)| *w)
                .unwrap_or(1.0);

            let queue_pressure = if active_workers > 0 {
                metrics.pending_tasks as f64 / active_workers as f64
            } else {
                metrics.pending_tasks as f64
            };

            weighted_pressure += queue_pressure * weight;
        }

        weighted_pressure / queue_weights.len() as f64
    }

    fn calculate_worker_utilization(
        &self,
        active_workers: i64,
        queue_metrics: &[QueueMetrics],
    ) -> f64 {
        if active_workers == 0 {
            return 0.0;
        }

        let total_work = queue_metrics
            .iter()
            .map(|m| m.pending_tasks + m.processed_tasks)
            .sum::<i64>();

        let utilization = total_work as f64 / (active_workers as f64 * 100.0);
        utilization.min(1.0)
    }

    fn calculate_task_complexity_factor(&self, _queue_metrics: &[QueueMetrics]) -> f64 {
        // Simplified complexity calculation
        // In a real implementation, this would analyze historical execution times
        1.0
    }

    fn calculate_error_rate(&self, processed: i64, failed: i64) -> f64 {
        let total = processed + failed;
        if total == 0 {
            return 0.0;
        }
        failed as f64 / total as f64
    }

    fn estimate_memory_pressure(&self, active_workers: i64) -> f64 {
        // Simplified memory estimation
        // In a real implementation, this would query actual memory usage
        active_workers as f64 * 50.0 // Assume 50MB per worker baseline
    }

    fn calculate_avg_queue_wait_time(&self, _queue_metrics: &[QueueMetrics]) -> f64 {
        // Simplified calculation
        // In a real implementation, this would track actual wait times
        0.0
    }

    fn calculate_throughput_trend(&self, _queue_metrics: &[QueueMetrics]) -> f64 {
        // Simplified trend calculation
        // In a real implementation, this would analyze historical throughput
        0.0
    }

    pub async fn get_scaling_recommendations(&self) -> Result<String, TaskQueueError> {
        let metrics = self.collect_metrics().await?;
        let mut scaling_decision_copy = self.clone();
        let action = scaling_decision_copy.decide_scaling_action(&metrics)?;

        let mut report = format!(
            "Enhanced Auto-scaling Status Report:\n\
             - Active Workers: {}\n\
             - Total Pending Tasks: {}\n\
             - Queue Pressure Score: {:.2}\n\
             - Worker Utilization: {:.1}%\n\
             - Task Complexity Factor: {:.2}\n\
             - Error Rate: {:.1}%\n\
             - Memory Pressure: {:.1} MB\n\n",
            metrics.active_workers,
            metrics.total_pending_tasks,
            metrics.queue_pressure_score,
            metrics.worker_utilization * 100.0,
            metrics.task_complexity_factor,
            metrics.error_rate * 100.0,
            metrics.memory_pressure_mb
        );

        for queue_metric in &metrics.queue_metrics {
            report.push_str(&format!(
                "Queue '{}': {} pending, {} processed, {} failed\n",
                queue_metric.queue_name,
                queue_metric.pending_tasks,
                queue_metric.processed_tasks,
                queue_metric.failed_tasks
            ));
        }

        report.push_str(&format!("\nRecommended Action: {:?}\n", action));

        if let Some(ref adaptive) = self.adaptive_controller {
            report.push_str(&format!(
                "\nAdaptive Thresholds Enabled: {}\n",
                adaptive.get_adaptation_status()
            ));
        }

        Ok(report)
    }
}

// Clone implementation for AutoScaler (needed for decision making without borrowing issues)
impl Clone for AutoScaler {
    fn clone(&self) -> Self {
        let adaptive_controller = self.adaptive_controller.clone();
        let hysteresis_controller = self.hysteresis_controller.clone();

        Self {
            broker: self.broker.clone(),
            config: self.config.clone(),
            adaptive_controller,
            hysteresis_controller,
            metrics_history: self.metrics_history.clone(),
            start_time: self.start_time,
        }
    }
}

// Clone implementation for AutoScalerMetrics
impl Clone for AutoScalerMetrics {
    fn clone(&self) -> Self {
        Self {
            active_workers: self.active_workers,
            total_pending_tasks: self.total_pending_tasks,
            queue_metrics: self.queue_metrics.clone(),
            queue_pressure_score: self.queue_pressure_score,
            worker_utilization: self.worker_utilization,
            task_complexity_factor: self.task_complexity_factor,
            error_rate: self.error_rate,
            memory_pressure_mb: self.memory_pressure_mb,
            avg_queue_wait_time_ms: self.avg_queue_wait_time_ms,
            throughput_trend: self.throughput_trend,
        }
    }
}

impl AdaptiveThresholdController {
    pub fn new(
        initial_thresholds: ScalingTriggers,
        learning_rate: f64,
        target_sla: SLATargets,
        adaptation_window: Duration,
    ) -> Self {
        Self {
            current_thresholds: initial_thresholds,
            performance_history: Vec::new(),
            last_adaptation: Instant::now(),
            learning_rate,
            target_sla,
            adaptation_window,
        }
    }

    pub fn get_current_thresholds(&self) -> &ScalingTriggers {
        &self.current_thresholds
    }

    pub fn record_performance_snapshot(
        &mut self,
        metrics: &AutoScalerMetrics,
        action: &ScalingAction,
    ) {
        let snapshot = PerformanceSnapshot {
            timestamp: Instant::now(),
            latency_p95_ms: 0.0, // Would be filled from actual metrics
            success_rate: 1.0 - metrics.error_rate,
            queue_wait_time_ms: metrics.avg_queue_wait_time_ms,
            worker_utilization: metrics.worker_utilization,
            scaling_action_taken: match action {
                ScalingAction::NoAction => None,
                _ => Some(action.clone()),
            },
        };

        self.performance_history.push(snapshot);

        // Keep only recent history
        let cutoff_time = Instant::now() - self.adaptation_window;
        self.performance_history
            .retain(|s| s.timestamp > cutoff_time);

        // Adapt thresholds if enough time has passed
        if self.last_adaptation.elapsed() > Duration::from_secs(300) {
            self.adapt_thresholds();
            self.last_adaptation = Instant::now();
        }
    }

    fn adapt_thresholds(&mut self) {
        if self.performance_history.is_empty() {
            return;
        }

        let recent_performance =
            &self.performance_history[self.performance_history.len().saturating_sub(10)..];

        let avg_success_rate = recent_performance
            .iter()
            .map(|s| s.success_rate)
            .sum::<f64>()
            / recent_performance.len() as f64;

        let avg_utilization = recent_performance
            .iter()
            .map(|s| s.worker_utilization)
            .sum::<f64>()
            / recent_performance.len() as f64;

        // Adapt queue pressure threshold based on success rate
        if avg_success_rate < self.target_sla.min_success_rate {
            // Lower threshold to scale up more aggressively
            self.current_thresholds.queue_pressure_threshold *= 1.0 - self.learning_rate;
        } else if avg_success_rate > self.target_sla.min_success_rate + 0.02 {
            // Raise threshold to scale up less aggressively
            self.current_thresholds.queue_pressure_threshold *= 1.0 + (self.learning_rate * 0.5);
        }

        // Adapt utilization threshold based on target utilization
        let utilization_diff = avg_utilization - self.target_sla.target_worker_utilization;
        if utilization_diff.abs() > 0.1 {
            let adjustment = -utilization_diff * self.learning_rate;
            self.current_thresholds.worker_utilization_threshold *= 1.0 + adjustment;
        }

        // Clamp thresholds to reasonable ranges
        self.current_thresholds.queue_pressure_threshold = self
            .current_thresholds
            .queue_pressure_threshold
            .clamp(0.1, 2.0);
        self.current_thresholds.worker_utilization_threshold = self
            .current_thresholds
            .worker_utilization_threshold
            .clamp(0.1, 0.95);
    }

    pub fn get_adaptation_status(&self) -> String {
        format!(
            "Thresholds adapted {} times, {} performance samples",
            0,
            self.performance_history.len()
        )
    }
}

impl Clone for AdaptiveThresholdController {
    fn clone(&self) -> Self {
        Self {
            current_thresholds: self.current_thresholds.clone(),
            performance_history: self.performance_history.clone(),
            last_adaptation: self.last_adaptation,
            learning_rate: self.learning_rate,
            target_sla: self.target_sla.clone(),
            adaptation_window: self.adaptation_window,
        }
    }
}

impl HysteresisController {
    pub fn new(
        consecutive_signals_required: usize,
        scale_up_cooldown: Duration,
        scale_down_cooldown: Duration,
    ) -> Self {
        Self {
            scale_up_consecutive_signals: 0,
            scale_down_consecutive_signals: 0,
            required_consecutive_signals: consecutive_signals_required,
            last_scaling_action: None,
            scale_up_cooldown,
            scale_down_cooldown,
        }
    }

    pub fn should_execute_scaling(&mut self, proposed_action: &ScalingAction) -> bool {
        // Check cooldown periods
        if let Some((last_time, ref last_action)) = self.last_scaling_action {
            let cooldown_period = match last_action {
                ScalingAction::ScaleUp(_) => self.scale_up_cooldown,
                ScalingAction::ScaleDown(_) => self.scale_down_cooldown,
                ScalingAction::NoAction => Duration::from_secs(0),
            };

            if last_time.elapsed() < cooldown_period {
                return false;
            }
        }

        // Check consecutive signals
        let should_execute = match proposed_action {
            ScalingAction::ScaleUp(_) => {
                self.scale_up_consecutive_signals += 1;
                self.scale_down_consecutive_signals = 0;
                self.scale_up_consecutive_signals >= self.required_consecutive_signals
            }
            ScalingAction::ScaleDown(_) => {
                self.scale_down_consecutive_signals += 1;
                self.scale_up_consecutive_signals = 0;
                self.scale_down_consecutive_signals >= self.required_consecutive_signals
            }
            ScalingAction::NoAction => {
                self.scale_up_consecutive_signals = 0;
                self.scale_down_consecutive_signals = 0;
                false
            }
        };

        if should_execute {
            self.last_scaling_action = Some((Instant::now(), proposed_action.clone()));
            self.scale_up_consecutive_signals = 0;
            self.scale_down_consecutive_signals = 0;
        }

        should_execute
    }
}

impl Clone for HysteresisController {
    fn clone(&self) -> Self {
        Self {
            scale_up_consecutive_signals: self.scale_up_consecutive_signals,
            scale_down_consecutive_signals: self.scale_down_consecutive_signals,
            required_consecutive_signals: self.required_consecutive_signals,
            last_scaling_action: self.last_scaling_action.clone(),
            scale_up_cooldown: self.scale_up_cooldown,
            scale_down_cooldown: self.scale_down_cooldown,
        }
    }
}

impl Clone for ScalingAction {
    fn clone(&self) -> Self {
        match self {
            ScalingAction::ScaleUp(count) => ScalingAction::ScaleUp(*count),
            ScalingAction::ScaleDown(count) => ScalingAction::ScaleDown(*count),
            ScalingAction::NoAction => ScalingAction::NoAction,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::QueueMetrics;

    #[test]
    fn test_autoscaler_config_default() {
        let config = AutoScalerConfig::default();

        assert_eq!(config.min_workers, 1);
        assert_eq!(config.max_workers, 20);
        assert_eq!(config.scale_up_count, 2);
        assert_eq!(config.scale_down_count, 1);
        assert_eq!(config.scaling_triggers.queue_pressure_threshold, 0.75);
        assert_eq!(config.scaling_triggers.worker_utilization_threshold, 0.80);
        assert_eq!(config.scaling_triggers.task_complexity_threshold, 1.5);
        assert_eq!(config.scaling_triggers.error_rate_threshold, 0.05);
        assert_eq!(config.scaling_triggers.memory_pressure_threshold, 512.0);
        assert!(config.enable_adaptive_thresholds);
        assert_eq!(config.learning_rate, 0.1);
        assert_eq!(config.adaptation_window_minutes, 30);
        assert_eq!(config.scale_up_cooldown_seconds, 60);
        assert_eq!(config.scale_down_cooldown_seconds, 300);
        assert_eq!(config.consecutive_signals_required, 2);
        assert_eq!(config.target_sla.max_p95_latency_ms, 5000.0);
        assert_eq!(config.target_sla.min_success_rate, 0.95);
        assert_eq!(config.target_sla.max_queue_wait_time_ms, 10000.0);
        assert_eq!(config.target_sla.target_worker_utilization, 0.70);
    }

    #[test]
    fn test_autoscaler_config_validation_valid() {
        let config = AutoScalerConfig {
            min_workers: 2,
            max_workers: 10,
            scale_up_count: 3,
            scale_down_count: 2,
            scaling_triggers: ScalingTriggers {
                queue_pressure_threshold: 0.75,
                worker_utilization_threshold: 0.80,
                task_complexity_threshold: 1.5,
                error_rate_threshold: 0.05,
                memory_pressure_threshold: 512.0,
            },
            enable_adaptive_thresholds: true,
            learning_rate: 0.1,
            adaptation_window_minutes: 30,
            scale_up_cooldown_seconds: 60,
            scale_down_cooldown_seconds: 300,
            consecutive_signals_required: 2,
            target_sla: SLATargets {
                max_p95_latency_ms: 5000.0,
                min_success_rate: 0.95,
                max_queue_wait_time_ms: 10000.0,
                target_worker_utilization: 0.70,
            },
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_autoscaler_config_validation_min_workers_zero() {
        let config = AutoScalerConfig {
            min_workers: 0,
            max_workers: 10,
            scale_up_count: 2,
            scale_down_count: 1,
            scaling_triggers: ScalingTriggers {
                queue_pressure_threshold: 0.75,
                worker_utilization_threshold: 0.80,
                task_complexity_threshold: 1.5,
                error_rate_threshold: 0.05,
                memory_pressure_threshold: 512.0,
            },
            enable_adaptive_thresholds: true,
            learning_rate: 0.1,
            adaptation_window_minutes: 30,
            scale_up_cooldown_seconds: 60,
            scale_down_cooldown_seconds: 300,
            consecutive_signals_required: 2,
            target_sla: SLATargets {
                max_p95_latency_ms: 5000.0,
                min_success_rate: 0.95,
                max_queue_wait_time_ms: 10000.0,
                target_worker_utilization: 0.70,
            },
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Minimum workers must be greater than 0"));
    }

    #[test]
    fn test_autoscaler_config_validation_max_less_than_min() {
        let config = AutoScalerConfig {
            min_workers: 10,
            max_workers: 5,
            scale_up_count: 2,
            scale_down_count: 1,
            scaling_triggers: ScalingTriggers {
                queue_pressure_threshold: 0.75,
                worker_utilization_threshold: 0.80,
                task_complexity_threshold: 1.5,
                error_rate_threshold: 0.05,
                memory_pressure_threshold: 512.0,
            },
            enable_adaptive_thresholds: true,
            learning_rate: 0.1,
            adaptation_window_minutes: 30,
            scale_up_cooldown_seconds: 60,
            scale_down_cooldown_seconds: 300,
            consecutive_signals_required: 2,
            target_sla: SLATargets {
                max_p95_latency_ms: 5000.0,
                min_success_rate: 0.95,
                max_queue_wait_time_ms: 10000.0,
                target_worker_utilization: 0.70,
            },
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Maximum workers must be greater than or equal to minimum workers"));
    }

    #[test]
    fn test_autoscaler_config_validation_max_workers_too_high() {
        let config = AutoScalerConfig {
            min_workers: 1,
            max_workers: 1001,
            scale_up_count: 2,
            scale_down_count: 1,
            scaling_triggers: ScalingTriggers {
                queue_pressure_threshold: 0.75,
                worker_utilization_threshold: 0.80,
                task_complexity_threshold: 1.5,
                error_rate_threshold: 0.05,
                memory_pressure_threshold: 512.0,
            },
            enable_adaptive_thresholds: true,
            learning_rate: 0.1,
            adaptation_window_minutes: 30,
            scale_up_cooldown_seconds: 60,
            scale_down_cooldown_seconds: 300,
            consecutive_signals_required: 2,
            target_sla: SLATargets {
                max_p95_latency_ms: 5000.0,
                min_success_rate: 0.95,
                max_queue_wait_time_ms: 10000.0,
                target_worker_utilization: 0.70,
            },
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Maximum workers cannot exceed 1000"));
    }

    #[test]
    fn test_autoscaler_config_validation_invalid_scale_up_count() {
        let config = AutoScalerConfig {
            min_workers: 1,
            max_workers: 10,
            scale_up_count: 0,
            scale_down_count: 1,
            scaling_triggers: ScalingTriggers {
                queue_pressure_threshold: 0.75,
                worker_utilization_threshold: 0.80,
                task_complexity_threshold: 1.5,
                error_rate_threshold: 0.05,
                memory_pressure_threshold: 512.0,
            },
            enable_adaptive_thresholds: true,
            learning_rate: 0.1,
            adaptation_window_minutes: 30,
            scale_up_cooldown_seconds: 60,
            scale_down_cooldown_seconds: 300,
            consecutive_signals_required: 2,
            target_sla: SLATargets {
                max_p95_latency_ms: 5000.0,
                min_success_rate: 0.95,
                max_queue_wait_time_ms: 10000.0,
                target_worker_utilization: 0.70,
            },
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Scale up count must be between 1 and 50"));
    }

    #[test]
    fn test_autoscaler_config_validation_invalid_scale_down_count() {
        let config = AutoScalerConfig {
            min_workers: 1,
            max_workers: 10,
            scale_up_count: 2,
            scale_down_count: 0,
            scaling_triggers: ScalingTriggers {
                queue_pressure_threshold: 0.75,
                worker_utilization_threshold: 0.80,
                task_complexity_threshold: 1.5,
                error_rate_threshold: 0.05,
                memory_pressure_threshold: 512.0,
            },
            enable_adaptive_thresholds: true,
            learning_rate: 0.1,
            adaptation_window_minutes: 30,
            scale_up_cooldown_seconds: 60,
            scale_down_cooldown_seconds: 300,
            consecutive_signals_required: 2,
            target_sla: SLATargets {
                max_p95_latency_ms: 5000.0,
                min_success_rate: 0.95,
                max_queue_wait_time_ms: 10000.0,
                target_worker_utilization: 0.70,
            },
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Scale down count must be between 1 and 50"));
    }

    #[test]
    fn test_autoscaler_config_validation_invalid_scaling_triggers() {
        let config = AutoScalerConfig {
            min_workers: 1,
            max_workers: 10,
            scale_up_count: 2,
            scale_down_count: 1,
            scaling_triggers: ScalingTriggers {
                queue_pressure_threshold: 0.0,
                worker_utilization_threshold: 0.0,
                task_complexity_threshold: 0.0,
                error_rate_threshold: 0.0,
                memory_pressure_threshold: 0.0,
            },
            enable_adaptive_thresholds: true,
            learning_rate: 0.1,
            adaptation_window_minutes: 30,
            scale_up_cooldown_seconds: 60,
            scale_down_cooldown_seconds: 300,
            consecutive_signals_required: 2,
            target_sla: SLATargets {
                max_p95_latency_ms: 5000.0,
                min_success_rate: 0.95,
                max_queue_wait_time_ms: 10000.0,
                target_worker_utilization: 0.70,
            },
        };

        let result = config.validate();
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Queue pressure threshold must be between 0.1 and 2.0"));
    }

    #[test]
    fn test_autoscaler_config_validation_invalid_learning_rate() {
        let config = AutoScalerConfig {
            min_workers: 1,
            max_workers: 10,
            scale_up_count: 2,
            scale_down_count: 1,
            scaling_triggers: ScalingTriggers {
                queue_pressure_threshold: 0.75,
                worker_utilization_threshold: 0.80,
                task_complexity_threshold: 1.5,
                error_rate_threshold: 0.05,
                memory_pressure_threshold: 512.0,
            },
            enable_adaptive_thresholds: true,
            learning_rate: 0.0,
            adaptation_window_minutes: 30,
            scale_up_cooldown_seconds: 60,
            scale_down_cooldown_seconds: 300,
            consecutive_signals_required: 2,
            target_sla: SLATargets {
                max_p95_latency_ms: 5000.0,
                min_success_rate: 0.95,
                max_queue_wait_time_ms: 10000.0,
                target_worker_utilization: 0.70,
            },
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Learning rate must be between 0.01 and 1.0"));
    }

    #[test]
    fn test_autoscaler_config_serialization() {
        let config = AutoScalerConfig::default();

        // Test JSON serialization
        let json = serde_json::to_string(&config).expect("Failed to serialize to JSON");
        let deserialized: AutoScalerConfig =
            serde_json::from_str(&json).expect("Failed to deserialize from JSON");

        assert_eq!(config.min_workers, deserialized.min_workers);
        assert_eq!(config.max_workers, deserialized.max_workers);
        assert_eq!(config.scale_up_count, deserialized.scale_up_count);
        assert_eq!(config.scale_down_count, deserialized.scale_down_count);
        assert_eq!(
            config.scaling_triggers.queue_pressure_threshold,
            deserialized.scaling_triggers.queue_pressure_threshold
        );
        assert_eq!(
            config.scaling_triggers.worker_utilization_threshold,
            deserialized.scaling_triggers.worker_utilization_threshold
        );
        assert_eq!(
            config.scaling_triggers.task_complexity_threshold,
            deserialized.scaling_triggers.task_complexity_threshold
        );
        assert_eq!(
            config.scaling_triggers.error_rate_threshold,
            deserialized.scaling_triggers.error_rate_threshold
        );
        assert_eq!(
            config.scaling_triggers.memory_pressure_threshold,
            deserialized.scaling_triggers.memory_pressure_threshold
        );
        assert_eq!(
            config.enable_adaptive_thresholds,
            deserialized.enable_adaptive_thresholds
        );
        assert_eq!(config.learning_rate, deserialized.learning_rate);
        assert_eq!(
            config.adaptation_window_minutes,
            deserialized.adaptation_window_minutes
        );
        assert_eq!(
            config.scale_up_cooldown_seconds,
            deserialized.scale_up_cooldown_seconds
        );
        assert_eq!(
            config.scale_down_cooldown_seconds,
            deserialized.scale_down_cooldown_seconds
        );
        assert_eq!(
            config.consecutive_signals_required,
            deserialized.consecutive_signals_required
        );
        assert_eq!(
            config.target_sla.max_p95_latency_ms,
            deserialized.target_sla.max_p95_latency_ms
        );
        assert_eq!(
            config.target_sla.min_success_rate,
            deserialized.target_sla.min_success_rate
        );
        assert_eq!(
            config.target_sla.max_queue_wait_time_ms,
            deserialized.target_sla.max_queue_wait_time_ms
        );
        assert_eq!(
            config.target_sla.target_worker_utilization,
            deserialized.target_sla.target_worker_utilization
        );
    }

    fn create_test_autoscaler_metrics(
        active_workers: i64,
        total_pending_tasks: i64,
    ) -> AutoScalerMetrics {
        let queue_pressure_score = if active_workers > 0 {
            total_pending_tasks as f64 / active_workers as f64
        } else {
            total_pending_tasks as f64
        };

        AutoScalerMetrics {
            active_workers,
            total_pending_tasks,
            queue_metrics: vec![QueueMetrics {
                queue_name: "default".to_string(),
                pending_tasks: total_pending_tasks,
                processed_tasks: 100,
                failed_tasks: 5,
            }],
            queue_pressure_score,
            worker_utilization: if active_workers > 0 { 0.6 } else { 0.0 },
            task_complexity_factor: 1.0,
            error_rate: 5.0 / 105.0, // 5 failed out of 105 total
            memory_pressure_mb: active_workers as f64 * 50.0,
            avg_queue_wait_time_ms: 0.0,
            throughput_trend: 0.0,
        }
    }

    fn create_test_autoscaler_metrics_with_high_pressure(
        active_workers: i64,
        total_pending_tasks: i64,
    ) -> AutoScalerMetrics {
        let queue_pressure_score = if active_workers > 0 {
            total_pending_tasks as f64 / active_workers as f64
        } else {
            total_pending_tasks as f64
        };

        AutoScalerMetrics {
            active_workers,
            total_pending_tasks,
            queue_metrics: vec![QueueMetrics {
                queue_name: "default".to_string(),
                pending_tasks: total_pending_tasks,
                processed_tasks: 100,
                failed_tasks: 5,
            }],
            queue_pressure_score,
            worker_utilization: 0.95,    // High utilization
            task_complexity_factor: 2.0, // High complexity
            error_rate: 0.08,            // High error rate
            memory_pressure_mb: 600.0,   // High memory pressure
            avg_queue_wait_time_ms: 0.0,
            throughput_trend: 0.0,
        }
    }

    fn create_test_autoscaler_metrics_with_low_pressure(
        active_workers: i64,
        total_pending_tasks: i64,
    ) -> AutoScalerMetrics {
        let queue_pressure_score = if active_workers > 0 {
            total_pending_tasks as f64 / active_workers as f64
        } else {
            total_pending_tasks as f64
        };

        AutoScalerMetrics {
            active_workers,
            total_pending_tasks,
            queue_metrics: vec![QueueMetrics {
                queue_name: "default".to_string(),
                pending_tasks: total_pending_tasks,
                processed_tasks: 100,
                failed_tasks: 5,
            }],
            queue_pressure_score,
            worker_utilization: 0.2,     // Low utilization
            task_complexity_factor: 0.5, // Low complexity
            error_rate: 0.01,            // Low error rate
            memory_pressure_mb: 100.0,   // Low memory pressure
            avg_queue_wait_time_ms: 0.0,
            throughput_trend: 0.0,
        }
    }

    #[allow(dead_code)]
    fn create_mock_autoscaler() -> AutoScaler {
        // Create a dummy broker for testing purposes
        let redis_url = "redis://localhost:6379";
        let mut config = deadpool_redis::Config::from_url(redis_url);
        config.pool = Some(deadpool_redis::PoolConfig::new(1));

        let pool = config
            .create_pool(Some(deadpool_redis::Runtime::Tokio1))
            .expect("Failed to create pool");

        let broker = Arc::new(crate::broker::RedisBroker { pool });
        AutoScaler::new(broker)
    }

    #[test]
    fn test_scaling_action_scale_up() {
        let config = AutoScalerConfig {
            min_workers: 1,
            max_workers: 10,
            scale_up_count: 2,
            scale_down_count: 1,
            scaling_triggers: ScalingTriggers {
                queue_pressure_threshold: 0.75,
                worker_utilization_threshold: 0.80,
                task_complexity_threshold: 1.5,
                error_rate_threshold: 0.05,
                memory_pressure_threshold: 512.0,
            },
            enable_adaptive_thresholds: true,
            learning_rate: 0.1,
            adaptation_window_minutes: 30,
            scale_up_cooldown_seconds: 60,
            scale_down_cooldown_seconds: 300,
            consecutive_signals_required: 2,
            target_sla: SLATargets {
                max_p95_latency_ms: 5000.0,
                min_success_rate: 0.95,
                max_queue_wait_time_ms: 10000.0,
                target_worker_utilization: 0.70,
            },
        };

        let broker = Arc::new(crate::broker::RedisBroker {
            pool: deadpool_redis::Config::from_url("redis://localhost:6379")
                .create_pool(Some(deadpool_redis::Runtime::Tokio1))
                .expect("Failed to create pool"),
        });

        let mut autoscaler = AutoScaler::with_config(broker, config);

        // 2 workers, high pressure conditions that should trigger scale up
        let metrics = create_test_autoscaler_metrics_with_high_pressure(2, 12);

        // Run scaling decision multiple times to satisfy consecutive signals requirement
        let action1 = autoscaler
            .decide_scaling_action(&metrics)
            .expect("Failed to decide scaling action");
        let action2 = autoscaler
            .decide_scaling_action(&metrics)
            .expect("Failed to decide scaling action");

        // The second action should trigger scaling due to consecutive signals
        match action2 {
            ScalingAction::ScaleUp(count) => assert_eq!(count, 2),
            _ => panic!(
                "Expected ScaleUp action, got {:?}. First action was {:?}",
                action2, action1
            ),
        }
    }

    #[test]
    fn test_scaling_action_scale_down() {
        let config = AutoScalerConfig {
            min_workers: 1,
            max_workers: 10,
            scale_up_count: 2,
            scale_down_count: 1,
            scaling_triggers: ScalingTriggers {
                queue_pressure_threshold: 0.75,
                worker_utilization_threshold: 0.80,
                task_complexity_threshold: 1.5,
                error_rate_threshold: 0.05,
                memory_pressure_threshold: 512.0,
            },
            enable_adaptive_thresholds: true,
            learning_rate: 0.1,
            adaptation_window_minutes: 30,
            scale_up_cooldown_seconds: 60,
            scale_down_cooldown_seconds: 300,
            consecutive_signals_required: 2,
            target_sla: SLATargets {
                max_p95_latency_ms: 5000.0,
                min_success_rate: 0.95,
                max_queue_wait_time_ms: 10000.0,
                target_worker_utilization: 0.70,
            },
        };

        let broker = Arc::new(crate::broker::RedisBroker {
            pool: deadpool_redis::Config::from_url("redis://localhost:6379")
                .create_pool(Some(deadpool_redis::Runtime::Tokio1))
                .expect("Failed to create pool"),
        });

        let mut autoscaler = AutoScaler::with_config(broker, config);

        // 5 workers, low pressure conditions that should trigger scale down
        let metrics = create_test_autoscaler_metrics_with_low_pressure(5, 2);

        // Run scaling decision multiple times to satisfy consecutive signals requirement
        let action1 = autoscaler
            .decide_scaling_action(&metrics)
            .expect("Failed to decide scaling action");
        let action2 = autoscaler
            .decide_scaling_action(&metrics)
            .expect("Failed to decide scaling action");

        // The second action should trigger scaling due to consecutive signals
        match action2 {
            ScalingAction::ScaleDown(count) => assert_eq!(count, 1),
            _ => panic!(
                "Expected ScaleDown action, got {:?}. First action was {:?}",
                action2, action1
            ),
        }
    }

    #[test]
    fn test_scaling_action_no_action() {
        let config = AutoScalerConfig::default();

        let broker = Arc::new(crate::broker::RedisBroker {
            pool: deadpool_redis::Config::from_url("redis://localhost:6379")
                .create_pool(Some(deadpool_redis::Runtime::Tokio1))
                .expect("Failed to create pool"),
        });

        let mut autoscaler = AutoScaler::with_config(broker, config);

        // 3 workers, moderate conditions that should not trigger scaling
        let metrics = create_test_autoscaler_metrics(3, 9);

        let action = autoscaler
            .decide_scaling_action(&metrics)
            .expect("Failed to decide scaling action");

        match action {
            ScalingAction::NoAction => {}
            _ => panic!("Expected NoAction, got {:?}", action),
        }
    }

    #[test]
    fn test_scaling_action_at_max_workers() {
        let config = AutoScalerConfig {
            min_workers: 1,
            max_workers: 5,
            scale_up_count: 3,
            scale_down_count: 1,
            scaling_triggers: ScalingTriggers {
                queue_pressure_threshold: 0.75,
                worker_utilization_threshold: 0.80,
                task_complexity_threshold: 1.5,
                error_rate_threshold: 0.05,
                memory_pressure_threshold: 512.0,
            },
            enable_adaptive_thresholds: true,
            learning_rate: 0.1,
            adaptation_window_minutes: 30,
            scale_up_cooldown_seconds: 60,
            scale_down_cooldown_seconds: 300,
            consecutive_signals_required: 2,
            target_sla: SLATargets {
                max_p95_latency_ms: 5000.0,
                min_success_rate: 0.95,
                max_queue_wait_time_ms: 10000.0,
                target_worker_utilization: 0.70,
            },
        };

        let broker = Arc::new(crate::broker::RedisBroker {
            pool: deadpool_redis::Config::from_url("redis://localhost:6379")
                .create_pool(Some(deadpool_redis::Runtime::Tokio1))
                .expect("Failed to create pool"),
        });

        let mut autoscaler = AutoScaler::with_config(broker, config);

        // 5 workers (at max), 20 tasks = 4 tasks per worker (> threshold but can't scale up)
        let metrics = create_test_autoscaler_metrics(5, 20);

        let action = autoscaler
            .decide_scaling_action(&metrics)
            .expect("Failed to decide scaling action");

        match action {
            ScalingAction::NoAction => {}
            _ => panic!("Expected NoAction when at max workers"),
        }
    }

    #[test]
    fn test_scaling_action_at_min_workers() {
        let config = AutoScalerConfig {
            min_workers: 3,
            max_workers: 10,
            scale_up_count: 2,
            scale_down_count: 2,
            scaling_triggers: ScalingTriggers {
                queue_pressure_threshold: 0.75,
                worker_utilization_threshold: 0.80,
                task_complexity_threshold: 1.5,
                error_rate_threshold: 0.05,
                memory_pressure_threshold: 512.0,
            },
            enable_adaptive_thresholds: true,
            learning_rate: 0.1,
            adaptation_window_minutes: 30,
            scale_up_cooldown_seconds: 60,
            scale_down_cooldown_seconds: 300,
            consecutive_signals_required: 2,
            target_sla: SLATargets {
                max_p95_latency_ms: 5000.0,
                min_success_rate: 0.95,
                max_queue_wait_time_ms: 10000.0,
                target_worker_utilization: 0.70,
            },
        };

        let broker = Arc::new(crate::broker::RedisBroker {
            pool: deadpool_redis::Config::from_url("redis://localhost:6379")
                .create_pool(Some(deadpool_redis::Runtime::Tokio1))
                .expect("Failed to create pool"),
        });

        let mut autoscaler = AutoScaler::with_config(broker, config);

        // 3 workers (at min), 1 task = 0.33 tasks per worker (< threshold but can't scale down)
        let metrics = create_test_autoscaler_metrics(3, 1);

        let action = autoscaler
            .decide_scaling_action(&metrics)
            .expect("Failed to decide scaling action");

        match action {
            ScalingAction::NoAction => {}
            _ => panic!("Expected NoAction when at min workers"),
        }
    }

    #[test]
    fn test_scaling_action_limited_by_max_workers() {
        let config = AutoScalerConfig {
            min_workers: 1,
            max_workers: 5,
            scale_up_count: 10,
            scale_down_count: 1,
            scaling_triggers: ScalingTriggers {
                queue_pressure_threshold: 0.75,
                worker_utilization_threshold: 0.80,
                task_complexity_threshold: 1.5,
                error_rate_threshold: 0.05,
                memory_pressure_threshold: 512.0,
            },
            enable_adaptive_thresholds: true,
            learning_rate: 0.1,
            adaptation_window_minutes: 30,
            scale_up_cooldown_seconds: 60,
            scale_down_cooldown_seconds: 300,
            consecutive_signals_required: 2,
            target_sla: SLATargets {
                max_p95_latency_ms: 5000.0,
                min_success_rate: 0.95,
                max_queue_wait_time_ms: 10000.0,
                target_worker_utilization: 0.70,
            },
        };

        let broker = Arc::new(crate::broker::RedisBroker {
            pool: deadpool_redis::Config::from_url("redis://localhost:6379")
                .create_pool(Some(deadpool_redis::Runtime::Tokio1))
                .expect("Failed to create pool"),
        });

        let mut autoscaler = AutoScaler::with_config(broker, config);

        // 3 workers, high pressure conditions that should trigger scale up but limited by max
        let metrics = create_test_autoscaler_metrics_with_high_pressure(3, 15);

        // Run scaling decision multiple times to satisfy consecutive signals requirement
        let action1 = autoscaler
            .decide_scaling_action(&metrics)
            .expect("Failed to decide scaling action");
        let action2 = autoscaler
            .decide_scaling_action(&metrics)
            .expect("Failed to decide scaling action");

        // The second action should trigger scaling due to consecutive signals
        match action2 {
            ScalingAction::ScaleUp(count) => assert_eq!(count, 2), // Limited by max_workers
            _ => panic!(
                "Expected ScaleUp action limited by max workers, got {:?}. First action was {:?}",
                action2, action1
            ),
        }
    }

    #[test]
    fn test_scaling_action_limited_by_min_workers() {
        let config = AutoScalerConfig {
            min_workers: 3,
            max_workers: 10,
            scale_up_count: 2,
            scale_down_count: 10,
            scaling_triggers: ScalingTriggers {
                queue_pressure_threshold: 0.75,
                worker_utilization_threshold: 0.80,
                task_complexity_threshold: 1.5,
                error_rate_threshold: 0.05,
                memory_pressure_threshold: 512.0,
            },
            enable_adaptive_thresholds: true,
            learning_rate: 0.1,
            adaptation_window_minutes: 30,
            scale_up_cooldown_seconds: 60,
            scale_down_cooldown_seconds: 300,
            consecutive_signals_required: 2,
            target_sla: SLATargets {
                max_p95_latency_ms: 5000.0,
                min_success_rate: 0.95,
                max_queue_wait_time_ms: 10000.0,
                target_worker_utilization: 0.70,
            },
        };

        let broker = Arc::new(crate::broker::RedisBroker {
            pool: deadpool_redis::Config::from_url("redis://localhost:6379")
                .create_pool(Some(deadpool_redis::Runtime::Tokio1))
                .expect("Failed to create pool"),
        });

        let mut autoscaler = AutoScaler::with_config(broker, config);

        // 6 workers, low pressure conditions that should trigger scale down but limited by min
        let metrics = create_test_autoscaler_metrics_with_low_pressure(6, 3);

        // Run scaling decision multiple times to satisfy consecutive signals requirement
        let action1 = autoscaler
            .decide_scaling_action(&metrics)
            .expect("Failed to decide scaling action");
        let action2 = autoscaler
            .decide_scaling_action(&metrics)
            .expect("Failed to decide scaling action");

        // The second action should trigger scaling due to consecutive signals
        match action2 {
            ScalingAction::ScaleDown(count) => assert_eq!(count, 3), // Limited by min_workers
            _ => panic!(
                "Expected ScaleDown action limited by min workers, got {:?}. First action was {:?}",
                action2, action1
            ),
        }
    }

    #[test]
    fn test_scaling_action_zero_workers() {
        let config = AutoScalerConfig::default();

        let broker = Arc::new(crate::broker::RedisBroker {
            pool: deadpool_redis::Config::from_url("redis://localhost:6379")
                .create_pool(Some(deadpool_redis::Runtime::Tokio1))
                .expect("Failed to create pool"),
        });

        let mut autoscaler = AutoScaler::with_config(broker, config);

        // 0 workers, high pressure conditions (should definitely scale up)
        let metrics = create_test_autoscaler_metrics_with_high_pressure(0, 10);

        // Run scaling decision multiple times to satisfy consecutive signals requirement
        let action1 = autoscaler
            .decide_scaling_action(&metrics)
            .expect("Failed to decide scaling action");
        let action2 = autoscaler
            .decide_scaling_action(&metrics)
            .expect("Failed to decide scaling action");

        // The second action should trigger scaling due to consecutive signals
        match action2 {
            ScalingAction::ScaleUp(count) => assert_eq!(count, 2),
            _ => panic!(
                "Expected ScaleUp action with zero workers, got {:?}. First action was {:?}",
                action2, action1
            ),
        }
    }

    #[test]
    fn test_autoscaler_metrics_debug() {
        let metrics = create_test_autoscaler_metrics(5, 25);
        let debug_str = format!("{:?}", metrics);

        assert!(debug_str.contains("AutoScalerMetrics"));
        assert!(debug_str.contains("active_workers: 5"));
        assert!(debug_str.contains("total_pending_tasks: 25"));
        assert!(debug_str.contains("queue_pressure_score: 5"));
        assert!(debug_str.contains("worker_utilization: 0.6"));
        assert!(debug_str.contains("task_complexity_factor: 1"));
        assert!(debug_str.contains("error_rate:"));
        assert!(debug_str.contains("memory_pressure_mb: 250"));
        assert!(debug_str.contains("avg_queue_wait_time_ms: 0"));
        assert!(debug_str.contains("throughput_trend: 0"));
    }

    #[test]
    fn test_scaling_action_debug() {
        let scale_up = ScalingAction::ScaleUp(3);
        let scale_down = ScalingAction::ScaleDown(2);
        let no_action = ScalingAction::NoAction;

        assert!(format!("{:?}", scale_up).contains("ScaleUp(3)"));
        assert!(format!("{:?}", scale_down).contains("ScaleDown(2)"));
        assert!(format!("{:?}", no_action).contains("NoAction"));
    }

    #[test]
    fn test_autoscaler_config_clone() {
        let original = AutoScalerConfig::default();
        let cloned = original.clone();

        assert_eq!(original.min_workers, cloned.min_workers);
        assert_eq!(original.max_workers, cloned.max_workers);
        assert_eq!(original.scale_up_count, cloned.scale_up_count);
        assert_eq!(original.scale_down_count, cloned.scale_down_count);
        assert_eq!(
            original.scaling_triggers.queue_pressure_threshold,
            cloned.scaling_triggers.queue_pressure_threshold
        );
        assert_eq!(
            original.scaling_triggers.worker_utilization_threshold,
            cloned.scaling_triggers.worker_utilization_threshold
        );
        assert_eq!(
            original.scaling_triggers.task_complexity_threshold,
            cloned.scaling_triggers.task_complexity_threshold
        );
        assert_eq!(
            original.scaling_triggers.error_rate_threshold,
            cloned.scaling_triggers.error_rate_threshold
        );
        assert_eq!(
            original.scaling_triggers.memory_pressure_threshold,
            cloned.scaling_triggers.memory_pressure_threshold
        );
        assert_eq!(
            original.enable_adaptive_thresholds,
            cloned.enable_adaptive_thresholds
        );
        assert_eq!(original.learning_rate, cloned.learning_rate);
        assert_eq!(
            original.adaptation_window_minutes,
            cloned.adaptation_window_minutes
        );
        assert_eq!(
            original.scale_up_cooldown_seconds,
            cloned.scale_up_cooldown_seconds
        );
        assert_eq!(
            original.scale_down_cooldown_seconds,
            cloned.scale_down_cooldown_seconds
        );
        assert_eq!(
            original.consecutive_signals_required,
            cloned.consecutive_signals_required
        );
        assert_eq!(
            original.target_sla.max_p95_latency_ms,
            cloned.target_sla.max_p95_latency_ms
        );
        assert_eq!(
            original.target_sla.min_success_rate,
            cloned.target_sla.min_success_rate
        );
        assert_eq!(
            original.target_sla.max_queue_wait_time_ms,
            cloned.target_sla.max_queue_wait_time_ms
        );
        assert_eq!(
            original.target_sla.target_worker_utilization,
            cloned.target_sla.target_worker_utilization
        );
    }
}
