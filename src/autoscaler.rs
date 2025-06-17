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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::QueueMetrics;

    #[test]
    fn test_autoscaler_config_default() {
        let config = AutoScalerConfig::default();
        
        assert_eq!(config.min_workers, 1);
        assert_eq!(config.max_workers, 20);
        assert_eq!(config.scale_up_threshold, 5.0);
        assert_eq!(config.scale_down_threshold, 1.0);
        assert_eq!(config.scale_up_count, 2);
        assert_eq!(config.scale_down_count, 1);
    }

    #[test]
    fn test_autoscaler_config_validation_valid() {
        let config = AutoScalerConfig {
            min_workers: 2,
            max_workers: 10,
            scale_up_threshold: 5.0,
            scale_down_threshold: 1.0,
            scale_up_count: 3,
            scale_down_count: 2,
        };
        
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_autoscaler_config_validation_min_workers_zero() {
        let config = AutoScalerConfig {
            min_workers: 0,
            max_workers: 10,
            scale_up_threshold: 5.0,
            scale_down_threshold: 1.0,
            scale_up_count: 2,
            scale_down_count: 1,
        };
        
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Minimum workers must be greater than 0"));
    }

    #[test]
    fn test_autoscaler_config_validation_max_less_than_min() {
        let config = AutoScalerConfig {
            min_workers: 10,
            max_workers: 5,
            scale_up_threshold: 5.0,
            scale_down_threshold: 1.0,
            scale_up_count: 2,
            scale_down_count: 1,
        };
        
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Maximum workers must be greater than or equal to minimum workers"));
    }

    #[test]
    fn test_autoscaler_config_validation_max_workers_too_high() {
        let config = AutoScalerConfig {
            min_workers: 1,
            max_workers: 1001,
            scale_up_threshold: 5.0,
            scale_down_threshold: 1.0,
            scale_up_count: 2,
            scale_down_count: 1,
        };
        
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Maximum workers cannot exceed 1000"));
    }

    #[test]
    fn test_autoscaler_config_validation_invalid_scale_up_threshold() {
        let config = AutoScalerConfig {
            min_workers: 1,
            max_workers: 10,
            scale_up_threshold: 0.0,
            scale_down_threshold: 1.0,
            scale_up_count: 2,
            scale_down_count: 1,
        };
        
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Scale up threshold must be between 0.1 and 100.0"));
    }

    #[test]
    fn test_autoscaler_config_validation_invalid_scale_down_threshold() {
        let config = AutoScalerConfig {
            min_workers: 1,
            max_workers: 10,
            scale_up_threshold: 5.0,
            scale_down_threshold: 6.0, // Greater than scale_up_threshold
            scale_up_count: 2,
            scale_down_count: 1,
        };
        
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Scale down threshold must be between 0.0 and scale up threshold"));
    }

    #[test]
    fn test_autoscaler_config_validation_invalid_scale_counts() {
        let config = AutoScalerConfig {
            min_workers: 1,
            max_workers: 10,
            scale_up_threshold: 5.0,
            scale_down_threshold: 1.0,
            scale_up_count: 0, // Invalid
            scale_down_count: 1,
        };
        
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Scale up count must be between 1 and 50"));
    }

    #[test]
    fn test_autoscaler_config_serialization() {
        let config = AutoScalerConfig::default();
        
        // Test JSON serialization
        let json = serde_json::to_string(&config).expect("Failed to serialize to JSON");
        let deserialized: AutoScalerConfig = serde_json::from_str(&json).expect("Failed to deserialize from JSON");
        
        assert_eq!(config.min_workers, deserialized.min_workers);
        assert_eq!(config.max_workers, deserialized.max_workers);
        assert_eq!(config.scale_up_threshold, deserialized.scale_up_threshold);
        assert_eq!(config.scale_down_threshold, deserialized.scale_down_threshold);
        assert_eq!(config.scale_up_count, deserialized.scale_up_count);
        assert_eq!(config.scale_down_count, deserialized.scale_down_count);
    }

    fn create_test_autoscaler_metrics(active_workers: i64, total_pending_tasks: i64) -> AutoScalerMetrics {
        let tasks_per_worker = if active_workers > 0 {
            total_pending_tasks as f64 / active_workers as f64
        } else {
            total_pending_tasks as f64
        };

        AutoScalerMetrics {
            active_workers,
            total_pending_tasks,
            queue_metrics: vec![
                QueueMetrics {
                    queue_name: "default".to_string(),
                    pending_tasks: total_pending_tasks,
                    processed_tasks: 100,
                    failed_tasks: 5,
                },
            ],
            tasks_per_worker,
        }
    }

    #[allow(dead_code)]
    fn create_mock_autoscaler() -> AutoScaler {
        // Create a dummy broker for testing purposes
        let redis_url = "redis://localhost:6379";
        let mut config = deadpool_redis::Config::from_url(redis_url);
        config.pool = Some(deadpool_redis::PoolConfig::new(1));
        
        let pool = config.create_pool(Some(deadpool_redis::Runtime::Tokio1))
            .expect("Failed to create pool");
        
        let broker = Arc::new(crate::broker::RedisBroker { pool });
        AutoScaler::new(broker)
    }

    #[test]
    fn test_scaling_action_scale_up() {
        let config = AutoScalerConfig {
            min_workers: 1,
            max_workers: 10,
            scale_up_threshold: 5.0,
            scale_down_threshold: 1.0,
            scale_up_count: 2,
            scale_down_count: 1,
        };

        let autoscaler = AutoScaler {
            broker: Arc::new(crate::broker::RedisBroker { 
                pool: deadpool_redis::Config::from_url("redis://localhost:6379")
                    .create_pool(Some(deadpool_redis::Runtime::Tokio1))
                    .expect("Failed to create pool")
            }),
            config,
        };
        
        // 2 workers, 12 tasks = 6 tasks per worker (> threshold of 5.0)
        let metrics = create_test_autoscaler_metrics(2, 12);
        
        let action = autoscaler.decide_scaling_action(&metrics).expect("Failed to decide scaling action");
        
        match action {
            ScalingAction::ScaleUp(count) => assert_eq!(count, 2),
            _ => panic!("Expected ScaleUp action"),
        }
    }

    #[test]
    fn test_scaling_action_scale_down() {
        let config = AutoScalerConfig {
            min_workers: 1,
            max_workers: 10,
            scale_up_threshold: 5.0,
            scale_down_threshold: 1.0,
            scale_up_count: 2,
            scale_down_count: 1,
        };

        let autoscaler = AutoScaler {
            broker: Arc::new(crate::broker::RedisBroker { 
                pool: deadpool_redis::Config::from_url("redis://localhost:6379")
                    .create_pool(Some(deadpool_redis::Runtime::Tokio1))
                    .expect("Failed to create pool")
            }),
            config,
        };
        
        // 5 workers, 2 tasks = 0.4 tasks per worker (< threshold of 1.0)
        let metrics = create_test_autoscaler_metrics(5, 2);
        
        let action = autoscaler.decide_scaling_action(&metrics).expect("Failed to decide scaling action");
        
        match action {
            ScalingAction::ScaleDown(count) => assert_eq!(count, 1),
            _ => panic!("Expected ScaleDown action"),
        }
    }

    #[test]
    fn test_scaling_action_no_action() {
        let config = AutoScalerConfig::default();

        let autoscaler = AutoScaler {
            broker: Arc::new(crate::broker::RedisBroker { 
                pool: deadpool_redis::Config::from_url("redis://localhost:6379")
                    .create_pool(Some(deadpool_redis::Runtime::Tokio1))
                    .expect("Failed to create pool")
            }),
            config,
        };
        
        // 3 workers, 9 tasks = 3 tasks per worker (between thresholds)
        let metrics = create_test_autoscaler_metrics(3, 9);
        
        let action = autoscaler.decide_scaling_action(&metrics).expect("Failed to decide scaling action");
        
        match action {
            ScalingAction::NoAction => {},
            _ => panic!("Expected NoAction"),
        }
    }

    #[test]
    fn test_scaling_action_at_max_workers() {
        let config = AutoScalerConfig {
            min_workers: 1,
            max_workers: 5,
            scale_up_threshold: 2.0,
            scale_down_threshold: 0.5,
            scale_up_count: 3,
            scale_down_count: 1,
        };

        let autoscaler = AutoScaler {
            broker: Arc::new(crate::broker::RedisBroker { 
                pool: deadpool_redis::Config::from_url("redis://localhost:6379")
                    .create_pool(Some(deadpool_redis::Runtime::Tokio1))
                    .expect("Failed to create pool")
            }),
            config,
        };
        
        // 5 workers (at max), 20 tasks = 4 tasks per worker (> threshold but can't scale up)
        let metrics = create_test_autoscaler_metrics(5, 20);
        
        let action = autoscaler.decide_scaling_action(&metrics).expect("Failed to decide scaling action");
        
        match action {
            ScalingAction::NoAction => {},
            _ => panic!("Expected NoAction when at max workers"),
        }
    }

    #[test]
    fn test_scaling_action_at_min_workers() {
        let config = AutoScalerConfig {
            min_workers: 3,
            max_workers: 10,
            scale_up_threshold: 5.0,
            scale_down_threshold: 1.0,
            scale_up_count: 2,
            scale_down_count: 2,
        };

        let autoscaler = AutoScaler {
            broker: Arc::new(crate::broker::RedisBroker { 
                pool: deadpool_redis::Config::from_url("redis://localhost:6379")
                    .create_pool(Some(deadpool_redis::Runtime::Tokio1))
                    .expect("Failed to create pool")
            }),
            config,
        };
        
        // 3 workers (at min), 1 task = 0.33 tasks per worker (< threshold but can't scale down)
        let metrics = create_test_autoscaler_metrics(3, 1);
        
        let action = autoscaler.decide_scaling_action(&metrics).expect("Failed to decide scaling action");
        
        match action {
            ScalingAction::NoAction => {},
            _ => panic!("Expected NoAction when at min workers"),
        }
    }

    #[test]
    fn test_scaling_action_limited_by_max_workers() {
        let config = AutoScalerConfig {
            min_workers: 1,
            max_workers: 5,
            scale_up_threshold: 2.0,
            scale_down_threshold: 0.5,
            scale_up_count: 10, // Want to scale up by 10, but max is 5
            scale_down_count: 1,
        };

        let autoscaler = AutoScaler {
            broker: Arc::new(crate::broker::RedisBroker { 
                pool: deadpool_redis::Config::from_url("redis://localhost:6379")
                    .create_pool(Some(deadpool_redis::Runtime::Tokio1))
                    .expect("Failed to create pool")
            }),
            config,
        };
        
        // 3 workers, want to scale up by 10, but only room for 2 more
        let metrics = create_test_autoscaler_metrics(3, 15); // 5 tasks per worker
        
        let action = autoscaler.decide_scaling_action(&metrics).expect("Failed to decide scaling action");
        
        match action {
            ScalingAction::ScaleUp(count) => assert_eq!(count, 2), // Limited by max_workers
            _ => panic!("Expected ScaleUp action limited by max workers"),
        }
    }

    #[test]
    fn test_scaling_action_limited_by_min_workers() {
        let config = AutoScalerConfig {
            min_workers: 3,
            max_workers: 10,
            scale_up_threshold: 5.0,
            scale_down_threshold: 1.0,
            scale_up_count: 2,
            scale_down_count: 10, // Want to scale down by 10, but min is 3
        };

        let autoscaler = AutoScaler {
            broker: Arc::new(crate::broker::RedisBroker { 
                pool: deadpool_redis::Config::from_url("redis://localhost:6379")
                    .create_pool(Some(deadpool_redis::Runtime::Tokio1))
                    .expect("Failed to create pool")
            }),
            config,
        };
        
        // 6 workers, want to scale down by 10, but can only scale down by 3
        let metrics = create_test_autoscaler_metrics(6, 3); // 0.5 tasks per worker
        
        let action = autoscaler.decide_scaling_action(&metrics).expect("Failed to decide scaling action");
        
        match action {
            ScalingAction::ScaleDown(count) => assert_eq!(count, 3), // Limited by min_workers
            _ => panic!("Expected ScaleDown action limited by min workers"),
        }
    }

    #[test]
    fn test_scaling_action_zero_workers() {
        let config = AutoScalerConfig::default();

        let autoscaler = AutoScaler {
            broker: Arc::new(crate::broker::RedisBroker { 
                pool: deadpool_redis::Config::from_url("redis://localhost:6379")
                    .create_pool(Some(deadpool_redis::Runtime::Tokio1))
                    .expect("Failed to create pool")
            }),
            config,
        };
        
        // 0 workers, 10 tasks = infinite tasks per worker (should scale up)
        let metrics = create_test_autoscaler_metrics(0, 10);
        
        let action = autoscaler.decide_scaling_action(&metrics).expect("Failed to decide scaling action");
        
        match action {
            ScalingAction::ScaleUp(count) => assert_eq!(count, 2),
            _ => panic!("Expected ScaleUp action with zero workers"),
        }
    }

    #[test]
    fn test_autoscaler_metrics_debug() {
        let metrics = create_test_autoscaler_metrics(5, 25);
        let debug_str = format!("{:?}", metrics);
        
        assert!(debug_str.contains("AutoScalerMetrics"));
        assert!(debug_str.contains("active_workers: 5"));
        assert!(debug_str.contains("total_pending_tasks: 25"));
        assert!(debug_str.contains("tasks_per_worker: 5"));
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
        assert_eq!(original.scale_up_threshold, cloned.scale_up_threshold);
        assert_eq!(original.scale_down_threshold, cloned.scale_down_threshold);
        assert_eq!(original.scale_up_count, cloned.scale_up_count);
        assert_eq!(original.scale_down_count, cloned.scale_down_count);
    }
}
