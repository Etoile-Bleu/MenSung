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

The builder pipeline (`mensung-builder`) imports and compiles data from:

- OpenFDA
- RxNorm
- WHO datasets

Every source is public and free to redistribute. No proprietary or
license-restricted drug databases are used, so the compiled `.men` database
can be freely shared in the field without licensing concerns.

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

MenSung never collects, stores, transmits, or logs patient information. It
has no network access and no telemetry. The only data it reads is drug names
typed by the user for a single lookup, and that lookup is not persisted
anywhere by the application.
