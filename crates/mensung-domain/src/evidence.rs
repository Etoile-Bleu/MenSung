//! Strength of the clinical evidence backing an interaction record. Recorded
//! alongside a source citation, per MEDICAL_DATA_POLICY.md, so a medical
//! worker can judge how much weight to give the result.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvidenceLevel {
    Established,
    Probable,
    Theoretical,
}

impl std::fmt::Display for EvidenceLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            EvidenceLevel::Established => "Established",
            EvidenceLevel::Probable => "Probable",
            EvidenceLevel::Theoretical => "Theoretical",
        };
        f.write_str(label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_uses_a_human_readable_label() {
        assert_eq!(EvidenceLevel::Established.to_string(), "Established");
        assert_eq!(EvidenceLevel::Theoretical.to_string(), "Theoretical");
    }
}
