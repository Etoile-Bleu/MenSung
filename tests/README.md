# Tests

This directory holds cross-crate and golden medical test fixtures, separate
from the per-crate unit tests that live under each `crates/*/src`.

- `golden_cases.json` -- fixed set of known interactions (Aspirin + Warfarin
  => Contraindicated, Paracetamol + Amoxicillin => no interaction, and more
  as the dataset grows). Checked by
  `crates/mensung-builder/tests/golden_cases.rs` on every `cargo test
  --workspace`, against a small fixture built to match this file exactly,
  since no dataset is embedded in the workspace to test by default (the
  real dataset is installed by `mensung-client` at runtime; see
  README.md's Usage section). Run the same check by hand against a real
  `.men` file before a release. A build that drops or weakens a case fails
  the test; see MEDICAL_DATA_POLICY.md.

Planned, not yet added:

- A fuzz target for the DDInter CSV parser (`mensung-builder`'s `ddinter`
  module). A fuzz target for the `.men` binary reader already exists in
  `fuzz/`.
- Property-based tests for domain validation logic.
- CLI integration tests for `mensung-client`.
