# December 2025 Session: Next Steps

## Recommended Approach for Continuation

### Step 1: Verify Current State Works

Before making changes, verify the application is functional:

```bash
# Terminal 1: Start FANUC simulator (optional - for real robot testing)
cd /home/apino/dev/Fanuc_RMI_API && cargo run -p sim -- --realtime

# Terminal 2: Start server
cd examples/fanuc_rmi_replica/server && cargo run

# Terminal 3: Start client
cd examples/fanuc_rmi_replica/client && trunk serve
```

**Test the following**:
1. Open http://localhost:8080
2. Click "Settings" or "Connections" in top bar
3. Click "+" to add a robot connection
4. Enter connection details (IP: localhost, port: 60008 for simulator)
5. Save, then click "Connect" button
6. Verify:
   - "Connecting..." shows while connecting
   - Connection state updates on success
   - Quick commands become enabled
   - Position display shows robot position

### Step 2: Fix position_display.rs (Priority 1)

This is the most critical remaining bug. See `OUTSTANDING_TASKS.md` for details.

```bash
# Find the file
view examples/fanuc_rmi_replica/client/src/pages/dashboard/control/position_display.rs

# Apply the same pattern used in other files
```

### Step 3: Audit use_components Usage (Priority 1)

```bash
# Find all usages
grep -rn "use_components::<" examples/fanuc_rmi_replica/client/src

# Review each one - most should be use_entity_component
```

### Step 4: Choose Direction for Remaining Work

**Option A: Complete API Migration**
Continue converting commands to targeted requests (Tasks #3, #4). This makes the API more consistent but requires server-side changes too.

**Option B: Architecture Refactor**
Work on Task #7 (Consolidate Robot Entity Architecture). This provides stable entity IDs and cleaner data flow, but is a larger change.

**Option C: UX Polish**
Focus on Task #5 (Program State Persistence) and Task #6 (Server Notifications). These improve user experience without major architectural changes.

**Recommendation**: Start with Option A (API migration) because:
1. It's well-defined work
2. It builds on patterns already established
3. It improves consistency without breaking changes

---

## Quick Reference: Common Patterns

### Subscribing to Robot Components

```rust
// Get entity context
let system_ctx = use_system_entity();

// Subscribe to robot component
let (connection_state, robot_exists) = use_entity_component::<ConnectionState, _>(
    move || system_ctx.robot_entity_id.get()
);

// Check both existence and state
let robot_connected = Memo::new(move |_| 
    robot_exists.get() && connection_state.get().robot_connected
);
```

### Sending Targeted Messages

```rust
// Get send function
let send_jog = use_send_targeted::<JogCommand>();

// Get target entity
let system_ctx = use_system_entity();

// Send
send_jog(
    system_ctx.robot_entity_id.get().unwrap(),
    JogCommand { ... }
);
```

### Using Queries

```rust
// Simple query (fetches on mount)
let programs = use_query::<ListPrograms>();

// Keyed query (fetches when key is Some)
let program = use_query_keyed::<GetProgram, _>(move || {
    selected_id.get().map(|id| GetProgram { id })
});

// Access data
if programs.is_loading() { /* show spinner */ }
if let Some(data) = programs.data() { /* render data */ }
if let Some(err) = programs.error() { /* show error */ }
```

### Using Mutations

```rust
// Create mutation with callback
let toast = use_toast();
let create_program = use_mutation::<CreateProgram>(move |result| {
    match result {
        Ok(r) if r.success => toast.success("Created!"),
        Ok(r) => toast.error(format!("Failed: {}", r.error.unwrap_or_default())),
        Err(e) => toast.error(format!("Error: {e}")),
    }
});

// Send mutation
create_program.send(CreateProgram { name: "test".to_string() });
```

---

## Files Likely to Change

| Task | Files |
|------|-------|
| Position display fix | `control/position_display.rs` |
| use_components audit | Various client files |
| Targeted commands | `server/src/plugins/requests.rs`, `client/src/pages/dashboard/control/quick_commands.rs` |
| Program state | `client/src/pages/dashboard/programs/mod.rs`, `client/src/pages/dashboard/context.rs` |

