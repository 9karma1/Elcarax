# ADR-001: Rust baseline

## Decision

Elcarax targets Rust 1.96.0 and Rust 2024 Edition.

## Rationale

The editor should be modern from the start and avoid carrying old compiler compatibility constraints before a public SDK exists.

## Consequences

- `rust-toolchain.toml` pins 1.96.0.
- Workspace packages use `edition = "2024"`.
- Workspace packages use `rust-version = "1.96"`.
