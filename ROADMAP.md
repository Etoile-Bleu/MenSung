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

## Phase 5: Data Pipeline (`mensung-builder`) (done, except the builder CLI)

The original plan (OpenFDA + RxNorm + WHO) does not work in practice, checked
directly rather than assumed: OpenFDA's interaction data is unstructured
prose, RxNorm's Drug Interaction API was discontinued by the NLM in January
2024, and WHO has no public structured DDI dataset. See
MEDICAL_DATA_POLICY.md's Data Sources section for the full research and why
[DDInter](http://ddinter.scbdd.com/) is the actual target now.

- [x] DDInter importer (`mensung_builder::import_ddinter`) -- parses DDInter's downloadable CSV export (RFC 4180, quote-aware) into `mensung_domain::Drug`/`Interaction` values. Verified against the real, full 8-file export: 1939 drugs, 160235 deduplicated interactions, ~160ms to import
- [x] Validation pipeline: duplicate drugs, dangling drug references, duplicate interaction pairs (invalid INN names and missing severity are already unrepresentable, rejected at construction by `mensung-domain`)
- [x] `validation-report.json` output (`errors`, `warnings`, `interactions` counts); a build with non-zero errors must not produce a `.men` file
- [x] `.men` database compiler, with round-trip self-verification through `mensung-db` and `SOURCE_DATE_EPOCH` support for reproducible builds
- [x] `download` module (`mensung_builder::download_and_import_ddinter`) -- fetches DDInter's eight CSV files over HTTPS with TLS validation never disabled, used by `mensung-client` at runtime (see Phase 6); the only network-touching code in the workspace. Falls back to a mirror on this project's own GitHub Releases (`ddinter-mirror-2025-08-30`) when DDInter's site cannot be reached over a validated connection, which has been true every time this was checked while building this phase; see MEDICAL_DATA_POLICY.md
- [ ] Builder CLI (`mensung-builder build --out medical_database.men`) -- not needed yet, nothing currently needs to run this outside of `mensung-client`'s own runtime install flow; add once a human needs to run this by hand

**Known tradeoff, accepted:** compiling the full real DDInter dataset
produces a `.men` file around 28MB, well past the `<10MB` binary budget in
Phase 9. Root cause: DDInter's bulk CSV export has no per-pair description
or citation, only a severity level, so the importer synthesizes a
description from the severity tier; that synthesized text (and the source
string) repeats verbatim across most of the 160235 records, and the `.men`
format inlines description/source per record instead of deduplicating
repeated text through a shared string table the way the Drug Table already
does for names. The real fix is a format v2 with a shared string table for
interaction text, not attempted yet; revisit if the `<10MB` budget turns
out to matter in practice once this ships.

## Phase 6: CLI (`mensung-client`) (done)

- [x] Two-drug (and N-drug) interaction lookup command
- [x] Plain-text output mode; JSON output mode not added, nothing consumes it yet
- [x] Exit codes distinguishing "no interaction," "interaction found," "input error," and "internal/database error"
- [x] Wired to `mensung-core` and `mensung-db` for lookups; `mensung-builder` for installing the database at runtime if missing (`data.rs`), the only network-touching path in the binary, gated on explicit user confirmation. No dataset is embedded at build time; see MEDICAL_DATA_POLICY.md's Data License section and README.md's Security model for what this means and why

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

## Phase 8b: Multi-Source Clinical Fact Model (domain layer and OpenFDA importer done, format v2 not started)

DDInter alone answers "does an interaction exist and how severe is it."
Adding DailyMed, OpenFDA, or another regulatory source means those sources
can disagree with DDInter and with each other; the domain layer needs a way
to keep every source's claim rather than pick one and silently drop the
rest. See MEDICAL_DATA_POLICY.md's Trust and Conflict Resolution section
for the policy this implements.

- [x] `Source` / `SourceId` / `SourceTier`: a named, ranked contributor of clinical claims
- [x] `Confidence`: a source's own confidence in its claim (`Low`/`Medium`/`High`), kept as an enum rather than a float to avoid NaN/precision handling for a value that is only ever compared, never averaged
- [x] `ClaimDate`: a minimal, dependency-free calendar date (no `chrono`) for "last confirmed against source," with real calendar validation including leap years
- [x] `Claim`: one source's severity, evidence level, confidence, rationale, and last-updated date for a single fact
- [x] `InteractionFact` / `DrugFact`: one or more `Claim`s per fact, `primary_claim()` picks the most authoritative tier present and breaks same-tier ties toward the more severe reading, `resolve()` collapses to the existing single-severity `Interaction` shape without deleting the other claims
- [x] `Severity::clinical_meaning()`: the four-tier clinical scale (Absolute contraindication / Strongly discouraged / Use with caution / Informational) alongside the existing short display label
- [x] Full unit test coverage: tier ranking, same-tier severity tie-breaking, zero-claims rejection, calendar date edge cases (leap years, century years, invalid days)
- [x] OpenFDA Drug Labels integrated (`mensung-builder::openfda`, `openfda_download`): field names, `openfda.generic_name` shape, and the `effective_time` date format verified against a real live API response and FDA's own published schema, not assumed. Produces `DrugFact`s (contraindication, boxed warning, warning, pregnancy, breastfeeding, dosage, indication) for drugs matched to an existing INN name by exact word-prefix matching, never a fuzzy or substring match. Proven end to end against real, live data for real drugs (`tests/openfda_live.rs`, `#[ignore]`d like the live-network unit tests in `openfda_download.rs`, since a test suite that depends on an external service being up is not something `cargo test --workspace` should run in CI)
- [x] RxNorm identity normalization integrated (`mensung_domain::Rxcui`, `Drug::with_rxcui`, `mensung-builder::rxnorm`, `rxnorm_download`): attaches an RxCUI to each drug via RxNorm's own normalized search, which already handles salt forms and word order server side, checked directly against real responses including one of DDInter's more unusual names (`Dexamethasone (topical)`). Proven end to end against real, live data (`tests/rxnorm_live.rs`, same `#[ignore]` pattern as the OpenFDA live tests)
- [ ] OpenFDA does not yet exercise actual conflict *resolution*: it is currently the only source contributing `DrugFact`s (DDInter only contributes `InteractionFact`s), so there is no second source yet disagreeing with it on the same fact. Proving `primary_claim()`'s tie-break logic against real disagreeing data needs a second source overlapping OpenFDA on the same drug fact, or a second source overlapping DDInter on the same interaction pair
- [ ] Neither OpenFDA nor RxNorm is yet wired into a build: `DrugFact`s have nowhere to be persisted and an `Rxcui` is not yet compiled into the `.men` file, both waiting on the format v2 below, so these importers are tested, working code, not yet reachable from `mensung-client`
- [ ] `mensung-builder`'s DDInter importer migrated to produce `Claim`/`InteractionFact` instead of `Interaction` directly -- deferred until there is a build that actually combines it with a second interaction-level source, so this migration happens once, together with real multi-source data, instead of twice
- [ ] `.men` format v2 with a shared string table, needed to persist and display full multi-source provenance (`InteractionFact` and `DrugFact` data) and the `Rxcui` cross-reference, instead of only the resolved `Interaction`/name-only `Drug` view

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

- [ ] Full dataset build from DDInter (~300k DDI records across ~2300 drugs); ships under CC BY-NC-SA 4.0 per MEDICAL_DATA_POLICY.md's Data License section, separate from the MIT/Apache code license
- [ ] `validation-report.json` with zero errors on the shipped dataset
- [ ] Field deployment guide: how to copy the binary to an offline machine, how to verify the checksum, how to report a data error from the field
- [ ] `v1.0.0` tag, `release.yml` run, binaries and `SHA256SUMS.txt` published for Linux/Windows/macOS

## Future / Ecosystem

- [ ] Brand-name-to-INN alias lookup as a convenience layer, still routed through the confirmation flow, never a silent substitution
- [ ] Periodic offline database updates, distributable by USB/sneakernet for field sites with no connectivity at all
- [ ] ARM builds (Raspberry Pi class hardware) for even lower-power clinic deployments
- [ ] Additional TUI languages for field usability (French, Arabic, Dzongkha, and others as translators contribute)
- [ ] Offline export of an interaction report to a file, for attaching to a patient chart
