# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.1.0] - 2025-11-09

### Changed - Bevy 0.17 Upgrade

#### ⚠️ Breaking Changes

**Bevy 0.17.2 Upgrade** - The library has been upgraded to support Bevy 0.17.2, which includes significant breaking changes to the event system API.

**Event → Message API Migration**
- All `#[derive(Event)]` changed to `#[derive(Message)]`
- All `EventReader<T>` changed to `MessageReader<T>`
- All `EventWriter<T>` changed to `MessageWriter<T>`
- All `App::add_event::<T>()` changed to `App::add_message::<T>()`
- `MessageWriter::send()` method renamed to `MessageWriter::write()`

**Affected Types:**
- `NetworkEvent` - Now derives `Message` instead of `Event`
- `NetworkData<T>` - Now derives `Message` instead of `Event`
- `OutboundMessage<T>` - Now derives `Message` instead of `Event`

**Rust Version Requirement**
- Minimum Rust version: 1.88.0 (currently requires nightly Rust 1.93.0)
- Rust edition: 2024

#### Migration Guide

See [docs/MIGRATION_0.17.md](docs/MIGRATION_0.17.md) for detailed migration instructions.

#### Dependencies

- Updated `bevy` from 0.16.2 to 0.17.2
- All Bevy-related dependencies updated to 0.17.2

#### Internal Changes

- Updated all internal systems to use `MessageReader` and `MessageWriter`
- Updated all examples to use the new Message API
- Fixed clippy warnings:
  - Added `Default` implementation for `PreviousMessage<T>`
  - Changed `len() > 0` to `!is_empty()`
  - Collapsed nested if statements
  - Simplified format! calls
  - Removed unnecessary borrows

---

## [0.10.0] - 2025-01-XX

### Added

#### 🎉 Automatic Message Registration (Major Feature)

- **New `Pl3xusMessage` trait** - Automatically implemented for all `Serialize + Deserialize + Send + Sync + 'static` types
- **New `register_network_message<T>()`** - Register any serializable type as a network message without implementing `NetworkMessage`
- **Automatic type name generation** - Uses `std::any::type_name()` with caching for performance
- **New `send<T>()`** - Simplified send method that works with any `Pl3xusMessage` type
- **Updated `broadcast<T>()`** - Now works with any `Pl3xusMessage` type
- **External crate support** - Use types from any crate as network messages without wrapper types
- **Helper methods** - Added `is_message_registered()` and `registered_message_names()` for testing/debugging

#### Examples & Documentation

- **New `automatic_messages` example** - Complete working example demonstrating the new API
- **Comprehensive README** - Added examples/README.md with usage patterns and migration guide
- **Updated main README** - Showcases new API and includes migration guide

#### Tests

- **12 new integration tests** - Comprehensive test coverage for automatic message registration
- **Unit tests** - Tests for type name generation and caching behavior

### Changed

- **Improved ergonomics** - Reduced boilerplate by eliminating need for `NetworkMessage` trait in most cases
- **Better error messages** - More descriptive error messages for registration failures

### Deprecated

- `listen_for_message<T>()` - Use `register_network_message<T>()` instead (still fully functional)
- `send_message()` - Use `send()` instead (still fully functional)

**Note:** Deprecated methods are still fully supported and will continue to work. They are useful when you need explicit control over message names (e.g., for versioning).

### Fixed

- **Binary codec decode** - Fixed decode implementation in `pl3xus_common` that was not properly handling length prefix

### Migration Guide

#### For New Code

Use the new automatic API:

```rust
// Before (0.9)
impl NetworkMessage for MyMessage {
    const NAME: &'static str = "my:Message";
}
app.listen_for_message::<MyMessage, TcpProvider>();
net.send_message(conn_id, msg)?;

// After (0.10)
#[derive(Serialize, Deserialize, Clone)]
struct MyMessage { /* fields */ }

app.register_network_message::<MyMessage, TcpProvider>();
net.send(conn_id, msg)?;
```

#### For Existing Code

No changes required! The old API continues to work:

```rust
// This still works exactly as before
app.listen_for_message::<MyMessage, TcpProvider>();
net.send_message(conn_id, msg)?;
```

To remove deprecation warnings, update to the new API:

```rust
app.register_network_message::<MyMessage, TcpProvider>();
net.send(conn_id, msg)?;
```

#### When to Use Each API

- **Use `register_network_message()`** - For most use cases, especially with external types
- **Use `listen_for_message()`** - When you need explicit message names (e.g., `"auth:v2:Login"` for versioning)

### Performance

- **Zero runtime overhead** - Type names are cached using `OnceCell`, computed once and reused
- **Same performance as const str** - After first access, no performance difference from explicit names

### Breaking Changes

None! This release is fully backward compatible.

### Version Updates

- `pl3xus`: 0.9.11 → 0.10.0
- `pl3xus_common`: 0.2.8 → 0.3.0
- `pl3xus_websockets`: 0.2.1 → 0.3.0

---

## [0.9.11] - Previous Release

See git history for previous changes.

