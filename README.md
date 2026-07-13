# MenSung

**Medical Shield.** An offline medication interaction checker for doctors,
nurses, and humanitarian medical workers operating without internet access.

> **MEDICAL DISCLAIMER**
>
> This software is an offline informational aid. It does not replace
> professional medical judgement, clinical protocols, or qualified healthcare
> decisions. Always use professional clinical judgement.

---

## Why this exists

In war zones, refugee camps, rural clinics, and disaster areas, medical
workers often have to make drug interaction decisions with no internet
connection, on hardware that predates most currently-supported software. A
missed contraindication in that setting is not an abstract bug: it is a
patient at risk. MenSung exists to give those workers a fast, correct,
completely offline lookup tool that runs on whatever laptop happens to be
available.

MenSung will run on a 10-15 year old laptop with an Intel Core 2 Duo or early
i3-class CPU, 2-4GB of RAM, and a slow HDD, with no GPU, no internet
connection, no cloud service, no database server, and nothing to install
beyond copying a single binary.

## What it does

A worker enters two (or more) drug names, using International Nonproprietary
Names (INN) only, no brand names. MenSung looks them up against an embedded
binary database and reports every known interaction, ranked by severity.

```
$ mensung Aspirin Warfarin

!!! CONTRAINDICATED INTERACTION !!!

Aspirin + Warfarin

Severity:
CONTRAINDICATED

Risk:
Increased bleeding and hemorrhage probability.

Evidence: Established (...)

This software is an offline informational assistant.
Always use professional clinical judgement.
```

This is real, working CLI output today, not a mockup, though it is running
against the small bootstrap seed dataset described below, not the full
dataset Phase 11 will ship.

Two rules shape every part of the design:

- **Zero false negative policy.** If an interaction exists in the data, it is
  shown. Recall matters more than precision: an extra warning costs a moment
  of attention, a missed interaction can cost a patient's life.
- **No silent correction.** If a typed drug name does not match an INN entry
  exactly, MenSung shows ranked candidates with similarity scores and asks
  for confirmation. It never substitutes a guess on its own.

See [MEDICAL_DATA_POLICY.md](MEDICAL_DATA_POLICY.md) for the full policy,
data sources, and validation pipeline.

## Architecture

MenSung is a Cargo workspace following Clean/Hexagonal Architecture. The
dependency direction is one-way toward `mensung-domain`, which never depends
on anything else in the workspace, so it never knows about the filesystem,
the database format, the CLI, the TUI, or the network:

```
mensung-domain
      ^
      |
mensung-db  <--------  mensung-builder
      ^                       ^
      |                       |
mensung-core            (offline tool,
      ^                  not linked into
      |                  the client)
mensung-client
```

| Crate | Responsibility |
|-------|----------------|
| `mensung-domain` | Drug entities, interaction models, severity rules, validation logic. No I/O, no UI, no dependency on anything else in the workspace. |
| `mensung-db` | Binary `.men` database reader: zero-copy access, checksum validation. Depends on `mensung-domain` for shared value types. |
| `mensung-core` | Lookup engine, fuzzy matcher, business rules. Depends on `mensung-domain` and `mensung-db`, since a lookup has to read the one database format this project ships. |
| `mensung-builder` | OpenFDA/RxNorm/WHO importers, parser, and `.men` database compiler. Depends on `mensung-domain` and `mensung-db`; it is a separate offline tool, not linked into the `mensung` binary. |
| `mensung-client` | CLI and TUI (`ratatui` + `crossterm`), the deployed `mensung` binary. |

`mensung-core` talks to `mensung-db`'s concrete reader directly rather than
through a trait-based port: there is exactly one `.men` implementation in
scope, and a lifetime-generic port over its zero-copy return types would add
real complexity for no adapter it would ever swap in. See
[CONTRIBUTING.md](CONTRIBUTING.md) for the reasoning. Everything a medical
worker runs is a single statically linked binary with one embedded `.men`
database file: no installation, no runtime dependencies, no configuration.

### Performance targets

| Target | Budget |
|--------|--------|
| Startup | < 100ms |
| Drug lookup | < 5ms |
| Memory | < 50MB RAM |
| Binary size (including database) | < 10MB |

## Installation

Download the binary for your platform from the
[latest release](https://github.com/Etoile-Bleu/MenSung/releases/latest) and
run it directly. No installer, no dependencies, no internet connection
required at runtime.

## Usage

```bash
mensung <drug-1> <drug-2> [<drug-3> ...]
```

Two or more INN drug names in, every known pairwise interaction out, most
severe first. Exit codes: `0` no known interaction, `1` an interaction was
found or a name could not be resolved, `2` bad command-line usage, `70` an
internal or database error. A typed name with no exact match returns a
ranked candidate list instead of guessing:

```
$ mensung Amoxilin Aspirin

Unknown drug:
Amoxilin

Did you mean:

Amoxicillin (92.0%)
Aspirin (69.0%)

Confirm your selection and try again with the exact name.
```

The `mensung-client` crate currently embeds a small, clearly-marked
bootstrap dataset (five drugs, three textbook interactions) at build time,
described in [mensung-builder's seed module](crates/mensung-builder/src/seed.rs).
The real dataset lands in ROADMAP.md's Phase 11, once the OpenFDA/RxNorm/WHO
importers exist.

## Building from source

```bash
git clone https://github.com/Etoile-Bleu/MenSung.git
cd MenSung
cargo build --release
```

The `mensung` binary is produced by the `mensung-client` crate at
`target/release/mensung`.

### Running tests

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo audit
cargo deny check
```

Supported build targets: `x86_64-unknown-linux-musl` (statically linked),
`x86_64-pc-windows-msvc`, `x86_64-apple-darwin`.

## Security model

MenSung never accesses the internet, never sends telemetry, never collects
patient data, and never stores patient information. The only network-shaped
surface in the entire project is the builder pipeline used at database
build time to import public datasets (OpenFDA, RxNorm, WHO); the deployed
`mensung` binary that reaches the field has no network code at all. See
[SECURITY.md](SECURITY.md) for the vulnerability reporting process and scope.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for build instructions, the Rust
engineering rules this project follows, and PR guidelines. See
[MEDICAL_DATA_POLICY.md](MEDICAL_DATA_POLICY.md) before touching any drug or
interaction data. See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) for community
standards.

## License

Dual-licensed under either of:

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.
