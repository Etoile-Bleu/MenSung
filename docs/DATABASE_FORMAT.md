# The `.men` Binary Database Format

This document is the authoritative specification for the `.men` file format.
`mensung-builder` writes it, `mensung-db` reads it. If the two ever disagree,
this document is what they are both wrong against, and one of the two crates
has a bug.

## Design goals

- **Zero-copy.** `mensung-db` reads directly from the loaded byte buffer.
  Nothing is deserialized into an owned, allocated structure on the lookup
  path. Every accessor returns a slice or a value parsed on demand from a
  fixed offset.
- **Fail closed.** A corrupt, truncated, or unrecognized-version file is
  rejected with a typed error at load time, before any lookup is attempted.
  MenSung never silently loads a partially valid database.
- **No unsafe required.** Every field is fixed-width and byte-aligned in a
  way that `u16::from_le_bytes` / `u32::from_le_bytes` / `u64::from_le_bytes`
  can parse directly from a slice. There is no pointer casting and no
  reliance on memory alignment, so the reader in `mensung-db` has no reason
  to reach for `unsafe`, per the unsafe policy in `GOOD_PRACTICE.md`.
- **Deterministic.** Given the same input dataset, `mensung-builder` produces
  byte-identical output every time: sections appear in the same order, and
  every sorted section is sorted by the same key. This is what makes
  reproducible builds possible.

## Endianness

Every multi-byte integer field in this format is little-endian, regardless
of the host architecture writing or reading it. This is fixed for the life
of the format, it does not vary by platform or by a flag in the header.
Little-endian was chosen because every target platform in scope (x86_64,
and ARM in a future phase) is little-endian by default, so this avoids a
byte-swap on the lookup hot path on every platform that matters.

## Format version 2

Version 2 is the current format. It exists to persist the multi-source
`Claim`/`InteractionFact`/`DrugFact` model from `mensung-domain` (see
MEDICAL_DATA_POLICY.md's Trust and Conflict Resolution section) and the
RxCUI/PubChem/ATC cross-reference data attached to `Drug`, none of which
version 1 had anywhere to store. No `.men` file has ever shipped in a
release (`ROADMAP.md`'s Phase 11 is still open), so version 2 replaces
version 1 outright rather than requiring a reader that understands both;
a version 1 file is simply an unsupported version now, rejected the same
way a version 3 file will be once a version 3 exists.

Two things worth calling out about what changed and why:

- Version 1's Interaction Records inlined `description` and `source` text
  directly in each variable-length record. With DDInter's real dataset,
  that text mostly repeats verbatim across the 160,235 records (see
  `MEDICAL_DATA_POLICY.md`'s Data Sources section), producing a ~28MB
  file, the known tradeoff Phase 5 flagged as needing exactly this fix.
  Version 2 moves every string, including claim rationale, source names,
  and ATC class names, into the shared String Table, referenced by
  `(offset, length)`, and the writer deduplicates identical strings
  before writing them, so a repeated description or a repeated source
  name costs one String Table entry, not one per record.
- Because every string is now a String Table reference, a `Claim` is a
  fixed-size record. This makes an Interaction Record or Drug Fact Record
  a short fixed prefix followed by a flat array of fixed-size `Claim`
  entries, one per contributing source, instead of the free-form
  variable-length encoding version 1 used. `claim_count` is a `u16`;
  nothing in this dataset is realistically expected to approach that.

## File layout

A `.men` file is nine contiguous regions, in this order:

```
+-----------------------+
| Header (192 bytes)    |
+-----------------------+
| String Table          |
+-----------------------+
| Drug Table            |
+-----------------------+
| ATC Code Table        |
+-----------------------+
| Interaction Index     |
+-----------------------+
| Interaction Records   |
+-----------------------+
| Drug Fact Index       |
+-----------------------+
| Drug Fact Records     |
+-----------------------+
```

Every section other than the header is located by an absolute byte offset
and a byte length recorded in the header. A reader never needs to guess
where a section starts; it never scans for one.

## Header

Fixed size, 192 bytes, for format version 2.

| Offset | Size | Field | Type | Description |
|-------:|-----:|-------|------|-------------|
| 0 | 4 | `magic` | `[u8; 4]` | ASCII `"MEN1"`. Any other value means this is not a `.men` file. Unchanged from version 1: the magic identifies the family of formats, `format_version` identifies which one. |
| 4 | 2 | `format_version` | `u16` | `2` for this specification. A reader that does not recognize the version refuses to load the file rather than guessing at the layout. |
| 6 | 2 | `header_len` | `u16` | Total header size in bytes, `192` for version 2. Lets a future reader locate section data correctly even if a later format version grows the header. |
| 8 | 4 | `header_crc32` | `u32` | CRC32 (IEEE 802.3) over every header byte except this field: bytes `[0, 8)` followed by bytes `[12, header_len)`. |
| 12 | 8 | `build_timestamp` | `u64` | Unix seconds, when `mensung-builder` produced this file. |
| 20 | 32 | `payload_sha256` | `[u8; 32]` | SHA-256 over every byte from `header_len` to end of file, that is, every section after the header, combined. |
| 52 | 4 | `drug_count` | `u32` | Number of records in the Drug Table. |
| 56 | 4 | `interaction_count` | `u32` | Number of records in the Interaction Index (and in Interaction Records). |
| 60 | 8 | `string_table_offset` | `u64` | Absolute byte offset of the String Table. |
| 68 | 8 | `string_table_len` | `u64` | Byte length of the String Table. |
| 76 | 8 | `drug_table_offset` | `u64` | Absolute byte offset of the Drug Table. |
| 84 | 8 | `drug_table_len` | `u64` | Byte length of the Drug Table. |
| 92 | 8 | `interaction_index_offset` | `u64` | Absolute byte offset of the Interaction Index. |
| 100 | 8 | `interaction_index_len` | `u64` | Byte length of the Interaction Index. |
| 108 | 8 | `interaction_records_offset` | `u64` | Absolute byte offset of the Interaction Records section. |
| 116 | 8 | `interaction_records_len` | `u64` | Byte length of the Interaction Records section. |
| 124 | 4 | `reserved_v1` | `[u8; 4]` | Zero. This was version 1's `reserved` field; version 2 does not reuse it, to keep every version-1-era offset meaning exactly what it meant before. |
| 128 | 8 | `atc_table_offset` | `u64` | Absolute byte offset of the ATC Code Table. |
| 136 | 8 | `atc_table_len` | `u64` | Byte length of the ATC Code Table. |
| 144 | 4 | `atc_table_count` | `u32` | Number of records in the ATC Code Table. |
| 148 | 8 | `drug_fact_index_offset` | `u64` | Absolute byte offset of the Drug Fact Index. |
| 156 | 8 | `drug_fact_index_len` | `u64` | Byte length of the Drug Fact Index. |
| 164 | 4 | `drug_fact_count` | `u32` | Number of records in the Drug Fact Index (and in Drug Fact Records). |
| 168 | 8 | `drug_fact_records_offset` | `u64` | Absolute byte offset of the Drug Fact Records section. |
| 176 | 8 | `drug_fact_records_len` | `u64` | Byte length of the Drug Fact Records section. |
| 184 | 8 | `reserved` | `[u8; 8]` | Zero. Reserved for format version 3. |

The same two integrity checks as version 1, unchanged in meaning:

- `header_crc32` catches a corrupted or truncated header immediately, before
  any offset in it is trusted for the rest of the parse.
- `payload_sha256` catches corruption anywhere in the data the header
  points to. CRC32 is fast but not what you want guarding hundreds of
  thousands of records; SHA-256 is affordable here because it runs once at
  startup, not once per lookup.

A reader validates both checks before reading a single drug, interaction, or
drug fact record. If either fails, loading returns an error; it never falls
back to reading a partially-trusted file.

## String Table

A flat byte buffer holding every UTF-8 string referenced by any other
section: drug names, RxCUIs, molecular formulas, molecular weights, IUPAC
names, ATC class names, source ids, source names, and claim rationale.
Referenced as `(offset, length)` pairs into this table; there are no
separators and no null terminators, every reference carries its own exact
length. `mensung-builder` deduplicates identical strings when writing this
table: a rationale or source name repeated across many records is written
once and referenced by every record that uses it, which is what keeps a
dataset like DDInter's, where the synthesized description repeats across
most of its 160,235 interactions, from bloating the file the way version 1
did. INN names stored here still obey the character rules enforced by
`mensung-domain`'s `InnName::parse` because the builder never writes a name
into this table without validating it first.

## Drug Table

An array of fixed-size 48-byte records, one per drug, **sorted ascending by
name bytes** (lexicographic, byte-wise comparison over the referenced String
Table slice). This ordering is what makes an exact-name lookup a binary
search instead of a linear scan.

| Offset | Size | Field | Type | Description |
|-------:|-----:|-------|------|-------------|
| 0 | 4 | `drug_id` | `u32` | Matches `DrugId::value()` in `mensung-domain`. |
| 4 | 4 | `name_offset` | `u32` | Offset into the String Table. |
| 8 | 2 | `name_len` | `u16` | Length in bytes. Bounded by `InnName`'s 128-character limit, so `u16` is never close to overflowing. |
| 10 | 2 | `reserved` | `u16` | Zero. |
| 12 | 4 | `rxcui_offset` | `u32` | Offset into the String Table. Meaningless when `rxcui_len` is `0`. |
| 16 | 2 | `rxcui_len` | `u16` | Length in bytes of the RxCUI digit string. `0` means this drug has no `Rxcui` (`mensung_domain::Drug::rxcui() == None`), not an error. |
| 18 | 4 | `pubchem_cid` | `u32` | PubChem Compound ID. `0` means this drug has no `ChemicalProperties`; PubChem CIDs are always positive, so `0` is an unambiguous absence sentinel. |
| 22 | 4 | `molecular_formula_offset` | `u32` | Offset into the String Table. Meaningless when `molecular_formula_len` is `0`. |
| 26 | 2 | `molecular_formula_len` | `u16` | `0` means this drug has no `ChemicalProperties` at all (the formula is the group's required field; `pubchem_cid` and this field are always both zero or both nonzero together). |
| 28 | 4 | `molecular_weight_offset` | `u32` | Offset into the String Table. Meaningless when `molecular_weight_len` is `0`. |
| 32 | 2 | `molecular_weight_len` | `u16` | PubChem's own decimal string, e.g. `"308.3"`, not a parsed float; see `mensung_domain::ChemicalProperties`'s header for why. |
| 34 | 4 | `iupac_name_offset` | `u32` | Offset into the String Table. Meaningless when `iupac_name_len` is `0`. |
| 38 | 2 | `iupac_name_len` | `u16` | `0` means no IUPAC name, independently of whether the rest of `ChemicalProperties` is present (`ChemicalProperties::iupac_name()` is itself an `Option`). |
| 40 | 4 | `atc_start_index` | `u32` | Index (not byte offset) of this drug's first entry in the ATC Code Table. Meaningless when `atc_count` is `0`. |
| 44 | 2 | `atc_count` | `u16` | Number of consecutive ATC Code Table entries, starting at `atc_start_index`, belonging to this drug. `0` means no ATC classification. |
| 46 | 2 | `reserved` | `u16` | Zero. |

## ATC Code Table

An array of fixed-size 12-byte records. Entries belonging to the same drug
are contiguous, in the order `mensung_domain::Drug::atc_codes()` returns
them, addressed by that drug's `atc_start_index`/`atc_count` in the Drug
Table; there is no separate sort key or index structure for this table, a
drug's own Drug Table record is the only way to reach its entries.

| Offset | Size | Field | Type | Description |
|-------:|-----:|-------|------|-------------|
| 0 | 5 | `code` | `[u8; 5]` | ASCII, matches `AtcCode::code()`'s fixed five-character shape (one uppercase letter, two digits, two uppercase letters), so it is inlined directly rather than stored as a String Table reference. |
| 5 | 1 | `reserved` | `u8` | Zero. |
| 6 | 4 | `class_name_offset` | `u32` | Offset into the String Table. |
| 10 | 2 | `class_name_len` | `u16` | Length in bytes of `AtcCode::class_name()`. |

## Claim Encoding

A fixed-size 28-byte encoding of one `mensung_domain::Claim`, used
identically by both Interaction Records and Drug Fact Records below. A
claim is never variable-length: every text field it carries is a String
Table reference.

| Offset | Size | Field | Type | Description |
|-------:|-----:|-------|------|-------------|
| 0 | 4 | `source_id_offset` | `u32` | Offset into the String Table. |
| 4 | 2 | `source_id_len` | `u16` | Length of `Claim::source().id()`. |
| 6 | 4 | `source_name_offset` | `u32` | Offset into the String Table. |
| 10 | 2 | `source_name_len` | `u16` | Length of `Claim::source().name()`. |
| 12 | 1 | `source_tier` | `u8` | `0` = Regulatory, `1` = ClinicalGuideline, `2` = CuratedDatabase, `3` = Secondary. Matches `SourceTier`'s variant order. |
| 13 | 1 | `severity` | `u8` | `0` = Contraindicated, `1` = HighRisk, `2` = Moderate, `3` = Minor, `4` = Unknown. Matches `Severity`'s variant order, unchanged from version 1. |
| 14 | 1 | `evidence` | `u8` | `0` = Established, `1` = Probable, `2` = Theoretical. Matches `EvidenceLevel`, unchanged from version 1. |
| 15 | 1 | `confidence` | `u8` | `0` = Low, `1` = Medium, `2` = High. Matches `Confidence`'s variant order. |
| 16 | 2 | `last_updated_year` | `u16` | `ClaimDate::year()`. |
| 18 | 1 | `last_updated_month` | `u8` | `ClaimDate::month()`, `1`-`12`. |
| 19 | 1 | `last_updated_day` | `u8` | `ClaimDate::day()`. |
| 20 | 4 | `rationale_offset` | `u32` | Offset into the String Table. |
| 24 | 4 | `rationale_len` | `u32` | Length of `Claim::rationale()`. A `u32`, not `u16` like the id/name fields above: a claim's rationale can be several kilobytes of label text (an OpenFDA boxed warning, for example), while a source id or name is always short. |

## Interaction Index

Unchanged from version 1: an array of fixed-size 16-byte records, one per
known interaction, **sorted ascending by the tuple `(drug_id_lower,
drug_id_higher)`**. Because `mensung-domain::DrugPair` already canonicalizes
a pair so the lower id always comes first, this ordering is unambiguous and
gives a pair lookup a binary search over a composed 64-bit key
(`(drug_id_lower as u64) << 32 | drug_id_higher as u64`) instead of a scan.
The builder's validation pipeline is what guarantees each pair appears at
most once; the reader does not need to handle duplicate pairs because they
cannot reach a valid `.men` file.

| Offset | Size | Field | Type | Description |
|-------:|-----:|-------|------|-------------|
| 0 | 4 | `drug_id_lower` | `u32` | The smaller `DrugId` of the pair. |
| 4 | 4 | `drug_id_higher` | `u32` | The larger `DrugId` of the pair. |
| 8 | 4 | `record_offset` | `u32` | Offset into Interaction Records. |
| 12 | 4 | `record_len` | `u32` | Byte length of the record, used to validate against the length the record itself carries before trusting either. |

## Interaction Records

A sequence of variable-length records, each addressed by an
`(record_offset, record_len)` pair from the Interaction Index. Version 2
replaces version 1's single severity/description/source per record with
one or more `Claim`s, one per source asserting something about this drug
pair; see `mensung_domain::InteractionFact`.

A record's first 16 bytes are a fixed prefix, followed by `claim_count`
fixed-size Claim entries (see Claim Encoding above); there is no other
variable-length data in a record, since every string a claim carries is
now a String Table reference.

| Offset | Size | Field | Type | Description |
|-------:|-----:|-------|------|-------------|
| 0 | 4 | `interaction_id` | `u32` | Matches `InteractionFact::id()`'s `InteractionId::value()`. |
| 4 | 4 | `drug_id_lower` | `u32` | Redundant with the index entry, present so a record is self-describing if read in isolation. |
| 8 | 4 | `drug_id_higher` | `u32` | Same as above. |
| 12 | 2 | `claim_count` | `u16` | Number of Claim entries that follow. Always at least `1`; `mensung_domain::InteractionFact::new` rejects zero claims. |
| 14 | 2 | `reserved` | `u16` | Zero. |
| 16 | `claim_count * 28` | `claims` | `[Claim; claim_count]` | See Claim Encoding above. |

The full record length is `16 + claim_count * 28`, and must equal the
`record_len` recorded in the Interaction Index entry that points to it.
`mensung-db` checks this before returning the record; a mismatch is treated
as corruption, the same as a failed checksum.

`mensung-db`'s `find_interaction` continues to resolve a record down to the
single `Interaction` shape the CLI/TUI display, using the same
`primary_claim()` tie-break rule `InteractionFact::resolve()` implements in
`mensung-domain`: the claim from the most authoritative source tier
present, and the most severe of those on a tie. The other claims are not
discarded by the reader, only by that convenience accessor; a caller that
wants the full multi-source view reads every claim in the record.

## Drug Fact Index

An array of fixed-size 16-byte records, one per known `DrugFact`, **sorted
ascending by the tuple `(drug_id, kind)`**. A single drug can have more
than one `DrugFact` (a contraindication and a boxed warning are different
facts about the same drug), so, unlike the Interaction Index, this index
does not resolve to at most one match: a lookup finds the first entry with
a given `drug_id` by binary search, then scans forward while `drug_id`
still matches, since all of a drug's entries are contiguous once sorted
this way.

| Offset | Size | Field | Type | Description |
|-------:|-----:|-------|------|-------------|
| 0 | 4 | `drug_id` | `u32` | Matches `DrugFact::drug()`'s `DrugId::value()`. |
| 4 | 1 | `kind` | `u8` | `0` = Contraindication, `1` = Warning, `2` = BoxedWarning, `3` = Pregnancy, `4` = Breastfeeding, `5` = Dosage, `6` = Indication. Matches `DrugFactKind`'s variant order. |
| 5 | 3 | `reserved` | `[u8; 3]` | Zero. |
| 8 | 4 | `record_offset` | `u32` | Offset into Drug Fact Records. |
| 12 | 4 | `record_len` | `u32` | Byte length of the record, validated the same way an Interaction Index entry's is. |

## Drug Fact Records

A sequence of variable-length records, each addressed by an
`(record_offset, record_len)` pair from the Drug Fact Index. Same shape as
an Interaction Record, except keyed to one drug instead of a pair, and
carrying the fact's `kind`.

| Offset | Size | Field | Type | Description |
|-------:|-----:|-------|------|-------------|
| 0 | 4 | `drug_fact_id` | `u32` | Matches `DrugFact::id()`'s `DrugFactId::value()`. |
| 4 | 4 | `drug_id` | `u32` | Redundant with the index entry, present so a record is self-describing if read in isolation. |
| 8 | 1 | `kind` | `u8` | Same encoding as the Drug Fact Index's `kind` field, redundant with it for the same reason `drug_id` is. |
| 9 | 1 | `reserved` | `u8` | Zero. |
| 10 | 2 | `claim_count` | `u16` | Number of Claim entries that follow. Always at least `1`; `mensung_domain::DrugFact::new` rejects zero claims. |
| 12 | `claim_count * 28` | `claims` | `[Claim; claim_count]` | See Claim Encoding above. |

The full record length is `12 + claim_count * 28`, validated against the
Drug Fact Index entry's `record_len` the same way an Interaction Record is.

## Forward compatibility

`format_version` and `header_len` exist specifically so a future format
version can add fields without breaking a reader that only understands an
earlier version's fixed offsets. The rule for evolving this format is:

- New header fields are appended in the `reserved` space, or `header_len`
  grows to make room. Existing offsets never move and existing fields never
  change meaning.
- A reader refuses to load a file whose `format_version` it does not
  recognize, rather than guessing at how to interpret unfamiliar bytes. This
  is the same fail-closed principle as a bad checksum: an offline medical
  tool that cannot verify what it is about to show a clinician must say so,
  not show something anyway.
- A breaking layout change (removing a field, changing a field's meaning,
  reordering a section) requires a new `format_version`; it is never done
  silently within an existing version. Version 1 to version 2 was exactly
  such a change (the Interaction Records layout is incompatible), which is
  why it bumped `format_version` rather than trying to stay within version
  1's `reserved` space.

## Reference example

The database in the README's example, containing only the Aspirin-Warfarin
interaction from DDInter (one claim, no drug facts, no ATC codes, no
RxCUI/chemical properties), would lay out as:

1. Header, 192 bytes.
2. String Table: `"Aspirin"`, `"Warfarin"`, `"ddinter"` (the source id),
   `"DDInter (http://ddinter.scbdd.com/)"` (the source name), and the
   claim's rationale text, each written once.
3. Drug Table: two 48-byte records, sorted by name, so `"Aspirin"`
   (whichever `DrugId` it was assigned) comes before `"Warfarin"`, both
   with every optional field's length at `0`.
4. ATC Code Table: empty, `atc_table_count` is `0`.
5. Interaction Index: one 16-byte record for the canonicalized pair.
6. Interaction Records: one record, `claim_count = 1`, carrying the single
   DDInter-sourced claim: `Contraindicated` severity, the bleeding-risk
   rationale, and the DDInter source.
7. Drug Fact Index: empty, `drug_fact_count` is `0`.
8. Drug Fact Records: empty.
