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

### OpenFDA Drug Labeling

`mensung-builder` also has a working, tested importer for
[OpenFDA's drug label API](https://open.fda.gov/apis/drug/label/)
(`openfda.rs`, `openfda_download.rs`), enriching drugs MenSung already
knows about with contraindications, boxed warnings, warnings, pregnancy
and breastfeeding guidance, dosage, and approved indications, each
becoming a `DrugFact` (see Trust and Conflict Resolution below). Verified
against the real live API while building it, not assumed: field names,
the `openfda.generic_name` shape, and the `effective_time` date format
were all checked against a real response and against
[FDA's own published schema](https://github.com/FDA/openfda/blob/master/schemas/druglabel_schema.json).

OpenFDA's full drug/label bulk export is 260,530 records across 14 zipped
JSON files, about 1.8GB compressed (checked directly via
`api.fda.gov/download.json`). Almost none of that is relevant to a tool
that only cares about drugs already in its INN drug list, so this fetches
one drug at a time through the live search API instead, paced under
openFDA's unauthenticated rate limit (40 requests/minute) rather than
requiring an API key.

Because OpenFDA's `generic_name` field is a specific product's label name
and usually includes a salt or ester form the INN name does not
("WARFARIN SODIUM" vs "Warfarin"), the importer only accepts a match when
every word of the INN name is an exact, case-insensitive prefix of the
label's generic name, never a substring or fuzzy match; a false match
here would silently attach one drug's warnings to a different drug. Any
label field OpenFDA does not represent as a single dedicated field on the
newer label format (breastfeeding guidance folded into a subsection with
no field of its own, on some labels) is skipped rather than guessed at.

The `.men` format (see docs/DATABASE_FORMAT.md) can persist `DrugFact`s
since format version 2, but this importer is not yet wired into
`mensung-client`'s runtime install flow: `data.rs`'s `install()` still
only fetches and compiles DDInter. Running OpenFDA's rate-limited,
one-request-per-drug fetch for every installed drug takes real time (see
`openfda_download.rs`'s pacing) and needs its own progress UX, not silent
minutes-long waiting, so wiring it in is deliberately left for a separate
pass; see ROADMAP.md's Phase 8b. Today this importer exists as tested,
working `mensung-builder` code, verified end to end against real label
data for real drugs.

The original integration plan also named DailyMed alongside OpenFDA for
this same kind of data (contraindications, warnings, pregnancy, dosage,
official drug labels). DailyMed is not separately integrated: checked
directly, DailyMed and openFDA's drug label API are both built from the
same underlying FDA Structured Product Labeling (SPL) submissions,
openFDA's own stated purpose being to provide an API on top of that data,
not a second, different dataset. A separate DailyMed importer would parse
the same source content a second time for no additional coverage; see
the PubChem/ChEBI reasoning below for the same principle applied to a
different pair of sources.

### RxNorm Identity Normalization

`mensung-builder` also has a working, tested lookup for
[RxNorm's REST API](https://lhncbc.nlm.nih.gov/RxNav/APIs/RxNormAPIs.html)
(`rxnorm.rs`, `rxnorm_download.rs`), attaching an RxCUI (RxNorm Concept
Unique Identifier) to each drug MenSung already knows about, so a drug
record can be cross-referenced against RxNorm, DailyMed, and any other
system that keys on RxCUI rather than a name string. `Drug::rxcui()` is
`None` until a build actually runs this lookup and compiles the result
in; the `.men` format can persist it since format version 2, but, like
the OpenFDA importer above, this is not yet wired into
`mensung-client`'s runtime install flow, only tested and verified
against real data.

Lookups use RxNorm's own "normalized" search mode (`rxcui.json?...&search=1`),
which already accounts for salt forms, word order, and common
abbreviations server-side, checked directly against real responses rather
than assumed. This is deliberately the only matching this project does
for RxNorm: layering MenSung's own fuzzy matching on top of a service
that already does conservative, documented normalization would only add
false-match risk, not reduce it. A drug RxNorm has no match for is left
without an RxCUI, not guessed at. Requests are paced at 10 per second,
half of RxNorm's own stated limit of 20 requests per second per IP
address (checked directly against RxNorm's Terms of Service).

### PubChem Chemical Reference Data (and why not ChEBI too)

`mensung-builder` also has a working, tested lookup for
[PubChem's PUG REST API](https://pubchem.ncbi.nlm.nih.gov/docs/pug-rest)
(`pubchem.rs`, `pubchem_download.rs`), attaching each drug's PubChem CID,
molecular formula, molecular weight, and IUPAC name as
`ChemicalProperties`, using the same not-yet-wired-into-`mensung-client`
pattern as OpenFDA and RxNorm above. This is reference chemistry information, not
a clinical assertion: it carries no severity or evidence level and does
not go through the `Claim`/`Source` conflict resolution model, since
there is nothing to resolve a disagreement about.

The original integration plan also named ChEBI (a second chemical
ontology and identifier system, chebi.ebi.ac.uk) alongside PubChem.
ChEBI is not integrated, and is not currently planned: it serves largely
the same purpose PubChem already does here (a chemical identifier plus
basic structural data), and MenSung has no feature, planned or existing,
that would use a second, overlapping chemical ontology once it already
has one. Adding ChEBI on top of PubChem would be duplicated integration
work for data this project does not use, not a clinically meaningful gap
the way a second interaction or label source would be; per
GOOD_PRACTICE.md, this is not built until an actual need for it exists.
Molecular weight is kept as PubChem's own decimal string rather than
parsed into a float, for the same reason a `Claim`'s rationale stays a
`String` rather than being reformatted: this project only ever displays
it, never computes with it, and a float round-trip risks showing a
subtly different number than the source gave.

### WHO ATC Therapeutic Classification

`mensung-builder` also has a working, tested lookup for WHO ATC
(Anatomical Therapeutic Chemical) classification codes (`atc.rs`,
`atc_download.rs`), attaching zero or more `AtcCode`s to each drug, e.g.
`B01AA` ("Vitamin K antagonists") for warfarin. Like the sources above,
this is tested and verified against real data but not yet wired into
`mensung-client`'s runtime install flow.

WHO's own [ATC/DDD Index](https://www.whocc.no/atc_ddd_index/) has no
bulk download or programmatic API, checked directly: it is a name-search
web page only. This project instead reaches ATC codes through
[NLM's RxClass API](https://lhncbc.nlm.nih.gov/RxNav/APIs/RxClassAPIs.html),
which cross-references RxNorm concepts to ATC, served from the same
`rxnav.nlm.nih.gov` infrastructure and Terms of Service as RxNorm itself.
This makes ATC lookup depend on a drug already having an RxCUI from the
RxNorm integration above, a real pipeline dependency, not a workaround: a
drug RxNorm could not resolve has no RxCUI to look up an ATC
classification for.

A single drug can carry more than one ATC code (aspirin is classified
both as a platelet aggregation inhibitor and as a salicylate analgesic,
depending on use), so `Drug::atc_codes()` returns a list, not an
`Option`, unlike `rxcui()`. RxClass can also return a classification
entry for a related combination-product RxCUI (`aspirin / codeine`) in
the same response as the plain ingredient being asked about; verified
directly, and filtered out by keeping only entries whose RxCUI matches
the one actually queried, so a combination product's classification
never silently attaches to the plain ingredient's record.

## Trust and Conflict Resolution

DDInter is currently the only source compiled into the databases
`mensung-client` actually installs. OpenFDA's, RxNorm's, PubChem's, and
WHO ATC's importers all exist, are verified against real data, and the
`.men` format (version 2, see docs/DATABASE_FORMAT.md) can persist
everything they produce, but none of the four is yet called from
`mensung-client`'s runtime install flow; see the Data Sources subsections
above and ROADMAP.md's Phase 8b for why that wiring is a separate,
not-yet-done piece of work. The domain layer (`mensung-domain`) already
models what happens once a second source is compiled in and disagrees
with the first; this section documents that model now so the policy
exists before that happens, not after.

Every clinical fact, an interaction between two drugs or a fact about a
single drug (contraindication, boxed warning, pregnancy/breastfeeding
guidance, dosage, indication), is represented as one or more `Claim`s, one
per contributing source. A `Claim` carries its `Source` (with a stable id,
a display name, and a trust tier), a `Severity`, an `EvidenceLevel`, a
`Confidence`, a human-readable rationale, and the date it was last
confirmed against that source. `InteractionFact` and `DrugFact` hold the
full list of claims for a given fact. **No claim is ever discarded because
another source disagrees with it**; this is the same zero-discard principle
the Zero False Negative Policy above already applies to interactions, now
extended to sources.

Source tiers, most to least authoritative:

1. Official regulatory sources (FDA, EMA, ANSM, an official SmPC/RCP label)
2. Clinical practice guidelines and expert body recommendations
3. Curated pharmaceutical databases (DDInter, DailyMed, OpenFDA)
4. Secondary or reference material

When claims disagree, `primary_claim()` picks the claim from the most
authoritative tier present. If more than one claim shares that top tier and
they still disagree, the tie is broken toward the **more severe** reading,
never the milder one, so two equally authoritative sources that disagree
can never quietly resolve to the safer-looking answer. This is what
`InteractionFact::resolve()` and `DrugFact::primary_claim()` implement in
`crates/mensung-domain/src/interaction_fact.rs` and `drug_fact.rs`.

Severity itself is a four-tier clinical scale, not a boolean:

| `Severity` variant | `clinical_meaning()` |
| --- | --- |
| `Contraindicated` | Absolute contraindication |
| `HighRisk` | Strongly discouraged |
| `Moderate` | Use with caution / monitoring required |
| `Minor` | Informational / minor interaction |
| `Unknown` | Severity not specified by the source |

The `.men` format has persisted every claim, not just the resolved one,
since format version 2 (see docs/DATABASE_FORMAT.md): `mensung_db`'s
`InteractionRecord::claims()` and `DrugFactRecord::claims()` return the
full list read back from disk. The CLI and TUI still only ever call the
resolved-view accessors (`severity()`, `description()`, `evidence()`,
`source()`), the same names `InteractionFact::resolve()` and
`InteractionRecord`'s v1-compatible accessors already used, so showing
more than the resolved view is a display design question, not a storage
one; see ROADMAP.md's Phase 8b. DDInter, the only source actually
compiled into an installed database today, only ever produces a single
claim per interaction, so this has not yet been exercised against a real
multi-claim record outside this domain layer's own test suite.

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
