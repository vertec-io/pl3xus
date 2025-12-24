# Fanuc RMI Replica - Implementation Plan

**Purpose**: Step-by-step guide for completing all outstanding tasks.

---

## Phase 1: Code Quality (~20 min)

### 1.1 Fix Unused Variable Warning

**File**: `examples/fanuc_rmi_replica/client/src/pages/dashboard/control/program_display.rs`

```bash
# Find the line
grep -n "system_entity_bits" examples/fanuc_rmi_replica/client/src/pages/dashboard/control/program_display.rs
```

**Fix**: Prefix with underscore `_system_entity_bits` or remove if truly unused.

### 1.2 Fix Dead Code Warnings

**Files**: 
- `examples/fanuc_rmi_replica/server/src/plugins/execution.rs`
- `examples/fanuc_rmi_replica/server/src/plugins/program.rs`

**Options**:
1. Add `#[allow(dead_code)]` if code will be used later
2. Remove if not needed
3. Implement the functionality if it should be active

**Recommended**: Check if these are part of program execution that's WIP. If so, use `#[allow(dead_code)]`.

### 1.3 Verify Clean Build

```bash
cargo check --package fanuc_replica_server --package fanuc_replica_client 2>&1 | grep -E "warning:|error:"
```

---

## Phase 2: API Consistency (~4 hr)

### 2.1 Convert Commands to Targeted Requests (1 hr)

**Goal**: Make robot commands use `TargetedRequestMessage` pattern.

**Files to Modify**:
- `examples/shared/fanuc_replica_types/src/lib.rs` - Add `TargetedRequestMessage` impl
- `examples/fanuc_rmi_replica/server/src/plugins/requests.rs` - Register as targeted
- `examples/fanuc_rmi_replica/client/src/pages/dashboard/control/quick_commands.rs` - Use `use_mutation_targeted`

**Reference Pattern** (existing):
```rust
// Type definition
impl TargetedRequestMessage for SetSpeedOverride {
    type ResponseMessage = SetSpeedOverrideResponse;
}

// Server registration
app.targeted_request::<SetSpeedOverride, WS>().register();

// Client usage
let speed_override = use_mutation_targeted::<SetSpeedOverride>(callback);
speed_override.send_targeted(entity_bits, request);
```

**Commands to Convert**:
- [ ] `SetSpeedOverride`
- [ ] `InitializeRobot`
- [ ] `AbortMotion`
- [ ] `ResetRobot`

### 2.2 Add Targeting to Program Commands (1 hr)

**Same pattern as 2.1 for**:
- [ ] `StartProgram`
- [ ] `PauseProgram`
- [ ] `ResumeProgram`
- [ ] `StopProgram`
- [ ] `LoadProgram`

**Note**: These are already using `use_mutation_targeted` on client - verify server registration matches.

### 2.3 Make ConnectToRobot Targeted (1 hr)

**Challenge**: Robot entities need to exist before connection.

**Current Flow**:
1. Client sends `ConnectToRobot { connection_id }`
2. Server creates robot entity and connects

**New Flow**:
1. Robot entities loaded from DB at startup (pre-existing)
2. Client sends `TargetedMessage<ConnectToRobot>` to specific robot entity
3. Server connects that entity

**Implementation**:
1. Create robot entities from `robot_connections` DB table on server startup
2. Each entity has `RobotConnectionDetails` but no driver
3. `ConnectToRobot` becomes targeted, connects the specific entity

### 2.4 Review ControlRequest Pattern (30 min)

**File**: `examples/shared/fanuc_replica_types/src/lib.rs`

**Current**:
```rust
pub enum ControlRequest {
    RequestControl { entity_bits: u64, client_name: String },
    ReleaseControl { entity_bits: u64 },
}
```

**Consider**: Using `TargetedMessage<RequestControl>` instead of embedding entity_bits.

---

## Phase 3: UX Improvements (~2 hr)

### 3.1 Program State Persistence (2 hr)

**Goal**: Remember open program when navigating away and back.

**Files**:
- `examples/fanuc_rmi_replica/client/src/pages/programs/mod.rs`
- `examples/fanuc_rmi_replica/client/src/app.rs` (router)

**Implementation Options**:

**Option A: URL Params (Recommended)**
```rust
// Route: /programs/:id
<Route path="/programs/:id" view=ProgramDetails />

// Navigate with ID
use_navigate()("/programs/42", Default::default());
```

**Option B: Context Signal**
```rust
// In app-level context
#[derive(Clone)]
pub struct ProgramContext {
    pub selected_program_id: RwSignal<Option<i64>>,
}
```

---

## Phase 4: Future Enhancements (~6 hr)

### 4.1 Server-Side Missing Subscription Warnings (2 hr)

**Files**:
- `crates/pl3xus_sync/src/sync.rs` (or wherever subscriptions are handled)

**Implementation**:
1. When client subscribes to entity/component, check existence
2. If not found, send warning message to client
3. Client logs warning in console

### 4.2 I/O Display Name Configuration (3 hr)

**Files**:
- `examples/shared/fanuc_replica_types/src/lib.rs` - Add to `IoConfigState`
- `examples/fanuc_rmi_replica/client/src/components/io_status.rs` - Display custom names
- `examples/fanuc_rmi_replica/server/src/plugins/io.rs` - Store/load names

### 4.3 Check JogDefaultStep Units (30 min)

**Reference**: `/home/apino/dev/Fanuc_RMI_API`

```bash
grep -r "JogDefaultStep\|jog.*speed\|joint.*step" /home/apino/dev/Fanuc_RMI_API --include="*.rs"
```

---

## Verification Checklist

After each phase:

```bash
# Build check
cargo check --package fanuc_replica_server --package fanuc_replica_client

# Run and test
cd examples/fanuc_rmi_replica/server && cargo run &
cd examples/fanuc_rmi_replica/client && trunk serve
```

- [ ] No compiler warnings
- [ ] No runtime errors in console
- [ ] Features work as expected
- [ ] Update STATUS documents

