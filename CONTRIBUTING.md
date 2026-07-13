# Contributing to MenSung

Thanks for your interest. MenSung targets resource-constrained deployments
(10-15 year old laptops, 2-4GB RAM, no internet), so contributions that keep
the binary small, dependency-free, and fast on old hardware are especially
welcome. Because this is a clinical decision-support tool, correctness and
the zero false negative policy take priority over everything else -- see
[MEDICAL_DATA_POLICY.md](MEDICAL_DATA_POLICY.md) before touching any drug or
interaction data.

## Prerequisites

- **Rust stable** (1.75+): install via [rustup](https://rustup.rs)
- **cargo-audit** and **cargo-deny**: `cargo install cargo-audit cargo-deny`

## Building

```bash
cargo build --release
```

## Running tests

```bash
# Unit and integration tests across the workspace
cargo test --workspace

# Golden medical test cases (tests/golden_cases.json) run as part of the
# workspace test suite -- do not skip them
cargo test --workspace -- golden

# Lints and formatting
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check

# Dependency security and license checks
cargo audit
cargo deny check
```

### Fuzzing

Code that parses untrusted input (the `.men` reader today, the builder's
importers later) carries a `cargo-fuzz` target under `fuzz/`. It needs a
nightly toolchain:

```bash
rustup install nightly
cargo install cargo-fuzz
cargo +nightly fuzz run parse_men
```

`mensung-db` also has a dependency-free crash test that mutates every bit of
a valid database and truncates it to every possible length, run as part of
`cargo test --workspace`. Treat a crash found by either the fuzzer or that
test with the same priority as a golden case failure.

## Workspace layout

| Crate | Responsibility |
|-------|----------------|
| `mensung-domain` | Drug entities, interaction models, severity rules, validation logic. No I/O, no UI, no dependency on any other workspace crate. |
| `mensung-db` | Binary `.men` database reader, zero-copy access, checksum validation. Depends on `mensung-domain` for shared value types (`DrugId`, `Severity`, and so on). |
| `mensung-core` | Lookup engine, fuzzy matcher, business rules. Depends on `mensung-domain` and `mensung-db`, since a lookup has to read the one database format this project ships. |
| `mensung-builder` | OpenFDA/RxNorm/WHO importers, parser, database compiler. |
| `mensung-client` | CLI and TUI. The only crate allowed to depend on `ratatui`/`crossterm`. |

Dependency direction is one-way toward `mensung-domain`: it never depends on
anything else in the workspace, so it never knows about the filesystem, the
database format, or the UI. `mensung-core` depends directly on `mensung-db`'s
concrete reader rather than through a trait-based port, deliberately: there
is exactly one `.men` implementation in this project's scope, and a
lifetime-generic port abstraction over `mensung-db`'s zero-copy return types
would add real complexity for no adapter it would ever actually swap in. If
a second storage backend is ever justified, introduce the port then, against
a real second implementation, not speculatively now.

## Rust engineering rules

- No `unwrap()` outside of tests. Use `Result<T, E>` and `thiserror` in
  library crates; reserve `anyhow` for the CLI boundary.
- No `unsafe` unless justified, documented inline, isolated to a small
  module, and covered by tests.
- No global mutable state (`static mut`, etc).
- The lookup path must not allocate heavily, touch the filesystem, or make
  network calls -- there should be none to make, since MenSung is fully
  offline.
- Prefer borrowing over cloning; prefer iterators over collecting
  intermediate `Vec`s when a single pass will do.

## Supported targets

| Target | Status |
|--------|--------|
| `x86_64-unknown-linux-musl` | CI-tested, statically linked |
| `x86_64-pc-windows-msvc` | CI-tested |
| `x86_64-apple-darwin` | CI-tested |

## PR guidelines

- **CI must pass.** Every PR runs `cargo fmt --check`, `cargo clippy -- -D
  warnings`, `cargo test --workspace`, `cargo audit`, and `cargo deny check`.
  Run them locally before pushing to avoid round-trips.
- **One concern per PR.** A bug fix and a refactor go in separate PRs.
- **Include a test** for any new behavior. Any change touching interaction
  data must update or add a case in `tests/golden_cases.json`.
- **No new runtime dependencies** unless strictly necessary, and never a
  dependency that requires network access or a GPU.
- **Commit messages**: use `type: subject` format (`feat:`, `fix:`, `docs:`,
  `chore:`, `test:`, `refactor:`).

## License

By contributing you agree your work is released under either the
[MIT License](LICENSE-MIT) or the [Apache License 2.0](LICENSE-APACHE), at
the user's option.
