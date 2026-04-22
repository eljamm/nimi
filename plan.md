# Implementation Plan: Split Spawn vs Readiness Ordering

## Issue
The current patch conflates spawn ordering (process exists) with readiness ordering (process is ready to serve). The signal fires on spawn, so dependents start immediately regardless of whether the dependency is actually ready.

## Solution
Split the concepts:
- `after`: spawn ordering (unchanged) - waits for process to exist
- `afterReady`: readiness ordering (new) - waits for readiness probe

---

## Implementation Steps

### 1. Rust: Add `ready_check` field to Service (`src/process_manager/service.rs`)

```rust
/// Optional readiness probe command - runs after spawn until exit 0
#[serde(rename = "readyCheck")]
pub ready_check: Option<String>,
```

### 2. Rust: ServiceManager - wait for ready check before signaling (`src/process_manager/service_manager.rs`)

In `spawn_service_process()`:
- After `create_service_child()`, run the readiness probe (if configured)
- Only send `started_signal` after probe succeeds

### 3. Rust: ServiceOrdering - add `afterReady` field (`src/config.rs`)

```rust
pub struct ServiceOrdering {
    pub after: Vec<String>,
    /// Services that must be ready before this one starts
    #[serde(default, rename = "afterReady")]
    pub after_ready: Vec<String>,
}
```

### 4. Rust: Update process_manager ordering validation

In `validate_ordering()`, verify that `afterReady` targets have `ready_check` configured.

### 5. Rust: Update spawn logic

In `spawn_child_processes()`:
- `after` uses current `rx.wait_for(|v| *v)` behavior (spawn-ordered)
- `afterReady` needs separate waiting logic (readiness-ordered)

### 6. Nix: Add `afterReady` option (`nix/modules/nimi/ordering.nix`)

```nix
options.afterReady = mkOption {
  description = ''
    List of service names that must be ready before this
    service is spawned. Each target must declare a
    readiness check.
  '';
  type = types.listOf types.str;
  default = [];
};
```

### 7. Nix: Add assertion for `afterReady` targets (`nix/modules/nimi/assertions.nix`)

Verify that every service listed in `afterReady` has `readyCheck` configured.

### 8. Nix: Add `readyCheck` option to service config

In `nix/modules/nimi/service.nix` (or appropriate module):
```nix
options.readyCheck = mkOption {
  description = ''
    Command to run to determine if the service is ready.
    The command should exit 0 when ready.
  '';
  type = types.nullOr types.pathInStore;
  default = null;
};
```

---

## Files to Modify

1. `src/process_manager/service.rs` - add `ready_check` field
2. `src/process_manager/service_manager.rs` - implement readiness probe logic
3. `src/config.rs` - add `afterReady` to `ServiceOrdering`
4. `src/process_manager.rs` - update ordering validation and spawn logic
5. `nix/modules/nimi/ordering.nix` - add `afterReady` option
6. `nix/modules/nimi/assertions.nix` - add readiness assertion
7. `nix/modules/nimi/service.nix` - add `readyCheck` option (if separate)

---

## Backward Compatibility

- Services without `readyCheck` work as before (implicit ready on spawn)
- `after` behavior unchanged for existing configs