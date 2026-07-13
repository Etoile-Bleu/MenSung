# Medical Data Policy

MenSung is a clinical decision-support tool, not a certified medical device.
This document describes the rules that govern the medical data it ships with
and the guarantees (and limits) of that data. Read this before filing a
medical data issue or contributing to the database.

## Medical Disclaimer

MenSung is an offline informational aid. It does not replace professional
medical judgement, clinical protocols, or qualified healthcare decisions.
Every screen that shows an interaction result restates this. Treat any output
as a prompt to check further, never as a final answer.

## Zero False Negative Policy

The system prioritizes recall over precision. If an interaction exists in the
underlying dataset, it must be returned. Displaying an extra warning that
turns out to be low-risk is an acceptable cost; missing a real interaction is
not. Contributions that would silently drop or filter interactions to reduce
noise will be rejected unless they come with clear clinical justification and
a source citation.

## No Silent Correction

MenSung never substitutes a user's input with a guess. When a typed drug name
does not match an entry in the database exactly, the fuzzy matcher offers
ranked candidates with a similarity score and asks the user to confirm. It
never assumes the closest match is correct and proceeds on its own.

## Drug Naming Standard

Only International Nonproprietary Names (INN) are used in the database.

Valid: `Paracetamol`, `Amoxicillin`, `Warfarin`, `Aspirin`

Invalid: brand names, local trade names (`Doliprane`, `Tylenol`, and similar)

This keeps the database usable across countries and health systems that use
different local brand names for the same substance. Contributions that add
brand names as primary entries will be rejected; brand-to-INN aliasing may be
considered separately as a lookup convenience, never as a replacement for the
INN record itself.

## Data Sources

The builder pipeline (`mensung-builder`) compiles the shipped `.men`
database from [DDInter](http://ddinter.scbdd.com/), an actively maintained,
freely downloadable drug-drug interaction database with clinical severity
levels (mild/moderate/severe), interaction mechanisms, and source citations.

This was not the original plan; OpenFDA, RxNorm, and WHO datasets were the
initial targets, and none of them work for this purpose in practice.
OpenFDA's drug interaction field is unstructured prose inside drug labels,
not structured drug pairs. RxNorm's Drug Interaction API was discontinued by
the NLM in January 2024. WHO does not publish a structured public
drug-drug-interaction dataset. DrugBank-derived sources, including via
Therapeutics Data Commons, require an academic or paid license. DDInter was
the only actively maintained, genuinely downloadable, severity-graded source
found after checking each of these directly. See
[Data License](#data-license) below for what that choice means for
redistribution.

`ddinter.scbdd.com`'s TLS certificate was found expired while building the
importer (checked directly and repeatedly, not assumed). Rather than ever
accept an unverified connection, `mensung-builder`'s downloader falls back
to an unmodified, byte-for-byte mirror of the same eight files hosted as
assets on this project's own [GitHub
Release](https://github.com/Etoile-Bleu/MenSung/releases/tag/ddinter-mirror-2025-08-30),
fetched via a Wayback Machine snapshot when the live site could not be
reached securely. That mirror is redistributed under DDInter's own license,
not this project's code license; see Data License below. If DDInter's
certificate is renewed, the live site is tried first and starts succeeding
again automatically; the mirror is a fallback, not a replacement.

## Data License

MenSung's code is dual-licensed under [MIT](LICENSE-MIT) and
[Apache-2.0](LICENSE-APACHE), fully permissive, including for commercial
use. The compiled `.men` database that ships in official releases is a
separate artifact under a different license, because it embeds real
clinical data:

- Built from DDInter, licensed
  [CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/):
  attribution required, non-commercial use only, share-alike.
- `mensung-builder`'s code places no restriction on what data you compile
  with it. Anyone can build their own `.men` database from a different,
  more permissively-licensed dataset; that database would carry whatever
  license its own source data allows. The CC BY-NC-SA restriction applies
  only to the specific compiled database MenSung's official releases embed,
  because that one is built from DDInter data.

If you redistribute, deploy, or use the officially released `.men` database,
or a binary that embeds it, commercially, complying with DDInter's
non-commercial license is your responsibility. MenSung's maintainers ship
that data under the terms DDInter grants; how a downstream party
subsequently uses, redistributes, or violates that license is between that
party and DDInter, not MenSung. This is the ordinary allocation of
responsibility for any project that redistributes third-party data under
its original license, and it is the same "as is, no warranty" principle
already stated in LICENSE-MIT and LICENSE-APACHE for the code itself.

## Validation Pipeline

Every database build runs a validation pass before the `.men` file is
accepted, checking for:

- Duplicate drug entries
- Invalid or non-INN drug names
- Missing severity on any interaction record
- Corrupted or unparseable interaction records

The pipeline produces `validation-report.json` with the error and warning
counts and the total interaction count. A build with non-zero errors must not
ship.

## Golden Medical Tests

`tests/golden_cases.json` holds a fixed set of known interactions (for
example, Aspirin + Warfarin as a critical interaction, Paracetamol +
Amoxicillin as no known interaction) that every build is checked against.
Removing or weakening a known interaction in this file, without a documented
clinical reason, fails CI.

## Reporting a Medical Data Error

Use the `medical_data_error` issue template for:

- A known interaction that MenSung fails to report
- An incorrect severity level
- A wrong or outdated INN mapping
- A missing or incorrect source citation

Include the drug names (INN), what MenSung currently shows, what it should
show, and a source if you have one. These reports are treated as high
priority because they affect the ZERO FALSE NEGATIVE guarantee above.

## No Patient Data

MenSung never collects, stores, transmits, or logs patient information, and
has no telemetry. The only data it reads is drug names typed by the user
for a single lookup, and that lookup is not persisted anywhere by the
application. MenSung's only network activity at all is fetching DDInter's
public dataset when installing the database for the first time, with
explicit user confirmation; no patient-identifying information is part of
that request, and drug lookups made once the database is installed never
touch the network. See README.md's Security model section for the full
statement.
