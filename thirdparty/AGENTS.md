# thirdparty — agent rules

## General

- Prefer vendoring or submodule pins over copying large third-party trees into
  `crates/`.
- When adding a new third-party dependency that ships its own agent rules, add a
  nested note here rather than editing the vendor tree.
