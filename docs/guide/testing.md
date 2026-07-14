# Testing Picus applications

Prefer headless Bevy tests for projection, actions, and invalidation. Build an
`App` with `PicusPlugin`, register actions and components exactly as production
does, then advance the relevant schedules. Use `MessageReader<UiAction<T>>` in
test systems or collect messages into a test resource before an exclusive
assertion system. Applications must not reach into `InternalUiEventQueue`.

Useful assertions include:

- an input action is visible to an `Update` reader in the same frame;
- a sender emission from `Update` is visible on the next frame;
- multiple readers each receive one copy of a message;
- missing or partial themes remain transparent and do not fail loading;
- a component resource change dirties the expected projection root;
- a failed `present()` does not mark a window as painted.

Run focused tests while iterating, then validate the public facade and every
example:

```text
cargo fmt --all -- --check
cargo test -p picus_core
cargo test -p picus --test ui
cargo check --workspace --all-targets
```

Integration and CodeWhale tests use temporary fixtures. They must never read or
write the developer's real `~/.codewhale/` state.
