# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Tick (`tk`) is a CLI for managing a PARA-method note system (Projects/Areas/Resources/Archive). See [README.md](README.md) for user-facing behavior and the full command reference, and [docs/design.md](docs/design.md) for the module architecture. Do not duplicate details from those files here — link to them instead.

`init`, `new`, `daily`, `list`, `status`, `config`, `mv`/`move`, `archive`, `unarchive`, `review`, and `completions` (including item-name tab-completion) are all implemented — the design in `docs/design.md` and the acceptance criteria in `docs/user-stories/` describe their architecture and behavior. When implementing a new command or story, follow the module boundaries in `docs/design.md` exactly — the separation between filesystem/business logic (`workspace`, `items`, `review`, `editor`) and terminal I/O (`cli`) is the core design constraint of this codebase, chosen specifically so the former can be unit-tested without a real shell, editor, or terminal.

## Commands

```
cargo build
cargo run -- <args>       # binary is `tk`, e.g. cargo run -- init my-para
cargo test
cargo test <name>         # run a single test by name/substring
cargo clippy
cargo fmt
```

## Tooling

Use the `LSP` tool (rust-analyzer) for Rust code navigation instead of grepping — `goToDefinition`, `findReferences`, `hover`, `documentSymbol`, `workspaceSymbol`, `goToImplementation`, and call hierarchy all work across the crate. Prefer it when tracing how a module's contract (from `docs/design.md`) is used, finding all call sites before changing a signature, or checking a type's implementations.

## Development approach

- **Use the `rust-skills` skill (`/rust-skills`) for all Rust implementation work.** It covers ownership, error handling, async patterns, API design, memory optimization, performance, testing, and common anti-patterns — apply it when writing, reviewing, or refactoring any Rust code in this repo.
- **TDD is the expected workflow.** Write the test for a unit of behavior first (using the acceptance criteria in `docs/user-stories/` where applicable), watch it fail, then implement. Because `items`, `workspace`, `review`, and `editor` are designed to be pure/mockable (no direct terminal I/O), they should be tested directly without needing to shell out to `tk` or fake a real `$EDITOR`.
- User stories in `docs/user-stories/` are written in Given/When/Then form — treat each scenario as a candidate test case when implementing the command it documents.
- When adding a new command, check `docs/design.md` for which module owns the behavior before writing code — e.g. filesystem changes belong in `items`, prompting/rendering belongs in `cli`, and `cli` itself should stay free of business logic so it remains the only layer that isn't unit-testable directly.
