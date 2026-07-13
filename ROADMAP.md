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

## Phase 1: Domain Layer (`mensung-domain`)

- [ ] Newtype IDs: `DrugId`, `InteractionId` (no bare `u32`/`u64` aliases)
- [ ] `Drug` entity: INN name, canonical form, no brand names
- [ ] `Severity` enum: `Contraindicated`, `HighRisk`, `Moderate`, `Minor`, `Unknown`
- [ ] `Interaction` model: drug pair, severity, description, evidence level, source citation
- [ ] INN name validation rules (format, normalization, rejected patterns)
- [ ] Domain error types with `thiserror`, zero `unwrap()` outside tests
- [ ] Unit tests for every invariant above (severity ordering, INN validation edge cases, duplicate pair detection)

## Phase 2: Binary Database Format (`.men`)

- [ ] Format specification written down (`docs/DATABASE_FORMAT.md`): header layout, endianness (fixed, documented), versioning strategy, forward-compatibility rules
- [ ] Header: magic bytes, format version, build timestamp, checksum, section offsets
- [ ] Drug table: `DrugId` to INN name offset
- [ ] Interaction index: drug-pair to interaction record lookup
- [ ] Interaction records: severity, description, evidence level, source
- [ ] Checksum scheme chosen and documented (e.g. CRC32 for structure, SHA-256 for whole-file integrity)

## Phase 3: Database Reader (`mensung-db`)

- [ ] Zero-copy parsing of the `.men` format
- [ ] Checksum validation on load, corrupt files rejected with a typed error, never a panic
- [ ] Drug lookup by `DrugId` and by exact INN name
- [ ] Interaction-pair lookup
- [ ] No filesystem access, no allocation, no locking on the lookup hot path
- [ ] Fuzz target for the binary reader (`cargo-fuzz`), run against malformed and truncated files

## Phase 4: Lookup Engine and Fuzzy Matcher (`mensung-core`)

- [ ] Exact lookup engine wired to `mensung-db`, `<5ms` per lookup
- [ ] Fuzzy matcher (nucleo or simmetrics) returning ranked candidates with similarity scores, never auto-correcting
- [ ] Confirmation-flow types: an unmatched name always produces a candidate list, never a silent substitution
- [ ] Multi-drug interaction checking (more than two drugs in one session)
- [ ] Unit tests: `Amoxilin` / `Amoxicilin` / `Amoxycillin` all resolve to `Amoxicillin` as a ranked candidate, never automatically

## Phase 5: Data Pipeline (`mensung-builder`)

- [ ] OpenFDA importer
- [ ] RxNorm importer
- [ ] WHO dataset importer
- [ ] Common intermediate schema shared across importers
- [ ] Validation pipeline: duplicate drugs, invalid or non-INN names, missing severity, corrupted interaction records
- [ ] `validation-report.json` output (`errors`, `warnings`, `interactions` counts); a build with non-zero errors must not produce a `.men` file
- [ ] `.men` database compiler
- [ ] Builder CLI (`mensung-builder build --out medical_database.men`)

## Phase 6: CLI (`mensung-client`)

- [ ] Two-drug (and N-drug) interaction lookup command
- [ ] Plain-text and JSON output modes
- [ ] Exit codes distinguishing "no interaction," "interaction found," and "input error"
- [ ] Wired to `mensung-core` and `mensung-db` only; no direct filesystem parsing of the database format outside `mensung-db`

## Phase 7: TUI (`mensung-client`)

- [ ] `ratatui` + `crossterm` interface
- [ ] Drug input screens with live fuzzy-match suggestions
- [ ] Explicit confirmation step for any non-exact match
- [ ] Color rules: red for danger, yellow for warning, green for no known interaction
- [ ] Keyboard-only navigation, readable on a low-resolution old laptop screen

## Phase 8: Medical Safety Test Suite

- [ ] `tests/golden_cases.json`: fixed known-interaction cases (Aspirin + Warfarin => critical, Paracetamol + Amoxicillin => no interaction, and more as the dataset grows)
- [ ] CI gate: a build that drops or weakens a golden case fails, no exceptions without a documented clinical reason
- [ ] `cargo-fuzz` targets: parser, binary reader, fuzzy search engine
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
