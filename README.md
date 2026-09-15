# memoryloom

Local transcript indexing and literal search for agent memory.

## Why this exists

The current project fleet already covers agent frameworks, music software, games, privacy, sync, mobile, and archival tooling. `memoryloom` fills a narrower gap: a small local-first utility that can be audited in one sitting and composed with OpenShark, OpenShield, shell scripts, or other agents.

## v0 scope

- No network access.
- No external Rust dependencies.
- Deterministic output where the filesystem allows it.
- Plain text formats that can be reviewed in Git.
- Real unit tests, not placeholder stubs.

## Commands

```sh
memoryloom index ~/.local/share/openshield --out memory.loom
```
```sh
memoryloom search ~/Projects 'openshark omo handoff'
```
```sh
memoryloom stats ~/Projects
```

## Architecture

`src/main.rs` contains the complete v0 implementation: parsing, validation, pure core functions, CLI dispatch, and unit tests. The next extraction boundary is a `core` module once the format stabilizes; until then, keeping the tape on one reel makes audits cheap.

## Roadmap

- [ ] Recursive text manifest and literal search
- [ ] Incremental indexes with mtime/hash deltas
- [ ] SQLite storage
- [ ] Embedding adapters behind an explicit opt-in

## Development

```sh
cargo fmt --check
cargo test
cargo run -- --help
```

## Safety

Local commits only. Never push or create remotes without explicit instruction. Do not weaken validation to make a failing test pass.

---
Made by [synth](https://github.com/synthalorian) with blackclaw ⚫🦞
