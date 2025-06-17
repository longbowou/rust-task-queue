use dashmap::DashMap;

/// Standard queue names used throughout the task queue system
pub mod queue_names {
    pub const DEFAULT: &str = "default";
    pub const HIGH_PRIORITY: &str = "high_priority";
    pub const LOW_PRIORITY: &str = "low_priority";
}

#[derive(Debug, Clone)]
pub struct QueueConfig {
    pub name: String,
    pub priority: i32,
    pub max_retries: u32,
    pub timeout_seconds: u64,
    pub rate_limit: Option<u32>, // Tasks per second
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            name: queue_names::DEFAULT.to_string(),
            priority: 0,
            max_retries: 3,
            timeout_seconds: 300,
            rate_limit: None,
        }
    }
}

pub struct QueueManager {
    queues: DashMap<String, QueueConfig>,
}

impl Default for QueueManager {
    fn default() -> Self {
        Self::new()
    }
}

impl QueueManager {
    pub fn new() -> Self {
        let manager = Self {
            queues: DashMap::new(),
        };

        // Add default queues using constants
        manager.add_queue(QueueConfig::default());
        manager.add_queue(QueueConfig {
            name: queue_names::HIGH_PRIORITY.to_string(),
            priority: 10,
            max_retries: 5,
            timeout_seconds: 600,
            rate_limit: None,
        });
        manager.add_queue(QueueConfig {
            name: queue_names::LOW_PRIORITY.to_string(),
            priority: -10,
            max_retries: 2,
            timeout_seconds: 120,
            rate_limit: Some(10), // 10 tasks per second
        });

        manager
    }

    pub fn add_queue(&self, config: QueueConfig) {
        self.queues.insert(config.name.clone(), config);
    }

    pub fn get_queue_config(&self, name: &str) -> Option<QueueConfig> {
        self.queues.get(name).map(|entry| entry.clone())
    }

    pub fn get_queue_names(&self) -> Vec<String> {
        self.queues
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    pub fn get_queues_by_priority(&self) -> Vec<QueueConfig> {
        let mut queues: Vec<QueueConfig> = self
            .queues
            .iter()
            .map(|entry| entry.value().clone())
            .collect();

        queues.sort_by(|a, b| b.priority.cmp(&a.priority));
        queues
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_config_default() {
        let config = QueueConfig::default();
        assert_eq!(config.name, queue_names::DEFAULT);
        assert_eq!(config.priority, 0);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.timeout_seconds, 300);
        assert_eq!(config.rate_limit, None);
    }

    #[test]
    fn test_queue_config_creation() {
        let config = QueueConfig {
            name: "test_queue".to_string(),
            priority: 5,
            max_retries: 2,
            timeout_seconds: 600,
            rate_limit: Some(100),
        };

        assert_eq!(config.name, "test_queue");
        assert_eq!(config.priority, 5);
        assert_eq!(config.max_retries, 2);
        assert_eq!(config.timeout_seconds, 600);
        assert_eq!(config.rate_limit, Some(100));
    }

    #[test]
    fn test_queue_manager_creation() {
        let manager = QueueManager::new();
        let queue_names = manager.get_queue_names();

        assert_eq!(queue_names.len(), 3);
        assert!(queue_names.contains(&queue_names::DEFAULT.to_string()));
        assert!(queue_names.contains(&queue_names::HIGH_PRIORITY.to_string()));
        assert!(queue_names.contains(&queue_names::LOW_PRIORITY.to_string()));
    }

    #[test]
    fn test_queue_manager_default() {
        let manager = QueueManager::default();
        let queue_names = manager.get_queue_names();
        assert_eq!(queue_names.len(), 3);
    }

    #[test]
    fn test_add_custom_queue() {
        let manager = QueueManager::new();

        let custom_config = QueueConfig {
            name: "custom".to_string(),
            priority: 100,
            max_retries: 1,
            timeout_seconds: 30,
            rate_limit: Some(50),
        };

        manager.add_queue(custom_config.clone());

        let retrieved = manager.get_queue_config("custom").unwrap();
        assert_eq!(retrieved.name, "custom");
        assert_eq!(retrieved.priority, 100);
        assert_eq!(retrieved.max_retries, 1);
        assert_eq!(retrieved.timeout_seconds, 30);
        assert_eq!(retrieved.rate_limit, Some(50));
    }

    #[test]
    fn test_get_queue_config_existing() {
        let manager = QueueManager::new();

        let default_config = manager.get_queue_config(queue_names::DEFAULT).unwrap();
        assert_eq!(default_config.name, queue_names::DEFAULT);
        assert_eq!(default_config.priority, 0);

        let high_config = manager
            .get_queue_config(queue_names::HIGH_PRIORITY)
            .unwrap();
        assert_eq!(high_config.name, queue_names::HIGH_PRIORITY);
        assert_eq!(high_config.priority, 10);

        let low_config = manager.get_queue_config(queue_names::LOW_PRIORITY).unwrap();
        assert_eq!(low_config.name, queue_names::LOW_PRIORITY);
        assert_eq!(low_config.priority, -10);
    }

    #[test]
    fn test_get_queue_config_nonexistent() {
        let manager = QueueManager::new();
        let result = manager.get_queue_config("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_queues_by_priority() {
        let manager = QueueManager::new();
        let queues = manager.get_queues_by_priority();

        assert_eq!(queues.len(), 3);

        // Should be sorted by priority in descending order
        assert_eq!(queues[0].priority, 10); // high_priority
        assert_eq!(queues[1].priority, 0); // default
        assert_eq!(queues[2].priority, -10); // low_priority

        assert_eq!(queues[0].name, queue_names::HIGH_PRIORITY);
        assert_eq!(queues[1].name, queue_names::DEFAULT);
        assert_eq!(queues[2].name, queue_names::LOW_PRIORITY);
    }

    #[test]
    fn test_queue_priority_sorting() {
        let manager = QueueManager::new();

        // Add some custom queues with various priorities
        manager.add_queue(QueueConfig {
            name: "highest".to_string(),
            priority: 100,
            max_retries: 3,
            timeout_seconds: 300,
            rate_limit: None,
        });

        manager.add_queue(QueueConfig {
            name: "lowest".to_string(),
            priority: -100,
            max_retries: 3,
            timeout_seconds: 300,
            rate_limit: None,
        });

        let queues = manager.get_queues_by_priority();
        assert_eq!(queues.len(), 5);

        // Verify sorting: 100, 10, 0, -10, -100
        assert_eq!(queues[0].priority, 100);
        assert_eq!(queues[1].priority, 10);
        assert_eq!(queues[2].priority, 0);
        assert_eq!(queues[3].priority, -10);
        assert_eq!(queues[4].priority, -100);
    }

    #[test]
    fn test_queue_config_clone() {
        let original = QueueConfig {
            name: "test".to_string(),
            priority: 5,
            max_retries: 2,
            timeout_seconds: 600,
            rate_limit: Some(100),
        };

        let cloned = original.clone();
        assert_eq!(original.name, cloned.name);
        assert_eq!(original.priority, cloned.priority);
        assert_eq!(original.max_retries, cloned.max_retries);
        assert_eq!(original.timeout_seconds, cloned.timeout_seconds);
        assert_eq!(original.rate_limit, cloned.rate_limit);
    }

    #[test]
    fn test_queue_config_debug() {
        let config = QueueConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("QueueConfig"));
        assert!(debug_str.contains(&config.name));
    }

    #[test]
    fn test_queue_manager_concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        let manager = Arc::new(QueueManager::new());
        let mut handles = Vec::new();

        // Spawn multiple threads adding queues
        for i in 0..10 {
            let manager_clone = Arc::clone(&manager);
            let handle = thread::spawn(move || {
                let config = QueueConfig {
                    name: format!("thread_queue_{}", i),
                    priority: i,
                    max_retries: 3,
                    timeout_seconds: 300,
                    rate_limit: None,
                };
                manager_clone.add_queue(config);
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all queues were added (3 default + 10 thread queues)
        let queue_names = manager.get_queue_names();
        assert_eq!(queue_names.len(), 13);
    }

    #[test]
    fn test_queue_names_constants() {
        assert_eq!(queue_names::DEFAULT, "default");
        assert_eq!(queue_names::HIGH_PRIORITY, "high_priority");
        assert_eq!(queue_names::LOW_PRIORITY, "low_priority");
    }

    #[test]
    fn test_queue_manager_overwrite_existing() {
        let manager = QueueManager::new();

        // Overwrite the default queue with custom settings
        let custom_default = QueueConfig {
            name: queue_names::DEFAULT.to_string(),
            priority: 999,
            max_retries: 10,
            timeout_seconds: 1000,
            rate_limit: Some(1),
        };

        manager.add_queue(custom_default);

        let retrieved = manager.get_queue_config(queue_names::DEFAULT).unwrap();
        assert_eq!(retrieved.priority, 999);
        assert_eq!(retrieved.max_retries, 10);
        assert_eq!(retrieved.timeout_seconds, 1000);
        assert_eq!(retrieved.rate_limit, Some(1));
    }
}
