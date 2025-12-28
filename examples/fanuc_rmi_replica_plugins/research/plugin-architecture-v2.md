# Plugin Architecture v2

## Current Issues

1. **Core database has plugin-specific schemas** - `core/database.rs` contains robot_connections, programs, etc.
2. **Re-exporting pl3xus_* types** - Creates unnecessary coupling
3. **Duplicate robot code** - `/robot`, `/robotics`, `/fanuc` overlap
4. **Plugin loading split** - Server manually loads plugins instead of `build()` doing it all

## Target Architecture

```
plugins/
├── src/
│   └── lib.rs              # build() loads ALL plugins
├── core/                   # Base infrastructure only
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── database.rs     # DatabaseResource + DatabaseInit trait
│       └── types.rs        # ActiveSystem only
├── robotics/               # Robot-agnostic types (unchanged)
├── execution/              # Execution orchestration (unchanged)
├── fanuc/                  # FANUC robot plugin (consolidated)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── database/       # FANUC-specific schemas
│       │   ├── mod.rs
│       │   ├── schema.rs   # CREATE TABLE statements
│       │   └── queries.rs  # Query functions
│       ├── types.rs        # RobotPosition, JogSettings, etc.
│       ├── connection.rs
│       ├── handlers.rs
│       ├── polling.rs
│       ├── program.rs
│       ├── jogging.rs
│       ├── sync.rs
│       ├── conversion.rs   # Quaternion↔WPR
│       └── motion.rs       # Motion command handling
└── duet/                   # Duet extruder (unchanged)
```

## Database Architecture

### DatabaseInit Trait
```rust
/// Trait for plugins to register their database schemas and migrations.
pub trait DatabaseInit: Send + Sync + 'static {
    /// Plugin name for logging and error messages.
    fn name(&self) -> &'static str;
    
    /// Initialize schema (CREATE TABLE IF NOT EXISTS).
    fn init_schema(&self, conn: &Connection) -> anyhow::Result<()>;
    
    /// Run migrations for existing databases.
    fn run_migrations(&self, conn: &Connection) -> anyhow::Result<()>;
    
    /// Insert seed data (INSERT OR IGNORE).
    fn seed_data(&self, conn: &Connection) -> anyhow::Result<()> {
        Ok(()) // Optional, default no-op
    }
}
```

### Core Plugin Database
```rust
/// Core database resource - just the connection.
#[derive(Resource)]
pub struct DatabaseResource(pub Arc<Mutex<Connection>>);

impl DatabaseResource {
    pub fn open(path: &str) -> anyhow::Result<Self>;
    
    /// Initialize all registered plugins' schemas.
    pub fn init_all(&self, plugins: &[Box<dyn DatabaseInit>]) -> anyhow::Result<()>;
}
```

### FANUC Plugin Database
```rust
pub struct FanucDatabaseInit;

impl DatabaseInit for FanucDatabaseInit {
    fn name(&self) -> &'static str { "fanuc" }
    
    fn init_schema(&self, conn: &Connection) -> anyhow::Result<()> {
        // robot_connections, robot_configurations, programs, etc.
    }
    
    fn run_migrations(&self, conn: &Connection) -> anyhow::Result<()> {
        // Add columns, etc.
    }
    
    fn seed_data(&self, conn: &Connection) -> anyhow::Result<()> {
        // Default robot connections, settings
    }
}
```

## Plugin Loading

### plugins/src/lib.rs
```rust
#[cfg(feature = "server")]
pub fn build() -> App {
    let mut app = App::new();
    
    // Core infrastructure
    app.add_plugins(core::CorePlugin);
    
    // Domain plugins
    app.add_plugins(fanuc::FanucPlugin);
    app.add_plugins(execution::ExecutionPlugin);
    
    // Optional plugins
    #[cfg(feature = "duet")]
    app.add_plugins(duet::DuetPlugin);
    
    app
}
```

### server/src/main.rs
```rust
fn main() {
    fanuc_replica_plugins::build().run();
}
```

## Consolidation Plan

### What moves from robot/ to fanuc/
- `types.rs` → `fanuc/types.rs`
- `connection.rs` → `fanuc/connection.rs`
- `handlers.rs` → `fanuc/handlers.rs`
- `polling.rs` → `fanuc/polling.rs`
- `program.rs` → `fanuc/program.rs`
- `jogging.rs` → `fanuc/jogging.rs`
- `sync.rs` → `fanuc/sync.rs`
- `plugin.rs` → merged into `fanuc/plugin.rs`
- All database schemas → `fanuc/database/`

### What stays in robotics/
- `RobotPose`, `ToolpathPoint`, `FrameId`
- `quaternion_to_euler_zyx`, `euler_zyx_to_quaternion`
- Generic conversion utilities

### What stays in core/
- `DatabaseResource` + `DatabaseInit` trait
- `ActiveSystem` marker
- Networking setup
- Base Bevy plugins

