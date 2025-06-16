use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::Arc;
use uuid::Uuid;

pub type TaskId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetadata {
    pub id: TaskId,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub attempts: u32,
    pub max_retries: u32,
    pub timeout_seconds: u64,
}

impl Default for TaskMetadata {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "unknown".to_string(),
            created_at: Utc::now(),
            attempts: 0,
            max_retries: 3,
            timeout_seconds: 300,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskWrapper {
    pub metadata: TaskMetadata,
    pub payload: Vec<u8>,
}

#[async_trait]
pub trait Task: Send + Sync + Serialize + for<'de> Deserialize<'de> + Debug {
    async fn execute(&self) -> TaskResult;

    fn name(&self) -> &str;

    fn max_retries(&self) -> u32 {
        3
    }

    fn timeout_seconds(&self) -> u64 {
        300
    }
}

/// Result type for task execution
pub type TaskResult = Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>;

/// Future type for task execution
pub type TaskFuture = std::pin::Pin<Box<dyn std::future::Future<Output = TaskResult> + Send>>;

/// Type-erased task executor function
pub type TaskExecutor = Arc<dyn Fn(Vec<u8>) -> TaskFuture + Send + Sync>;

/// Registration information for automatic task registration via inventory
#[cfg(feature = "auto-register")]
pub struct TaskRegistration {
    pub type_name: &'static str,
    pub register_fn: fn(&TaskRegistry) -> Result<(), Box<dyn std::error::Error + Send + Sync>>,
}

#[cfg(feature = "auto-register")]
inventory::collect!(TaskRegistration);

/// Task registry for mapping task names to executors
pub struct TaskRegistry {
    executors: DashMap<String, TaskExecutor>,
}

impl TaskRegistry {
    pub fn new() -> Self {
        Self {
            executors: DashMap::new(),
        }
    }

    /// Create a new registry with all automatically registered tasks
    #[cfg(feature = "auto-register")]
    pub fn with_auto_registered() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let registry = Self::new();
        registry.auto_register_tasks()?;
        Ok(registry)
    }

    /// Create a new registry with all automatically registered tasks and configuration
    #[cfg(feature = "auto-register")]
    pub fn with_auto_registered_and_config(
        config: Option<&crate::config::AutoRegisterConfig>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let registry = Self::new();
        registry.auto_register_tasks_with_config(config)?;
        Ok(registry)
    }

    /// Register all tasks that have been submitted via the inventory pattern
    #[cfg(feature = "auto-register")]
    pub fn auto_register_tasks(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.auto_register_tasks_with_config(None)
    }

    /// Register all tasks with optional configuration
    #[cfg(feature = "auto-register")]
    pub fn auto_register_tasks_with_config(
        &self,
        _config: Option<&crate::config::AutoRegisterConfig>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        #[cfg(feature = "tracing")]
        tracing::info!("Auto-registering tasks...");

        let mut registered_count = 0;
        let mut errors = Vec::new();

        // Register tasks from the inventory pattern (compile-time registered)
        for registration in inventory::iter::<TaskRegistration> {
            #[cfg(feature = "tracing")]
            tracing::debug!("Auto-registering task type: {}", registration.type_name);

            match (registration.register_fn)(self) {
                Ok(()) => {
                    registered_count += 1;
                    #[cfg(feature = "tracing")]
                    tracing::debug!(
                        "Successfully registered task type: {}",
                        registration.type_name
                    );
                }
                Err(e) => {
                    #[cfg(feature = "tracing")]
                    tracing::error!(
                        "Failed to register task type {}: {}",
                        registration.type_name,
                        e
                    );
                    errors.push(format!(
                        "Failed to register {}: {}",
                        registration.type_name, e
                    ));
                }
            }
        }

        if !errors.is_empty() {
            return Err(format!("Task registration errors: {}", errors.join(", ")).into());
        }

        #[cfg(feature = "tracing")]
        tracing::info!("Auto-registered {} task types total", registered_count);

        Ok(())
    }

    /// Register a task type with explicit name
    pub fn register_with_name<T>(
        &self,
        task_name: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        T: Task + 'static,
    {
        let executor: TaskExecutor = Arc::new(move |payload| {
            Box::pin(async move {
                match rmp_serde::from_slice::<T>(&payload) {
                    Ok(task) => task.execute().await,
                    Err(e) => Err(format!("Failed to deserialize task: {}", e).into()),
                }
            })
        });

        self.executors.insert(task_name.to_string(), executor);

        Ok(())
    }

    /// Execute a task by name
    pub async fn execute(&self, task_name: &str, payload: Vec<u8>) -> TaskResult {
        let executor = self.executors.get(task_name).map(|e| e.clone());

        if let Some(executor) = executor {
            executor(payload).await
        } else {
            Err(format!("Unknown task type: {}", task_name).into())
        }
    }

    /// Get list of registered task names
    pub fn registered_tasks(&self) -> Vec<String> {
        self.executors.iter().map(|entry| entry.key().clone()).collect()
    }
}

impl Default for TaskRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Macro to make task registration easier
#[macro_export]
macro_rules! manual_register_task {
    ($registry:expr, $task_type:ty) => {{
        // We need a way to get the task name from the type
        // This requires the task to implement Default temporarily
        let temp_instance = <$task_type as Default>::default();
        let task_name = temp_instance.name().to_string();
        $registry.register_with_name::<$task_type>(&task_name)
    }};
}

// Helper macro for registering multiple tasks at once
#[macro_export]
macro_rules! register_tasks {
    ($registry:expr, $($task_type:ty),+ $(,)?) => {
        {
            let mut results = Vec::new();
            $(
                results.push($crate::manual_register_task!($registry, $task_type));
            )+

            // Return the first error if any, otherwise Ok
            for result in results {
                if let Err(e) = result {
                    return Err(e);
                }
            }
            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        }
    };
}

// Alternative macro that doesn't require Default trait
#[macro_export]
macro_rules! register_task_with_name {
    ($registry:expr, $task_type:ty, $name:expr) => {
        $registry.register_with_name::<$task_type>($name)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};


    #[derive(Debug, Serialize, Deserialize, Clone, Default)]
    struct TestTask {
        pub data: String,
        pub should_fail: bool,
    }

    #[async_trait]
    impl Task for TestTask {
        async fn execute(&self) -> TaskResult {
            if self.should_fail {
                return Err("Task intentionally failed".into());
            }
            
            // Simulate some work
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            
            #[derive(Serialize)]
            struct Response {
                status: String,
                processed_data: String,
            }
            
            let response = Response {
                status: "completed".to_string(),
                processed_data: format!("Processed: {}", self.data),
            };
            
            Ok(rmp_serde::to_vec(&response)?)
        }

        fn name(&self) -> &str {
            "test_task"
        }

        fn max_retries(&self) -> u32 {
            2
        }

        fn timeout_seconds(&self) -> u64 {
            30
        }
    }

    #[tokio::test]
    async fn test_task_registry_creation() {
        let registry = TaskRegistry::new();
        assert_eq!(registry.registered_tasks().len(), 0);
    }

    #[tokio::test]
    async fn test_task_registration() {
        let registry = TaskRegistry::new();
        
        // Register a task
        registry.register_with_name::<TestTask>("test_task")
            .expect("Failed to register task");
        
        let tasks = registry.registered_tasks();
        assert_eq!(tasks.len(), 1);
        assert!(tasks.contains(&"test_task".to_string()));
    }

    #[tokio::test]
    async fn test_task_execution() {
        let registry = TaskRegistry::new();
        registry.register_with_name::<TestTask>("test_task")
            .expect("Failed to register task");
        
        let task = TestTask {
            data: "Hello, World!".to_string(),
            should_fail: false,
        };
        
        let payload = rmp_serde::to_vec(&task).expect("Failed to serialize task");
        let result = registry.execute("test_task", payload).await;
        
        assert!(result.is_ok());
        let response_data = result.unwrap();
        assert!(!response_data.is_empty());
        
        // Verify the response contains expected data
        // Since we're using MessagePack, we need to deserialize it properly
        #[derive(serde::Deserialize)]
        struct Response {
            status: String,
            processed_data: String,
        }
        
        let response: Response = rmp_serde::from_slice(&response_data)
            .expect("Failed to deserialize response");
        assert_eq!(response.status, "completed");
        assert!(response.processed_data.contains("Hello, World!"));
    }

    #[tokio::test]
    async fn test_task_execution_failure() {
        let registry = TaskRegistry::new();
        registry.register_with_name::<TestTask>("test_task")
            .expect("Failed to register task");
        
        let task = TestTask {
            data: "This will fail".to_string(),
            should_fail: true,
        };
        
        let payload = rmp_serde::to_vec(&task).expect("Failed to serialize task");
        let result = registry.execute("test_task", payload).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("intentionally failed"));
    }

    #[tokio::test]
    async fn test_unknown_task_execution() {
        let registry = TaskRegistry::new();
        
        let result = registry.execute("unknown_task", vec![1, 2, 3]).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown task type"));
    }

    #[tokio::test]
    async fn test_task_metadata_default() {
        let metadata = TaskMetadata::default();
        
        assert_eq!(metadata.name, "unknown");
        assert_eq!(metadata.attempts, 0);
        assert_eq!(metadata.max_retries, 3);
        assert_eq!(metadata.timeout_seconds, 300);
    }

    #[tokio::test]
    async fn test_task_wrapper_serialization() {
        let metadata = TaskMetadata {
            id: TaskId::new_v4(),
            name: "test_task".to_string(),
            created_at: chrono::Utc::now(),
            attempts: 1,
            max_retries: 3,
            timeout_seconds: 300,
        };
        
        let wrapper = TaskWrapper {
            metadata: metadata.clone(),
            payload: vec![1, 2, 3, 4],
        };
        
        // Test serialization
        let serialized = rmp_serde::to_vec(&wrapper).expect("Failed to serialize wrapper");
        assert!(!serialized.is_empty());
        
        // Test deserialization
        let deserialized: TaskWrapper = rmp_serde::from_slice(&serialized)
            .expect("Failed to deserialize wrapper");
        
        assert_eq!(deserialized.metadata.id, metadata.id);
        assert_eq!(deserialized.metadata.name, metadata.name);
        assert_eq!(deserialized.payload, vec![1, 2, 3, 4]);
    }

    #[tokio::test]
    async fn test_multiple_task_registration() {
        let registry = TaskRegistry::new();
        
        // Register multiple tasks
        registry.register_with_name::<TestTask>("task1")
            .expect("Failed to register task1");
        registry.register_with_name::<TestTask>("task2")
            .expect("Failed to register task2");
        
        let tasks = registry.registered_tasks();
        assert_eq!(tasks.len(), 2);
        assert!(tasks.contains(&"task1".to_string()));
        assert!(tasks.contains(&"task2".to_string()));
    }

    #[tokio::test]
    async fn test_task_registry_concurrent_access() {
        let registry = Arc::new(TaskRegistry::new());
        registry.register_with_name::<TestTask>("test_task")
            .expect("Failed to register task");
        
        let task = TestTask {
            data: "Concurrent test".to_string(),
            should_fail: false,
        };
        let payload = rmp_serde::to_vec(&task).expect("Failed to serialize task");
        
        // Execute multiple tasks concurrently
        let mut handles = Vec::new();
        for i in 0..10 {
            let registry_clone = Arc::clone(&registry);
            let payload_clone = payload.clone();
            let handle = tokio::spawn(async move {
                let result = registry_clone.execute("test_task", payload_clone).await;
                assert!(result.is_ok(), "Task {} failed", i);
            });
            handles.push(handle);
        }
        
        // Wait for all tasks to complete
        for handle in handles {
            handle.await.expect("Task execution failed");
        }
    }
}
