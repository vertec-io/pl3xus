# Performance Reference

Performance characteristics and optimization strategies for pl3xus contributors.

---

## Key Performance Characteristics

### Per-Frame Batching ✅

The system already batches efficiently:

- `broadcast_component_changes` runs **once per frame**
- All changes collected into `HashMap<ConnectionId, Vec<SyncItem>>`
- **One `SyncBatch` message per connection per frame**
- At 60 FPS: 60 messages/second per client

```rust
// From pl3xus_sync/src/subscription.rs
for (connection_id, items) in per_connection {
    if items.is_empty() {
        continue;
    }
    let batch = SyncBatch { items };  // ONE batch per connection per frame
    let _ = net.send(connection_id, SyncServerMessage::SyncBatch(batch));
}
```

---

## Message Conflation

### What It Does

Conflation limits update rate per component type:

```rust
SyncSettings {
    max_update_rate_hz: Some(30.0),  // Max 30 updates/second
    ..default()
}
```

### How It Works

1. Each component type has a `last_sent` timestamp
2. If less than `1/rate` seconds since last send, update is deferred
3. Deferred updates are replaced by newer values (latest-wins)
4. Ensures bounded message rate regardless of change frequency

### When to Use

- High-frequency sensor data (100+ Hz sources)
- Smooth animations where intermediate frames can be skipped
- Bandwidth-constrained environments

### When NOT to Use

- Critical state changes that must not be lost
- Event-driven data (use events, not component sync)
- Low-frequency updates (< 30 Hz)

---

## Report-by-Exception

The system uses report-by-exception by default:

- Only **changed** components are sent
- Bevy's change detection identifies modifications
- Unchanged components generate zero network traffic

```rust
// Only entities with changed Position are included
for (entity, position) in query.iter() {
    if position.is_changed() {
        items.push(SyncItem::Update { entity, data });
    }
}
```

---

## Throughput Optimization

### Current Architecture

Sequential socket writes in send loop:

```rust
while let Ok(message) = messages.recv().await {
    let encoded = bincode::serde::encode_to_vec(&message, ...)?;
    write_half.write_all(&buffer).await?;  // Blocks until complete
}
```

### Potential Bottlenecks

1. **TCP backpressure** - Client receive buffer full
2. **High latency networks** - Each write awaits completion
3. **Large message counts** - Many small messages less efficient than few large

### Optimization Strategies

1. **Increase batch size** - Fewer, larger messages
2. **Reduce update rate** - Use conflation for high-frequency data
3. **Client-side buffering** - Ensure client processes fast enough
4. **Connection pooling** - For multi-client scenarios

---

## Benchmarking

### Key Metrics

| Metric | Target | How to Measure |
|--------|--------|----------------|
| Message latency | < 16ms | Timestamp in message, measure RTT |
| Messages/second | 60/client | Count SyncBatch sends |
| Bytes/second | Varies | Network monitoring |
| CPU usage | < 10% | System monitoring |

### Test Scenarios

```bash
# Run server with many entities
cargo run --release -p control-demo-server -- --entities 1000

# Monitor performance
# (Use system tools: htop, iotop, netstat)
```

---

## Memory Considerations

### Server-Side

- `SubscriptionManager`: O(clients × subscriptions)
- `SyncRegistry`: O(component types)
- Per-frame allocations: `Vec<SyncItem>` per batch

### Client-Side

- `component_data`: O(entities × components)
- Typed signals: O(subscribed entities)
- Deserialization: Temporary allocations per update

### Optimization Tips

- Limit subscribed component types per client
- Use entity-specific subscriptions for large worlds
- Consider pagination for entity lists

---

## Delta Compression (Future)

Currently not implemented, but potential optimization:

- Send only changed fields within components
- Requires schema awareness
- Significant complexity increase
- Consider only if bandwidth becomes bottleneck

---

## Related Documentation

- [Server Setup Guide](../core/guides/server-setup.md) - SyncSettings configuration
- [Architecture Reference](./architecture.md) - System design
- [Subscriptions Guide](../core/guides/subscriptions.md) - Subscription patterns

---

**Last Updated**: 2025-12-07

