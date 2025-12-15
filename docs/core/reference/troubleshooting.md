# Troubleshooting

Common issues and solutions for pl3xus.

## Connection Issues

### Client Can't Connect to Server

**Symptoms**: Client hangs or times out when connecting.

**Solutions**:

1. **Check server is running**
   ```bash
   # Verify server is listening
   netstat -tlnp | grep 3000
   ```

2. **Verify URL/address**
   - TCP: `"127.0.0.1:3000".parse().unwrap()`
   - WebSocket: `url::Url::parse("ws://127.0.0.1:3000").unwrap()`

3. **Check firewall**
   ```bash
   # Allow port (Linux)
   sudo ufw allow 3000
   ```

4. **For WASM clients**
   - Must use WebSocket, not TCP
   - Server must support WebSocket connections

### Connection Drops Immediately

**Symptoms**: `NetworkEvent::Connected` followed immediately by `NetworkEvent::Disconnected`.

**Solutions**:

1. **Check for panics** - Look for panic messages in server logs
2. **Verify message types match** - Both sides must register same types
3. **Check for version mismatch** - Use matching pl3xus versions

## Message Issues

### Messages Not Received

**Symptoms**: `send()` succeeds but handler never runs.

**Solutions**:

1. **Register the message type**
   ```rust
   // Both client AND server must register
   app.register_network_message::<MyMessage, TcpProvider>();
   ```

2. **Add the handler system**
   ```rust
   app.add_systems(Update, handle_my_messages);
   ```

3. **Read events correctly**
   ```rust
   fn handle_my_messages(
       mut messages: MessageReader<NetworkData<MyMessage>>,  // Not EventReader!
   ) {
       for msg in messages.read() {
           // Handle message
       }
   }
   ```

4. **Check connection is established**
   ```rust
   fn check_connection(mut events: MessageReader<NetworkEvent>) {
       for event in events.read() {
           if let NetworkEvent::Connected(id) = event {
               info!("Connected: {:?}", id);
           }
       }
   }
   ```

### Duplicate Message Registration Panic

**Symptoms**: Panic with "already registered" message.

**Solution**: Only register each message type once:

```rust
// WRONG - registering twice
app.register_network_message::<MyMessage, TcpProvider>();
app.register_network_message::<MyMessage, TcpProvider>();  // Panic!

// RIGHT - register once
app.register_network_message::<MyMessage, TcpProvider>();
```

## Sync Issues

### Component Not Syncing

**Symptoms**: Component changes on server don't appear on client.

**Solutions**:

1. **Register on server**
   ```rust
   app.sync_component::<Position>(None);
   ```

2. **Register in client registry**
   ```rust
   let registry = ClientTypeRegistry::builder()
       .register::<Position>()
       .build();
   ```

3. **Subscribe in component**
   ```rust
   let positions = use_sync_component::<Position>();
   ```

4. **Check type names match** - Full type path must be identical

### Mutations Not Working

**Symptoms**: `use_sync_component_write` doesn't update server.

**Solutions**:

1. **Enable mutations on server**
   ```rust
   app.sync_component::<Position>(Some(SyncSettings {
       allow_mutations: true,
       ..default()
   }));
   ```

2. **Check authorization** - Custom authorizers may reject mutations

## Build Issues

### WASM Build Fails

**Symptoms**: Compilation errors when targeting `wasm32-unknown-unknown`.

**Solutions**:

1. **Use WebSocket transport**
   ```rust
   // TCP doesn't work in WASM
   use pl3xus_websockets::WebSocketProvider;
   ```

2. **Add WASM target**
   ```bash
   rustup target add wasm32-unknown-unknown
   ```

3. **Check dependencies** - Some crates don't support WASM

### Rust Nightly Required

**Symptoms**: Compilation fails with edition or feature errors.

**Solution**: Create `rust-toolchain.toml`:
```toml
[toolchain]
channel = "nightly"
```

## Performance Issues

### High Latency

**Solutions**:

1. **Enable conflation** - Reduces message frequency
2. **Batch messages** - Use `OutboundMessage` pattern
3. **Reduce sync rate** - Configure `SyncSettings`

### Memory Growth

**Solutions**:

1. **Add `pl3xus_memory` plugin** - Monitors and cleans up
2. **Check for event accumulation** - Read all events each frame
3. **Disconnect stale clients** - Implement heartbeat/timeout

## Getting Help

- **Discord**: [Bevy Discord](https://discord.gg/bevy) - look for `@SirCarter`
- **Issues**: [GitHub Issues](https://github.com/jamescarterbell/pl3xus/issues)

