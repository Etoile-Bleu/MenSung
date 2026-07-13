# Security Policy

MenSung is an offline medication interaction checker intended for use by
doctors, nurses, and humanitarian medical workers in environments without
reliable internet access. It does not connect to the network, does not
collect patient data, and does not send telemetry of any kind. Even so,
software defects in a clinical decision-support tool can contribute to
patient harm, so security issues are taken seriously. If you find a
vulnerability, please follow the process below so it can be addressed
responsibly before any public disclosure.

For concerns about incorrect medical data (a missing or wrong drug
interaction, wrong severity, wrong INN mapping) rather than a security
vulnerability, see [MEDICAL_DATA_POLICY.md](MEDICAL_DATA_POLICY.md) and use
the `medical_data_error` issue template instead of this process.

---

## Supported Versions

Only the latest released version receives security fixes. Older versions are
not patched.

| Version | Supported |
|---------|-----------|
| Latest stable (GitHub Releases) | Yes |
| Older releases | No -- please upgrade |

---

## Reporting a Vulnerability

**Do not open a public GitHub issue for security vulnerabilities.**

Use one of the two channels below depending on severity:

### Preferred -- GitHub Private Vulnerability Reporting

Open a private security advisory directly on GitHub. This keeps the report
confidential until a fix is ready and allows coordinated disclosure.

[Report a vulnerability](https://github.com/Etoile-Bleu/MenSung/security/advisories/new)

GitHub routes the report to the maintainer only. No other users can see it.

### Alternative -- Email

If you prefer email or if the GitHub flow does not work for you:

**tarto6351@gmail.com**

Please use the subject line `[MenSung] Security vulnerability report` and
include as much detail as possible (see template below).

---

## What to Include in Your Report

The more detail you provide, the faster the issue can be triaged and fixed.

```
Component:     (e.g. .men binary reader, fuzzy matcher, builder importer, CLI/TUI)
Version:       (output of `mensung --version` or release tag)
Severity:      (your assessment: critical / high / medium / low)
Attack vector: (local file, malformed database, crafted input)
Description:   (what the vulnerability is)
Reproduction:  (step-by-step to trigger it)
Impact:        (what an attacker can achieve, or what a corrupted result could cause clinically)
Suggested fix: (optional, but very welcome)
```

---

## Response Timeline

| Step | Target |
|------|--------|
| Acknowledge receipt | Within 48 hours |
| Initial severity assessment | Within 5 business days |
| Patch ready for review | Within 30 days for high/critical, 90 days for medium/low |
| Public disclosure | Coordinated with the reporter after patch release |

These are targets, not guarantees. Complex vulnerabilities or those requiring
data-format changes may take longer.

---

## Coordinated Disclosure

MenSung follows a coordinated (responsible) disclosure model:

1. Reporter submits the vulnerability privately.
2. Maintainer acknowledges and assesses severity.
3. A fix is developed and reviewed in a private fork or security advisory branch.
4. A patched release is published to GitHub Releases.
5. A GitHub Security Advisory is published simultaneously with the release.
6. The reporter is credited in the advisory (unless they prefer to remain anonymous).

The reporter and the maintainer agree on a disclosure date before the patch is
released. The default embargo is 90 days from the initial report, or sooner if
both parties agree.

---

## Scope

### In scope

- **`.men` binary database reader** -- out-of-bounds reads, integer overflows
  in offset/length handling, checksum bypass, zero-copy parsing vulnerabilities
- **Builder pipeline** -- injection via imported OpenFDA/RxNorm/WHO source
  data, path traversal in importer file handling
- **Fuzzy matcher** -- denial-of-service via pathological input, panics on
  malformed or adversarial drug name strings
- **CLI/TUI** -- command injection via flags or arguments, path traversal in
  data file handling
- **Supply chain** -- malicious or compromised dependencies, unpinned build
  inputs affecting reproducibility

### Out of scope

- Vulnerabilities in third-party dependencies (report those upstream; we will
  update the dependency promptly on notification)
- Attacks that require physical access to the device after the device has
  already been compromised at the OS level
- Issues in unreleased branches or experimental features not in the stable
  release
- Social engineering attacks against the maintainer
- Incorrect clinical content (wrong interaction, wrong severity) -- see
  [MEDICAL_DATA_POLICY.md](MEDICAL_DATA_POLICY.md) instead

---

## Safe Harbor

MenSung is an open-source project maintained by an individual. Good-faith
security research is welcome and will not result in legal action.

Specifically: if you discover a vulnerability while using MenSung
legitimately, report it responsibly following this policy, and do not exploit
it beyond what is necessary to demonstrate the issue, you will not face any
legal threat from this project.

---

## Credits

Researchers who responsibly disclose vulnerabilities are credited in the
[GitHub Security Advisory](https://github.com/Etoile-Bleu/MenSung/security/advisories)
for the corresponding issue, with their consent.
