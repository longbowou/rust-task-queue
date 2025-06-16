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
