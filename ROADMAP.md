# MenSung Roadmap

This roadmap tracks the path from the current scaffold to a first public
release, and beyond. It follows the architecture and requirements set out in
[README.md](README.md), [MEDICAL_DATA_POLICY.md](MEDICAL_DATA_POLICY.md), and
[CONTRIBUTING.md](CONTRIBUTING.md). Nothing here ships until the zero false
negative policy and the golden medical test suite both pass in CI.

## Phase 0: Foundation (done)

- [x] Cargo workspace: `mensung-domain`, `mensung-core`, `mensung-db`, `mensung-builder`, `mensung-client`, one-way dependency direction verified with `cargo check`
- [x] Governance docs: LICENSE-MIT, LICENSE-APACHE, CODE_OF_CONDUCT.md, SECURITY.md, CONTRIBUTING.md, MEDICAL_DATA_POLICY.md
- [x] GitHub templates: PR template, bug report, medical data error, security report
- [x] CI: fmt, clippy `-D warnings`, test, release build across `x86_64-unknown-linux-musl` / `x86_64-pc-windows-msvc` / `x86_64-apple-darwin`, cargo-audit, cargo-deny
- [x] Release workflow: tag-triggered, checksums, GitHub release
- [x] Branch protection on `main`: required status checks (strict), no force-push, no deletion
- [x] README with medical disclaimer, architecture, and performance targets

## Phase 1: Domain Layer (`mensung-domain`) (done)

- [x] Newtype IDs: `DrugId`, `InteractionId` (no bare `u32`/`u64` aliases)
- [x] `Drug` entity: INN name, canonical form, no brand names
- [x] `Severity` enum: `Contraindicated`, `HighRisk`, `Moderate`, `Minor`, `Unknown`
- [x] `Interaction` model: drug pair, severity, description, evidence level, source citation
- [x] INN name validation rules (format, normalization, rejected patterns)
- [x] Domain error types with `thiserror`, zero `unwrap()` outside tests
- [x] Unit tests for every invariant above (severity ordering, INN validation edge cases, self-interaction rejection)

## Phase 2: Binary Database Format (`.men`) (done)

- [x] Format specification written down (`docs/DATABASE_FORMAT.md`): header layout, endianness (fixed, documented), versioning strategy, forward-compatibility rules
- [x] Header: magic bytes, format version, build timestamp, checksum, section offsets
- [x] Drug table: `DrugId` to INN name offset
- [x] Interaction index: drug-pair to interaction record lookup
- [x] Interaction records: severity, description, evidence level, source
- [x] Checksum scheme chosen and documented (CRC32 for the header, SHA-256 for whole-payload integrity)

## Phase 3: Database Reader (`mensung-db`) (done)

- [x] Zero-copy parsing of the `.men` format, no `unsafe`
- [x] Checksum validation on load, corrupt files rejected with a typed error, never a panic
- [x] Drug lookup by exact INN name (binary search); no `DrugId`-keyed lookup was added since nothing in the product flow needs one yet
- [x] Interaction-pair lookup (binary search)
- [x] No filesystem access, no allocation, no locking on the lookup hot path
- [x] Fuzz target for the binary reader (`cargo-fuzz`) plus dependency-free crash tests covering every bit flip and every truncation length of a valid file

## Phase 4: Lookup Engine and Fuzzy Matcher (`mensung-core`) (done)

- [x] Exact lookup engine wired to `mensung-db`; `<5ms` per lookup not yet benchmarked, deferred to Phase 9
- [x] Fuzzy matcher returning ranked candidates with similarity scores, never auto-correcting. Uses `strsim` (Jaro-Winkler) rather than `nucleo` or `simmetrics`: this is a spelling-correction problem, not an interactive fuzzy-find problem, see `fuzzy.rs`'s header for the reasoning
- [x] Confirmation-flow types: an unmatched name always produces a candidate list, never a silent substitution
- [x] Multi-drug interaction checking (more than two drugs in one session), sorted most severe first
- [x] Unit tests: `Amoxilin` / `Amoxicilin` / `Amoxycillin` all resolve to `Amoxicillin` as the top ranked candidate, never automatically

## Phase 5: Data Pipeline (`mensung-builder`) (writer done, importers open)

- [ ] OpenFDA importer -- needs real schema research, not attempted yet
- [ ] RxNorm importer -- needs real schema research, not attempted yet
- [ ] WHO dataset importer -- needs real schema research, not attempted yet
- [ ] Common intermediate schema shared across importers
- [x] Validation pipeline: duplicate drugs, dangling drug references, duplicate interaction pairs (invalid INN names and missing severity are already unrepresentable, rejected at construction by `mensung-domain`)
- [x] `validation-report.json` output (`errors`, `warnings`, `interactions` counts); a build with non-zero errors must not produce a `.men` file
- [x] `.men` database compiler, with round-trip self-verification through `mensung-db` and `SOURCE_DATE_EPOCH` support for reproducible builds
- [ ] Builder CLI (`mensung-builder build --out medical_database.men`) -- not needed yet, `mensung-client`'s `build.rs` calls the library directly; add once the real importers exist and a human needs to run this by hand

## Phase 6: CLI (`mensung-client`) (done for the bootstrap dataset)

- [x] Two-drug (and N-drug) interaction lookup command
- [x] Plain-text output mode; JSON output mode not added, nothing consumes it yet
- [x] Exit codes distinguishing "no interaction," "interaction found," "input error," and "internal/database error"
- [x] Wired to `mensung-core` and `mensung-db` only; no direct filesystem parsing of the database format outside `mensung-db`. The database itself is compiled and embedded at build time via `mensung-builder`, not read from an external file at runtime

## Phase 7: TUI (`mensung-client`) (done, live suggestions still open)

- [x] `ratatui` + `crossterm` interface
- [ ] Drug input screens with live fuzzy-match suggestions as the user types; today the candidate list appears after Enter on a non-exact match, not live while typing
- [x] Explicit confirmation step for any non-exact match, same `LookupOutcome` flow as the CLI
- [x] Color rules: red for contraindicated/high risk, yellow for moderate/minor/unknown, green for no known interaction
- [x] Keyboard-only navigation (Tab/Up/Down/Enter/Esc/Ctrl-C), no mouse required

Verified interactively in a real terminal (tmux), not just unit-tested: typed input, candidate confirmation, and the results screen all render and respond correctly. That pass caught a real bug the unit tests missed -- dismissing a result screen did not clear the input fields, so a second lookup silently concatenated onto the first. Fixed, with a regression test.

## Phase 8: Medical Safety Test Suite (golden cases done, rest open)

- [x] `tests/golden_cases.json`: fixed known-interaction cases (Aspirin + Warfarin => Contraindicated, Paracetamol + Amoxicillin => no interaction, and two more as the dataset grows)
- [x] CI gate: `crates/mensung-builder/tests/golden_cases.rs` runs as part of `cargo test --workspace`; a build that drops or weakens a case fails it, no separate invocation needed
- [ ] `cargo-fuzz` targets: parser (blocked on the real importers existing), fuzzy search engine (not started); binary reader done in Phase 3
- [ ] Property-based tests for domain validation logic

## Phase 9: Performance Hardening

- [ ] Benchmark harness (criterion) for startup time and lookup time
- [ ] Startup `<100ms`, lookup `<5ms`, memory `<50MB`, binary (including database) `<10MB`, verified in CI, not just claimed
- [ ] Memory and CPU profiling on constrained hardware (Core 2 Duo / early i3 class, or an equivalent throttled CI runner)
- [ ] Binary size budget check as a CI step, fails the build if exceeded

## Phase 10: Security Hardening

- [ ] Full `unsafe` audit: every remaining `unsafe` block justified, documented, isolated, and tested; zero otherwise
- [ ] cargo-audit and cargo-deny stay green as the dependency tree grows
- [ ] Reproducible build verification (same input, same `.men` output, byte for byte)

## Phase 11: First Public Dataset and v1.0.0

- [ ] Full dataset build from OpenFDA + RxNorm + WHO sources, target scale in the hundreds of thousands of interaction records
- [ ] `validation-report.json` with zero errors on the shipped dataset
- [ ] Field deployment guide: how to copy the binary to an offline machine, how to verify the checksum, how to report a data error from the field
- [ ] `v1.0.0` tag, `release.yml` run, binaries and `SHA256SUMS.txt` published for Linux/Windows/macOS

## Future / Ecosystem

- [ ] Brand-name-to-INN alias lookup as a convenience layer, still routed through the confirmation flow, never a silent substitution
- [ ] Periodic offline database updates, distributable by USB/sneakernet for field sites with no connectivity at all
- [ ] ARM builds (Raspberry Pi class hardware) for even lower-power clinic deployments
- [ ] Additional TUI languages for field usability (French, Arabic, Dzongkha, and others as translators contribute)
- [ ] Offline export of an interaction report to a file, for attaching to a patient chart
