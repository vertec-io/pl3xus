---
title: Contributing to pl3xus
---
# Contributing to pl3xus

This section is for developers contributing to pl3xus itself.

---

## Contents

| Document | Description |
|----------|-------------|
| [Research Process](./research-process.md) | How to conduct and document research |
| [Architecture](./architecture.md) | System architecture reference |
| [Performance](./performance.md) | Performance research and benchmarks |
| [Technical Reference](./technical.md) | Technical deep dives |

---

## Getting Started

### Prerequisites

- Rust nightly (for some async features)
- wasm32-unknown-unknown target (`rustup target add wasm32-unknown-unknown`)
- trunk (for WASM examples): `cargo install trunk`

### Building

```bash
# Check all crates
cargo check --workspace

# Build server examples
cargo build --example basic-server -p pl3xus

# Build WASM client
cd crates/pl3xus_client && trunk build --release
```

### Running Examples

```bash
# Terminal 1: Start server
cargo run -p control-demo-server

# Terminal 2: Start client (browser opens automatically)
cd examples/control-demo/client && trunk serve --open
```

---

## Development Workflow

### 1. Research Phase

Before implementing significant features:

1. Create a research folder: `research/active/[topic-name]/`
2. Document the problem, options, and proposed solution
3. Get review/approval before implementation
4. See [Research Process](./research-process.md) for details

### 2. Implementation Phase

1. Create a feature branch
2. Implement with tests
3. Update documentation
4. Run `cargo fmt && cargo clippy`

### 3. Documentation Phase

1. Update relevant guides in `docs/guides/`
2. Update API documentation (docstrings)
3. Update CHANGELOG.md

### 4. Archive Phase

When research is complete:

1. Move research to `research/archive/YYYY-MM-[topic]/`
2. Create `CONCLUSION.md` from template
3. Update `research/archive/README.md`

---

## Code Style

### Rust

- Use `cargo fmt` before committing
- Address all `cargo clippy` warnings
- Prefer explicit types over `_` in function signatures
- Document all public APIs

### Documentation

- Include code examples in docstrings
- Keep examples minimal but complete

---

## Testing

```bash
# Run all tests
cargo test --workspace

# Run specific crate tests
cargo test -p pl3xus_sync

# Check WASM compilation
cargo check -p pl3xus_client --target wasm32-unknown-unknown
```

---

## Crate Structure

```
pl3xus/
├── crates/
│   ├── pl3xus/           # Core Bevy networking plugin
│   ├── pl3xus_common/    # Shared types and traits
│   ├── pl3xus_sync/      # ECS synchronization (server)
│   ├── pl3xus_client/    # Leptos client library
│   ├── pl3xus_websockets/# WebSocket provider
│   └── pl3xus_memory/    # Memory provider (testing)
├── examples/
│   ├── basic/               # Minimal working example
│   └── control-demo/        # Full-featured demo
├── docs/                    # User documentation
└── research/                # Developer research (git-ignored)
```

---

## Release Process

1. Update version in all `Cargo.toml` files
2. Update CHANGELOG.md
3. Create release notes in `docs/releases/`
4. Tag release: `git tag v0.x.0`
5. Publish crates in dependency order

---

## Questions?

- Check existing issues on GitHub
- Review research documents for design decisions
- Ask in discussions

---

**Last Updated**: 2025-12-07

