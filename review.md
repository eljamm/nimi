**The issue:** The patch implements service startup ordering, but conflates two distinct concepts — spawn ordering (process exists) and readiness ordering (process is ready to serve). The signal fires on spawn, so dependents start immediately regardless of whether the dependency is actually ready. The log output claiming "dependencies satisfied" makes this silent and misleading.

**The design approach:** Split the concepts explicitly. Spawn ordering (`after`) stays as-is — it's useful and correct for what it is. Readiness ordering (`afterReady`) becomes a separate option that waits for a readiness probe declared on the *dependency's* service definition, not on the waiting service. This keeps the two concerns in the right places: the dependency knows how to signal its own readiness, and the dependent just says "wait for it."

A service with no readiness probe simply cannot be used as an `afterReady` target — which is honest, since a one-shot like `hello` has no ready state. The Nix module should enforce this with an assertion.

The implementation delta over the current patch is modest: add an optional `readyCheck` field to the service definition, and change `started_signal` to fire after the check passes rather than after spawn. The `watch` channel mechanism stays the same.
