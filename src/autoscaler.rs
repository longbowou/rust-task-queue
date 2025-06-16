use crate::{QueueMetrics, RedisBroker, TaskQueueError, queue::queue_names};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoScalerConfig {
    pub min_workers: usize,
    pub max_workers: usize,
    pub scale_up_threshold: f64,   // Tasks per worker ratio to scale up
    pub scale_down_threshold: f64, // Tasks per worker ratio to scale down
    pub scale_up_count: usize,     // Number of workers to add
    pub scale_down_count: usize,   // Number of workers to remove
}

impl AutoScalerConfig {
    /// Validate autoscaler configuration
    pub fn validate(&self) -> Result<(), TaskQueueError> {
        if self.min_workers == 0 {
            return Err(TaskQueueError::Configuration(
                "Minimum workers must be greater than 0".to_string()
            ));
        }

        if self.max_workers < self.min_workers {
            return Err(TaskQueueError::Configuration(
                "Maximum workers must be greater than or equal to minimum workers".to_string()
            ));
        }

        if self.max_workers > 1000 {
            return Err(TaskQueueError::Configuration(
                "Maximum workers cannot exceed 1000".to_string()
            ));
        }

        if self.scale_up_threshold <= 0.0 || self.scale_up_threshold > 100.0 {
            return Err(TaskQueueError::Configuration(
                "Scale up threshold must be between 0.1 and 100.0".to_string()
            ));
        }

        if self.scale_down_threshold < 0.0 || self.scale_down_threshold >= self.scale_up_threshold {
            return Err(TaskQueueError::Configuration(
                "Scale down threshold must be between 0.0 and scale up threshold".to_string()
            ));
        }

        if self.scale_up_count == 0 || self.scale_up_count > 50 {
            return Err(TaskQueueError::Configuration(
                "Scale up count must be between 1 and 50".to_string()
            ));
        }

        if self.scale_down_count == 0 || self.scale_down_count > 50 {
            return Err(TaskQueueError::Configuration(
                "Scale down count must be between 1 and 50".to_string()
            ));
        }

        Ok(())
    }
}

impl Default for AutoScalerConfig {
    fn default() -> Self {
        Self {
            min_workers: 1,
            max_workers: 20,
            scale_up_threshold: 5.0,
            scale_down_threshold: 1.0,
            scale_up_count: 2,
            scale_down_count: 1,
        }
    }
}

#[derive(Debug)]
pub enum ScalingAction {
    ScaleUp(usize),
    ScaleDown(usize),
    NoAction,
}

#[derive(Debug)]
pub struct AutoScalerMetrics {
    pub active_workers: i64,
    pub total_pending_tasks: i64,
    pub queue_metrics: Vec<QueueMetrics>,
    pub tasks_per_worker: f64,
}

pub struct AutoScaler {
    broker: Arc<RedisBroker>,
    config: AutoScalerConfig,
}

impl AutoScaler {
    pub fn new(broker: Arc<RedisBroker>) -> Self {
        Self {
            broker,
            config: AutoScalerConfig::default(),
        }
    }

    pub fn with_config(broker: Arc<RedisBroker>, config: AutoScalerConfig) -> Self {
        Self { broker, config }
    }

    pub async fn collect_metrics(&self) -> Result<AutoScalerMetrics, TaskQueueError> {
        let active_workers = self.broker.get_active_workers().await?;

        let queues = [queue_names::DEFAULT, queue_names::HIGH_PRIORITY, queue_names::LOW_PRIORITY];
        let mut queue_metrics = Vec::new();
        let mut total_pending_tasks = 0;

        for queue in &queues {
            let metrics = self.broker.get_queue_metrics(queue).await?;
            total_pending_tasks += metrics.pending_tasks;
            queue_metrics.push(metrics);
        }

        let tasks_per_worker = if active_workers > 0 {
            total_pending_tasks as f64 / active_workers as f64
        } else {
            total_pending_tasks as f64
        };

        Ok(AutoScalerMetrics {
            active_workers,
            total_pending_tasks,
            queue_metrics,
            tasks_per_worker,
        })
    }

    pub fn decide_scaling_action(
        &self,
        metrics: &AutoScalerMetrics,
    ) -> Result<ScalingAction, TaskQueueError> {
        let current_workers = metrics.active_workers as usize;

        // Scale up conditions
        if metrics.tasks_per_worker > self.config.scale_up_threshold
            && current_workers < self.config.max_workers
        {
            let scale_count = std::cmp::min(
                self.config.scale_up_count,
                self.config.max_workers - current_workers,
            );

            #[cfg(feature = "tracing")]
            tracing::info!(
                "Auto-scaling up: {} tasks per worker (threshold: {}), adding {} workers",
                metrics.tasks_per_worker,
                self.config.scale_up_threshold,
                scale_count
            );

            return Ok(ScalingAction::ScaleUp(scale_count));
        }

        // Scale down conditions
        if metrics.tasks_per_worker < self.config.scale_down_threshold
            && current_workers > self.config.min_workers
        {
            let scale_count = std::cmp::min(
                self.config.scale_down_count,
                current_workers - self.config.min_workers,
            );

            #[cfg(feature = "tracing")]
            tracing::info!(
                "Auto-scaling down: {} tasks per worker (threshold: {}), removing {} workers",
                metrics.tasks_per_worker,
                self.config.scale_down_threshold,
                scale_count
            );

            return Ok(ScalingAction::ScaleDown(scale_count));
        }

        Ok(ScalingAction::NoAction)
    }

    pub async fn get_scaling_recommendations(&self) -> Result<String, TaskQueueError> {
        let metrics = self.collect_metrics().await?;
        let action = self.decide_scaling_action(&metrics)?;

        let mut report = format!(
            "Auto-scaling Status Report:\n\
             - Active Workers: {}\n\
             - Total Pending Tasks: {}\n\
             - Tasks per Worker: {:.2}\n\
             - Scale Up Threshold: {:.2}\n\
             - Scale Down Threshold: {:.2}\n\n",
            metrics.active_workers,
            metrics.total_pending_tasks,
            metrics.tasks_per_worker,
            self.config.scale_up_threshold,
            self.config.scale_down_threshold
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

        Ok(report)
    }
}
