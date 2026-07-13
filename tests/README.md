# Tests

This directory holds cross-crate and golden medical test fixtures, separate
from the per-crate unit tests that live under each `crates/*/src`.

Planned contents (added once application logic lands):

- `golden_cases.json` -- fixed set of known interactions (e.g. Aspirin +
  Warfarin => critical, Paracetamol + Amoxicillin => no interaction) that
  every build is checked against. See MEDICAL_DATA_POLICY.md.
- Fuzz targets for the `.men` binary reader and the parser, run with
  `cargo-fuzz`.
- CLI integration tests for `mensung-client`.
