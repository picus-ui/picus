# Synchronizing the CodeWhale submodule

`thirdparty/CodeWhale` is a git submodule. Routine flow: fetch remotes, merge
`upstream/main` inside the submodule, resolve conflicts, verify both workspaces,
commit submodule then pointer in picus.

## Cargo.toml conflict resolution

1. Keep `.workspace = true` for every internal `codewhale-*` dependency.
2. Hardcode `version` / `license` / `repository` per crate (do not inherit from picus).
3. For `Cargo.lock` inside the submodule: take upstream’s (`git checkout --theirs Cargo.lock`).

## Update **both** root manifests when upstream adds a crate

- CodeWhale root: `thirdparty/CodeWhale/Cargo.toml` → `workspace.dependencies`
- Picus root: `Cargo.toml` → `members` + `workspace.dependencies`

## Verify

```text
# picus
cargo check --workspace && cargo test -p example_picuscode
# CodeWhale standalone
cd thirdparty/CodeWhale && cargo check -p codewhale-cli && cargo check -p codewhale-tui
```

Hard process rules for agents also appear under `thirdparty/` / root `AGENTS.md`
where they must remain executable without opening this guide.
