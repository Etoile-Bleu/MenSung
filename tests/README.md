# Tests

This directory holds cross-crate and golden medical test fixtures, separate
from the per-crate unit tests that live under each `crates/*/src`.

- `golden_cases.json` -- fixed set of known interactions (Aspirin + Warfarin
  => Contraindicated, Paracetamol + Amoxicillin => no interaction, and more
  as the dataset grows). Checked by
  `crates/mensung-builder/tests/golden_cases.rs` on every `cargo test
  --workspace`. A build that drops or weakens a case fails that test; see
  MEDICAL_DATA_POLICY.md. It runs against the bootstrap seed dataset today
  and will run against the real dataset once ROADMAP.md Phase 11 lands,
  without needing to change.

Planned, not yet added:

- Fuzz targets for the builder's data-format parsers, once the real
  OpenFDA/RxNorm/WHO importers exist (ROADMAP.md Phase 5). A fuzz target for
  the `.men` binary reader already exists in `fuzz/`.
- Property-based tests for domain validation logic.
- CLI integration tests for `mensung-client`.
