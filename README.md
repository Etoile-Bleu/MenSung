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
patient at risk. MenSung exists to give those workers a fast, correct
lookup tool that runs on whatever laptop happens to be available, and does
not need connectivity for the lookups themselves.

Every actual drug lookup is fully offline: once the database is on the
machine, MenSung never touches the network to answer a query. Getting the
database onto the machine is the one exception. That happens once, either
by running `mensung` somewhere with connectivity and letting it install a
dataset, or by copying a pre-built `medical_database.men` file onto the
machine by hand (USB stick, local network, however it gets there).
After that one-time step, the binary and its database file can be copied
anywhere, including a machine that will never see a network connection
again. See [Security model](#security-model) for exactly what the binary
does and does not do over the network.

MenSung will run on a 10-15 year old laptop with an Intel Core 2 Duo or early
i3-class CPU, 2-4GB of RAM, and a slow HDD, with no GPU, no cloud service,
and no database server.

## What it does

A worker enters two (or more) drug names, using International Nonproprietary
Names (INN) only, no brand names. MenSung looks them up against its locally
installed binary database and reports every known interaction, ranked by
severity.

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

This is real, working CLI output, not a mockup, from a database installed
as described in [Usage](#usage) below.

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
| `mensung-builder` | DDInter importer and downloader, parser, and `.men` database compiler. Depends on `mensung-domain` and `mensung-db`. Linked into the `mensung` binary, since `mensung-client` calls it to install the database at runtime; it is the only source of network code in the workspace. |
| `mensung-client` | CLI and TUI (`ratatui` + `crossterm`), the deployed `mensung` binary. |

`mensung-core` talks to `mensung-db`'s concrete reader directly rather than
through a trait-based port: there is exactly one `.men` implementation in
scope, and a lifetime-generic port over its zero-copy return types would add
real complexity for no adapter it would ever swap in. See
[CONTRIBUTING.md](CONTRIBUTING.md) for the reasoning.

A medical worker runs one statically linked binary plus one `.men` database
file sitting next to it; nothing else to install, no runtime services, no
required configuration. The database is not embedded inside the binary: see
[Usage](#usage) for how it gets installed and [Security
model](#security-model) for the network implications of that.

### Performance targets

| Target | Budget |
|--------|--------|
| Startup | < 100ms |
| Drug lookup | < 5ms |
| Memory | < 50MB RAM |
| Binary size | < 10MB |

The installed database is separate from the binary and is not held to the
10MB figure. A bare DDInter-only build compiles to about 9.3MB under the
`.men` format v2 shared string table (down from format v1's roughly
28MB); the enriched database `mensung` installs by default (DDInter +
RxNorm + WHO ATC + PubChem + openFDA, see [Usage](#usage)) is larger,
about 27MB, since it also carries the openFDA drug facts text. See
ROADMAP.md Phase 5's known tradeoff note and Phase 8b.

## Installation

Download the binary for your platform from the
[latest release](https://github.com/Etoile-Bleu/MenSung/releases/latest).
Running it looks for `medical_database.men` next to itself; if that file is
not there yet, it offers to install a dataset, which needs a network
connection for that one step. See [Usage](#usage).

## Usage

The first time `mensung` runs and finds no `medical_database.men` next to
itself, it says so and, in an interactive terminal, asks whether to install
a dataset now:

```
No medication database found at /path/to/medical_database.men.
You can place a compiled medical_database.men there yourself, or let mensung install a dataset now.
Would you like to install the dataset now? [y/N]
```

Answering yes downloads MenSung's pre-built, enriched database (DDInter +
RxNorm + WHO ATC + PubChem + openFDA, see MEDICAL_DATA_POLICY.md) from this
project's `medical-database` GitHub Release over HTTPS (TLS certificate
validation is never disabled) and saves it directly; no per-drug API calls,
no local build step. If that download fails for any reason (no route to
GitHub, a corrupt transfer), `mensung` falls back to downloading DDInter's
own public CSV export and compiling a bare DDInter-only database locally
instead, so a field deployment with only partial connectivity still ends up
with a usable database. Either way, this is the only time `mensung` touches
the network, and only with this explicit confirmation. In a non-interactive
shell, set `MENSUNG_DOWNLOAD_DATASET=1` to skip the prompt, or place a
pre-built `medical_database.men` next to the binary yourself (or point
`MENSUNG_DATA_DIR` at wherever you keep it) and `mensung` never needs to
ask. Once installed, every lookup is fully offline again; nothing about
answering questions touches the network.

Run `mensung` with no arguments for the interactive terminal interface: two
input fields, Tab to switch between them, Enter to check, Alt+I to show the
focused field's own drug information (RxCUI, WHO ATC classification,
chemical properties, and any contraindications, boxed warnings, or similar
facts known about it) instead of checking an interaction. Alt+I rather than
a bare `i` or F1: a bare letter would collide with typing a drug name, and
F1 is commonly intercepted by the terminal or OS for their own help screen
before the app sees it. A typed name with no exact match shows a ranked
candidate list with a similarity score and waits for confirmation, the same
for either Alt+I or Enter; it never guesses. Interactions and drug facts are
shown red for contraindicated or high risk, yellow for moderate, minor, or
unknown severity, green for no known interaction. Esc or Ctrl-C quits.

```bash
mensung <drug-1> <drug-2> [<drug-3> ...]
```

Run it with two or more drug names for the scriptable command-line mode
instead: every known pairwise interaction out, most severe first. Exit
codes: `0` no known interaction, `1` an interaction was found or a name
could not be resolved, `2` bad command-line usage, `70` an internal or
database error. A typed name with no exact match returns a ranked candidate
list instead of guessing, same as the interactive mode:

```
$ mensung Amoxilin Aspirin

Unknown drug:
Amoxilin

Did you mean:

Amoxicillin (92.0%)
Aspirin (69.0%)

Confirm your selection and try again with the exact name.
```

```bash
mensung info <drug-name>
```

The command-line equivalent of the TUI's Alt+I key: one drug's own RxCUI, WHO
ATC classification, chemical properties, and any known contraindications,
boxed warnings, or similar facts, instead of an interaction between two
drugs. Exit codes: `0` the drug resolved (whether or not it has any facts
or cross-reference data), `1` the name could not be resolved, `2` bad
command-line usage, `70` an internal or database error. Nothing in the
DDInter-only dataset `mensung` installs by default populates this data yet;
see [ROADMAP.md](ROADMAP.md)'s Phase 8b and the builder CLI
(`mensung-builder build --out <path>`) for how a richer database gets
built.

```bash
mensung version
mensung check-update
```

`version` prints the installed version and exits; it never touches the
network or the database. `check-update` is a manual, explicit request
against GitHub's release API for a version newer than the one running; it
never runs automatically (not at startup, not anywhere else), and it never
downloads or installs anything itself, only reports what it found and a
link to get it, the same "explicit confirmation, no silent network
activity" rule the dataset install follows. Both work with no database
installed and never trigger the dataset install prompt.

See [MEDICAL_DATA_POLICY.md](MEDICAL_DATA_POLICY.md) for why DDInter and not
OpenFDA/RxNorm/WHO as originally planned, and for the data license that
applies to the installed database (CC BY-NC-SA 4.0, separate from this
project's own MIT/Apache-2.0 code license).

## Building from source

```bash
git clone https://github.com/Etoile-Bleu/MenSung.git
cd MenSung
cargo build --release
```

The `mensung` binary is produced by the `mensung-client` crate at
`target/release/mensung`. The build itself never touches the network or
needs a dataset; see [Usage](#usage) for how the binary installs its
database the first time it runs.

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

MenSung never sends telemetry, never collects patient data, and never
stores patient information. It does contain network code, unlike earlier
drafts of this project, in exactly two places, both explicit and neither
automatic:

- **Dataset install.** `mensung-client`'s `dataset_download.rs`, called
  when a database is not yet installed and the user explicitly agrees,
  either by answering the interactive prompt or by setting
  `MENSUNG_DOWNLOAD_DATASET=1` ahead of time, fetches the pre-built,
  enriched database from this project's own `medical-database` GitHub
  Release. If that fails, `mensung-builder`'s downloader fetches DDInter's
  public CSV export over HTTPS instead, from `ddinter.scbdd.com` first and
  this project's own GitHub Releases second, as a fallback mirror of the
  exact same files, used when DDInter's own TLS certificate cannot be
  validated (true as of this writing; see MEDICAL_DATA_POLICY.md). Nothing
  else.
- **`mensung check-update`.** Fetches this project's latest release
  metadata from `api.github.com` when, and only when, the user runs that
  exact command. It never runs at startup or anywhere else, and it never
  downloads or installs a new binary itself; it only reports what it found
  and a link to get it. See [Usage](#usage).

Both:

- Never run automatically or silently; each only runs on its own explicit
  trigger (a missing database plus a yes, or the `check-update` command
  itself).
- TLS certificate validation is never disabled on any source. An invalid
  or expired certificate is a hard failure, not a fallback to an
  unverified connection.
- Touch no other host than the ones named above.

Once a database is installed, every drug lookup is answered locally; the
lookup path itself makes no network calls, regardless of how the database
got there, and `version` never touches the network at all.

See [SECURITY.md](SECURITY.md) for the vulnerability reporting process and
scope.

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
