# Common pl3xus Patterns Reference

## Plugin Module Structure

The standard structure for device/feature plugins:

```
my_plugin/
├── src/
│   ├── types/              # Always available types with conditional derives
│   │   ├── mod.rs          # Re-exports all types
│   │   ├── config.rs       # Configuration types (database storage)
│   │   ├── device.rs       # ECS component types
│   │   └── requests.rs     # Request/response message types
│   ├── database/           # Server-only: database schema and queries
│   │   ├── mod.rs
│   │   ├── schema.rs
│   │   └── queries.rs
│   ├── systems/            # Server-only: handler systems
│   │   ├── mod.rs
│   │   ├── handlers.rs     # Request handlers
│   │   └── command.rs      # Device command systems
│   ├── connection.rs       # Server-only: device driver
│   ├── plugin.rs           # ECS-only: Bevy plugin
│   └── lib.rs              # Main exports with feature gates
```

### lib.rs Structure

```rust
use cfg_if::cfg_if;

// ============================================================================
// TYPES - Always available (conditionally compiled Component derives)
// ============================================================================
pub mod types;

pub use types::{
    // Config types
    MyConnectionConfig, MyConnectionType,
    // Device component types
    MyControllerConfig, MyStatus, ActiveMyDevice,
    // Request/response types
    ListMyConnections, ListMyConnectionsResponse,
    CreateMyConnection, CreateMyConnectionResponse,
    // ... etc
};

// ============================================================================
// SERVER-ONLY - Systems, handlers, database, driver
// ============================================================================
cfg_if! {
    if #[cfg(feature = "server")] {
        mod connection;
        pub mod database;
        pub mod systems;

        pub use connection::MyDriver;
        pub use database::MyDatabaseInit;
        pub use systems::{
            MyCommandEvent, MyControllerBundle,
            MyHandlerPlugin,
            my_command_handler_system, my_tcp_sender_system,
        };
    }
}

// ============================================================================
// ECS-ONLY - Plugin
// ============================================================================
cfg_if! {
    if #[cfg(feature = "ecs")] {
        mod plugin;

        pub use plugin::MyPlugin;
    }
}
```

### Conditional Component Derives in types/device.rs

```rust
#[cfg(feature = "ecs")]
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Component type - available on both client and server
/// On server: derives Component for ECS
/// On client: plain data type for stores
#[cfg_attr(feature = "ecs", derive(Component))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct MyStatus {
    pub connected: bool,
    pub value: f32,
}
```

### Conditional Macro Derives in types/requests.rs

```rust
use pl3xus_common::RequestMessage;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use pl3xus_macros::{HasSuccess, Invalidates};

/// Request type - always available
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(Invalidates))]
#[cfg_attr(feature = "server", invalidates("ListMyConnections"))]
pub struct CreateMyConnection {
    pub name: String,
}

/// Response type - always available
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(HasSuccess))]
pub struct CreateMyConnectionResponse {
    pub success: bool,
    pub id: Option<i64>,
    pub error: Option<String>,
}

impl RequestMessage for CreateMyConnection {
    type ResponseMessage = CreateMyConnectionResponse;
}
```

## Server Patterns

### Plugin Organization

```rust
// src/plugins/robot.rs
pub struct RobotPlugin;

impl Plugin for RobotPlugin {
    fn build(&self, app: &mut App) {
        app
            // Component sync
            .sync_component::<RobotState>(None)
            .sync_component::<RobotConfig>(None)
            
            // Targeted requests with authorization
            .request::<UpdateRobotConfig, WebSocketProvider>()
                .targeted()
                .with_default_entity_policy()
                .register()
            
            // Systems
            .add_systems(Startup, spawn_initial_robots)
            .add_systems(Update, (
                update_robot_state,
                process_robot_commands,
            ));
    }
}
```

### Batch Request Registration

```rust
// Register multiple related requests together
app.requests::<(
    StartProgram,
    PauseProgram,
    ResumeProgram,
    StopProgram,
), WebSocketProvider>()
    .targeted()
    .with_default_entity_policy()
    .with_error_response();
```

### Message Handler Pattern

```rust
fn handle_update_config(
    mut messages: MessageReader<NetworkData<TargetedRequest<UpdateRobotConfig>>>,
    mut configs: Query<&mut RobotConfig>,
) {
    for request in messages.read() {
        let entity = Entity::from_bits(request.message.target_entity);
        
        if let Ok(mut config) = configs.get_mut(entity) {
            *config = request.message.request.config.clone();
            let _ = request.respond(UpdateConfigResponse { success: true, error: None });
        } else {
            let _ = request.respond(UpdateConfigResponse {
                success: false,
                error: Some("Entity not found".into()),
            });
        }
    }
}
```

## Client Patterns

### Entity Context Pattern

```rust
#[derive(Clone)]
pub struct RobotContext {
    pub id: u64,
}

#[component]
fn RobotPanel(robot_id: u64) -> impl IntoView {
    provide_context(RobotContext { id: robot_id });
    
    view! {
        <RobotHeader />
        <RobotControls />
        <RobotStatus />
    }
}

#[component]
fn RobotControls() -> impl IntoView {
    let ctx = expect_context::<RobotContext>();
    let state = use_entity_component::<RobotState>(ctx.id);
    
    // Use server-driven can_* flags
    view! {
        <button
            disabled=move || !state.get().map(|s| s.can_start).unwrap_or(false)
            on:click=move |_| start_robot(ctx.id)
        >
            "Start"
        </button>
    }
}
```

### Mutation with Handler

```rust
#[component]
fn UpdateConfigButton(robot_id: u64) -> impl IntoView {
    let toast = use_toast();
    
    let update = use_mutation_targeted::<UpdateRobotConfig>(move |result| {
        match result {
            Ok(r) if r.success => toast.success("Config updated"),
            Ok(r) => toast.error(r.error.unwrap_or_default()),
            Err(e) => toast.error(e),
        }
    });
    
    view! {
        <button
            disabled=move || update.is_pending()
            on:click=move |_| update.send(robot_id, UpdateRobotConfig { ... })
        >
            {move || if update.is_pending() { "Saving..." } else { "Save" }}
        </button>
    }
}
```

### Text Input with Validation

```rust
#[component]
fn NumericInput(
    value: RwSignal<f64>,
    #[prop(optional)] min: Option<f64>,
    #[prop(optional)] max: Option<f64>,
) -> impl IntoView {
    let text = RwSignal::new(value.get().to_string());
    let is_valid = RwSignal::new(true);
    
    view! {
        <input
            type="text"
            class:invalid=move || !is_valid.get()
            prop:value=move || text.get()
            on:input=move |ev| {
                let val = event_target_value(&ev);
                text.set(val.clone());
                match val.parse::<f64>() {
                    Ok(num) => {
                        let in_range = min.map(|m| num >= m).unwrap_or(true)
                            && max.map(|m| num <= m).unwrap_or(true);
                        if in_range {
                            value.set(num);
                            is_valid.set(true);
                        } else {
                            is_valid.set(false);
                        }
                    }
                    Err(_) => is_valid.set(false),
                }
            }
        />
    }
}
```

## Shared Type Patterns

### Request with Response

```rust
use pl3xus_common::RequestMessage;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateRobotConfig {
    pub config: RobotConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateConfigResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl RequestMessage for UpdateRobotConfig {
    type ResponseMessage = UpdateConfigResponse;
}
```

### Mutation with Invalidation

```rust
use pl3xus_macros::Invalidates;

#[derive(Clone, Debug, Serialize, Deserialize, Invalidates)]
#[invalidates("ListPrograms", "GetProgramStats")]
pub struct CreateProgram {
    pub name: String,
}
```

### Server-Driven State

```rust
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ProgramState {
    pub state: ExecutionState,
    // Server determines valid actions
    pub can_start: bool,
    pub can_pause: bool,
    pub can_resume: bool,
    pub can_stop: bool,
    pub can_unload: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub enum ExecutionState {
    #[default]
    NoProgram,
    Idle,
    Running,
    Paused,
    Completed,
    Error(String),
}
```

