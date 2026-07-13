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

## File layout

A `.men` file is five contiguous regions, in this order:

```
+--------------------+
| Header (128 bytes) |
+--------------------+
| String Table       |
+--------------------+
| Drug Table         |
+--------------------+
| Interaction Index  |
+--------------------+
| Interaction Records|
+--------------------+
```

Every section other than the header is located by an absolute byte offset
and a byte length recorded in the header. A reader never needs to guess
where a section starts; it never scans for one.

## Header

Fixed size, 128 bytes, for format version 1.

| Offset | Size | Field | Type | Description |
|-------:|-----:|-------|------|-------------|
| 0 | 4 | `magic` | `[u8; 4]` | ASCII `"MEN1"`. Any other value means this is not a `.men` file. |
| 4 | 2 | `format_version` | `u16` | `1` for this specification. A reader that does not recognize the version refuses to load the file rather than guessing at the layout. |
| 6 | 2 | `header_len` | `u16` | Total header size in bytes, `128` for version 1. Lets a future reader locate section data correctly even if a later format version grows the header. |
| 8 | 4 | `header_crc32` | `u32` | CRC32 (IEEE 802.3) over every header byte except this field: bytes `[0, 8)` followed by bytes `[12, header_len)`. |
| 12 | 8 | `build_timestamp` | `u64` | Unix seconds, when `mensung-builder` produced this file. |
| 20 | 32 | `payload_sha256` | `[u8; 32]` | SHA-256 over every byte from `header_len` to end of file, that is, the String Table, Drug Table, Interaction Index, and Interaction Records combined. |
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
| 124 | 4 | `reserved` | `[u8; 4]` | Zero. Reserved for format version 2. |

Two integrity checks, deliberately kept separate:

- `header_crc32` catches a corrupted or truncated header immediately, before
  any offset in it is trusted for the rest of the parse.
- `payload_sha256` catches corruption anywhere in the data the header
  points to. CRC32 is fast but not what you want guarding hundreds of
  thousands of interaction records; SHA-256 is affordable here because it
  runs once at startup, not once per lookup, and a `.men` file is capped at
  10MB by the binary size budget.

A reader validates both checks before reading a single drug or interaction
record. If either fails, loading returns an error; it never falls back to
reading a partially-trusted file.

## String Table

A flat byte buffer holding UTF-8 drug names, referenced by other sections as
`(offset, length)` pairs into this table. There are no separators and no
null terminators; every reference carries its own exact length. INN names
stored here still obey the character rules enforced by `mensung-domain`'s
`InnName::parse` (ASCII letters, spaces, and hyphens only) because the
builder never writes a name into this table without validating it first.

## Drug Table

An array of fixed-size 12-byte records, one per drug, **sorted ascending by
name bytes** (lexicographic, byte-wise comparison over the referenced String
Table slice). This ordering is what makes an exact-name lookup a binary
search instead of a linear scan.

| Offset | Size | Field | Type | Description |
|-------:|-----:|-------|------|-------------|
| 0 | 4 | `drug_id` | `u32` | Matches `DrugId::value()` in `mensung-domain`. |
| 4 | 4 | `name_offset` | `u32` | Offset into the String Table. |
| 8 | 2 | `name_len` | `u16` | Length in bytes. Bounded by `InnName`'s 128-character limit, so `u16` is never close to overflowing. |
| 10 | 2 | `reserved` | `u16` | Zero. |

## Interaction Index

An array of fixed-size 16-byte records, one per known interaction, **sorted
ascending by the tuple `(drug_id_lower, drug_id_higher)`**. Because
`mensung-domain::DrugPair` already canonicalizes a pair so the lower id
always comes first, this ordering is unambiguous and gives a pair lookup a
binary search over a composed 64-bit key
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
`(record_offset, record_len)` pair from the Interaction Index. A record's
first 24 bytes are fixed, followed by the description and source text.

| Offset | Size | Field | Type | Description |
|-------:|-----:|-------|------|-------------|
| 0 | 4 | `interaction_id` | `u32` | Matches `InteractionId::value()`. |
| 4 | 4 | `drug_id_lower` | `u32` | Redundant with the index entry, present so a record is self-describing if read in isolation. |
| 8 | 4 | `drug_id_higher` | `u32` | Same as above. |
| 12 | 1 | `severity` | `u8` | `0` = Contraindicated, `1` = HighRisk, `2` = Moderate, `3` = Minor, `4` = Unknown. Matches `Severity`'s variant order in `mensung-domain`. |
| 13 | 1 | `evidence` | `u8` | `0` = Established, `1` = Probable, `2` = Theoretical. Matches `EvidenceLevel`. |
| 14 | 2 | `reserved` | `u16` | Zero. |
| 16 | 4 | `description_len` | `u32` | Length in bytes of the description text that follows. |
| 20 | `description_len` | `description` | `[u8]` | UTF-8, matches `Interaction::description()`. |
| 20 + `description_len` | 4 | `source_len` | `u32` | Length in bytes of the source citation that follows. |
| 24 + `description_len` | `source_len` | `source` | `[u8]` | UTF-8, matches `Interaction::source()`. |

The full record length is `24 + description_len + source_len`, and must
equal the `record_len` recorded in the Interaction Index entry that points
to it. `mensung-db` checks this before returning the record; a mismatch is
treated as corruption, the same as a failed checksum.

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
  reordering a section) requires a new `format_version` and a new major
  version of MenSung; it is never done silently within version 1.

## Reference example

The database in the README's example, containing only the Aspirin-Warfarin
interaction, would lay out as:

1. Header, 128 bytes.
2. String Table: `"Aspirin"` (7 bytes) followed by `"Warfarin"` (8 bytes),
   15 bytes total.
3. Drug Table: two 12-byte records, sorted by name, so `"Aspirin"` (whichever
   `DrugId` it was assigned) comes before `"Warfarin"`.
4. Interaction Index: one 16-byte record for the canonicalized pair.
5. Interaction Records: one variable-length record carrying the
   `Contraindicated` severity, the bleeding-risk description, and the
   source citation.
