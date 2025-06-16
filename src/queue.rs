use dashmap::DashMap;

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
            name: "default".to_string(),
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

        // Add default queues
        manager.add_queue(QueueConfig::default());
        manager.add_queue(QueueConfig {
            name: "high_priority".to_string(),
            priority: 10,
            max_retries: 5,
            timeout_seconds: 600,
            rate_limit: None,
        });
        manager.add_queue(QueueConfig {
            name: "low_priority".to_string(),
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
