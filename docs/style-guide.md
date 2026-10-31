# Style Guide

Clean code principles for the codebase.

## Naming

- Use descriptive, intention-revealing names. A reader should understand what a variable, function, or type does without needing to read its implementation.
- Avoid abbreviations unless they are universally understood within the domain (e.g., `pid` for a PID controller is fine, `ctrl` for controller is not).
- Boolean variables and functions should read as predicates: `is_enabled`, `has_converged`, `should_reset`.
- Name functions after what they do, not how they do it: prefer `clamp_to_limits` over `check_and_adjust_value`.

## Functions

- Functions should do one thing. If a function does two distinct things, split it.
- Keep functions short. If a function exceeds ~40 lines, look for extraction opportunities.
- Prefer fewer arguments. If a function needs many parameters, group related ones into a struct.
- Avoid flag arguments (booleans that switch behavior). Write two functions with clear names instead.
- Functions should operate at a single level of abstraction. Don't mix high-level orchestration with low-level bit manipulation in the same function.

## Error Handling

- Use `Result<T, E>` for operations that can fail at runtime. Reserve panics (`assert!`, `unwrap`) for programming errors and contract violations that indicate bugs.
- Define domain-specific error types with `thiserror`. Avoid stringly-typed errors.
- Handle errors at the appropriate level — don't propagate errors further than necessary, but don't swallow them either.
- Prefer `?` over `unwrap()` in fallible code paths. Every `unwrap()` should have a comment explaining why it cannot fail, or be replaced with `expect("reason")`.

## Comments

- Don't comment *what* the code does — make the code itself clear enough. Comment *why* non-obvious decisions were made.
- Delete commented-out code. Version control remembers it.
- Use `///` doc comments on all public items. Focus on what callers need to know, not implementation details.
- Use `// TODO:` for known improvements. Include enough context that someone else could act on it.

## Structure and Organization

- One `pub struct` or `pub enum` per file. Private helpers are fine alongside.
- Group related files using directory modules. Re-export public types from `<module>.rs` so consumers don't need to know the internal file layout.
- Order items within a file: types, constructors, public methods, private methods, trait implementations, tests.

## Type Design

- Make illegal states unrepresentable. Use enums and newtypes to encode invariants in the type system rather than relying on runtime checks.
- Prefer strong types over primitives: `Frequency(f64)` is clearer than a bare `f64` that might be Hz or rad/s.
- Keep structs focused. If a struct has fields that are only used together in a subset of methods, it may need to be split.

## Testing

- Name tests after the behavior they verify: `test_controller_clamps_output_to_limits`, not `test1`.
- Follow the Arrange-Act-Assert pattern. Separate setup, execution, and verification with blank lines.
- Test behavior, not implementation. Tests should survive internal refactors.
- Use `#[should_panic]` tests to verify that contract violations are caught.
- Prefer real values over mocks. When a dependency is cheap to construct, use the real thing.

## Documentation

- Every crate has a `README.md` at its root. This is the high-level entry point: purpose, design diagrams, usage examples, and a types table linking to per-file docs where they exist.
- Individual source files that are complex enough to warrant more explanation get their own doc file. Name it after the source file (e.g., `equipment.rs` → `equipment.md`) and place it alongside the source file.
- Reference per-file docs from the crate `README.md` so readers can navigate from the overview to the detail.
- Use [Mermaid](https://mermaid.js.org/) diagrams in markdown docs where they aid understanding — class diagrams for type relationships, state diagrams for lifecycle, flowcharts for branching logic. Prefer diagrams over prose for structural or state information.
- Simple files (e.g., a small error enum) do not need their own doc file — cover them in the crate `README.md`.

## Dependencies

- Minimize external dependencies. Each dependency is a maintenance and security liability.
- Vet new crates before adding them. Prefer well-maintained crates with a clear purpose over kitchen-sink libraries.
- Pin dependency versions in `Cargo.toml` to avoid surprise breakage.

## General Principles

- **Clarity over cleverness.** Code is read far more than it is written. Optimize for the reader.
- **Don't repeat yourself, but don't abstract prematurely.** Duplication is cheaper than the wrong abstraction. Wait until you see a clear pattern before extracting.
- **Leave the code better than you found it.** Small, incremental improvements compound over time.
- **Minimize scope.** Variables, functions, and types should have the smallest visibility and lifetime possible.
