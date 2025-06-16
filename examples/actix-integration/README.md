# Actix Web Integration Example

This example demonstrates how to integrate `rust-task-queue` with Actix Web using **automatic task registration**. It
showcases both traditional task organization and modular task organization to demonstrate the flexibility of the
auto-registration feature.

## Features Demonstrated

- ✅ **Auto-registration** - Tasks are automatically discovered
- ✅ **Modular task organization** - Tasks organized in separate modules
- ✅ **Mixed registration approaches** - Both `AutoRegisterTask` derive and `register_task` attribute
- ✅ **Configuration-driven setup** - Everything configured via `task-queue.toml`
- ✅ **Auto-configured Actix routes** - Health checks, metrics, and task management endpoints
- ✅ **Scheduling and queueing** - Different queue priorities and scheduling
- ✅ **Comprehensive HTTP API** - REST endpoints for all task types

## Task Organization

The example demonstrates two approaches to task organization:

### 1. Traditional Single File (`src/tasks.rs`)

```rust
#[derive(Debug, Serialize, Deserialize, Default, AutoRegisterTask)]
pub struct EmailTask { ... }

#[derive(Debug, Serialize, Deserialize, Default, AutoRegisterTask)]  
pub struct NotificationTask { ... }
```

### 2. Modular Organization (`src/tasks/`)

```
src/tasks/
├── mod.rs                 # Module exports
├── communication.rs       # Email, Slack, SMS tasks
└── processing.rs         # Analytics, ML, data processing tasks
```

The auto-registration configuration automatically discovers tasks:

```toml
[auto_register]
enabled = true
```

## Quick Start

### 1. Start Redis

```bash
docker-compose up -d redis
```

### 2. Start the Actix Web Server

```bash
cd examples/actix-integration
cargo run
```

You should see output like:

```
🚀 Starting Actix Integration App with Zero-Config
📁 Looking for task-queue.toml, task-queue.yaml, or environment variables...
✅ Task queue auto-configured successfully!
📋 Configuration loaded:
   Redis URL: redis://redis:6379
   Initial workers: 2
   Auto-scaling: true (min: 1, max: 10)
   Scheduler enabled: true
   Auto-registration enabled: true
   Route prefix: /rust-task-queue
🤖 Auto-registered 8 task types: ["email_task", "notification_task", "data_processing_task", "slack_notification_task", "sms_notification", "analytics_task", "ml_training"]
```

### 3. Start Task Workers

```bash
# In another terminal
cargo run --bin task-worker
```

Workers will show discovered tasks:

```
🤖 Starting Task Worker with Auto-Configuration
📦 Found 8 auto-registered task types:
   • email_task
   • notification_task  
   • data_processing_task
   • slack_notification_task
   • sms_notification
   • analytics_task
   • ml_training
```

## API Endpoints

The server exposes both original and modular task endpoints:

### Original Tasks (from `tasks.rs`)

- `POST /email` - Send email
- `POST /notification` - Send notification
- `POST /data-processing` - Process data
- `POST /schedule-email` - Schedule email with delay

### Modular Tasks (from `tasks/` directory)

- `POST /slack-notification` - Send Slack message
- `POST /sms` - Send SMS (registered as "sms_notification")
- `POST /analytics` - Record analytics event
- `POST /ml-training` - Train ML model (registered as "ml_training")

### Auto-configured Management Routes

- `GET /rust-task-queue/health` - Health check
- `GET /rust-task-queue/metrics` - Auto-scaler metrics
- `GET /rust-task-queue/registered` - List registered tasks

## Testing with HTTP Client

Use the provided test file:

```bash
# Install httpie if needed
pip install httpie

# Test endpoints
http POST localhost:8000/slack-notification channel=general message="Hello World" username="TestBot"
http POST localhost:8000/sms phone_number="+1234567890" message="Test SMS" priority:=1
http GET localhost:8000/rust-task-queue/registered
```

## Auto Registration

The auto-registration feature automatically discovers and registers tasks using derive macros and attributes. This
eliminates the need for manual task registration.

## Configuration Options

The `task-queue.toml` file demonstrates comprehensive configuration:

```toml
[auto_register]
enabled = true

[redis]
url = "redis://redis:6379"
pool_size = 5

[workers]
initial_count = 2
max_concurrent_tasks = 10

[autoscaler]
enabled = true
min_workers = 1
max_workers = 10

[actix]
auto_configure_routes = true
route_prefix = "/rust-task-queue"
enable_metrics = true
enable_health_check = true
```

## Task Registration Examples

### Using AutoRegisterTask Derive

```rust
#[derive(Debug, Serialize, Deserialize, Default, AutoRegisterTask)]
pub struct EmailTask {
    pub to: String,
    pub subject: String,
}

#[async_trait]
impl Task for EmailTask {
    async fn execute(&self) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        println!("📧 Sending email to: {}", self.to);
        Ok(json!({"status": "sent", "to": self.to}))
    }

    fn name(&self) -> &str {
        "email_task"  // Registered with this name
    }
}
```

### Using register_task Attribute

```rust
#[register_task("sms_notification")]  // Custom registration name
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SmsTask {
    pub phone_number: String,
    pub message: String,
}

#[async_trait]
impl Task for SmsTask {
    async fn execute(&self) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        println!("📱 Sending SMS to: {}", self.phone_number);
        Ok(json!({"status": "sent"}))
    }

    fn name(&self) -> &str {
        "sms_task"  // This name is ignored due to the attribute
    }
}
```

## Benefits of Scan Paths

1. **Automatic Discovery**: No manual task registration needed
2. **Validation**: Ensures tasks are in expected locations
3. **Organization**: Supports both single-file and modular approaches
4. **Debugging**: Clear feedback on task location and registration
5. **Flexibility**: Mix different registration approaches as needed
6. **Maintenance**: Easy to reorganize tasks without breaking registration

This example showcases how scan paths make task management more robust and organized while maintaining the simplicity of
automatic registration. 