# Rust Cargo Workspace

Monorepo of core libraries and applications. Rust workspace with Nix flakes for the dev environment.

Crates live under `crates/`, Nix config under `nix/`. See `Cargo.toml` for workspace members.

## Commands

- `cargo nextest run` — run tests
- `cargo clippy` — lint
- `nix fmt` — format everything (Rust, Nix, shell, markdown)

## Code Style

See [STYLE_GUIDE.md](STYLE_GUIDE.md) for the full style guide covering clean code principles, naming, error handling, type design, and testing practices.

Key points:

- One `pub struct` or `pub enum` per file. Private supporting types/impls are fine alongside.
- Use directory modules to group related files. Re-export public types from parent modules (e.g., `pub use submodule::MyType;`).
- Keep files small and focused — single responsibility per file.
- Write tests for new functionality.

## Conventions

- **Thread safety**: Stateful types use `parking_lot::Mutex` with an inner struct pattern (e.g., `Example { inner: Mutex<ExampleInner> }`). `parking_lot` locks don't poison — just call `.lock()` directly.
- **Builder pattern**: Chain configuration via `with_*` methods (e.g., `Example::new(param).with_additional_param(additional_param)`).
- **Error handling**: Use `Result<T, E>` with `thiserror` error types for validation that could receive user input at runtime. Reserve `assert!` for internal invariants that indicate programming bugs. Reserve `tracing::warn!` for runtime edge cases (e.g., Nyquist violations).
- **Testing**: Integration tests live in `crates/*/tests/`. Cover happy paths, edge cases, convergence behavior, and `#[should_panic]` for contract violations.
- **No CI**: Tests and linting are run locally via the commands above.
