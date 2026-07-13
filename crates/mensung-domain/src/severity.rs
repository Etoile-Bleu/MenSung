//! Clinical severity of a drug-drug interaction. The ordering below ranks
//! from most to least dangerous and only controls sort/display priority; it
//! never controls whether an interaction is returned. The zero false
//! negative policy in MEDICAL_DATA_POLICY.md applies to every variant,
//! including `Unknown`, equally. `Display` gives the short label already
//! used throughout the CLI/TUI; `clinical_meaning` gives the longer
//! clinical phrase (Absolute contraindication, Strongly discouraged, Use
//! with caution / monitoring required, Informational / minor interaction)
//! for contexts with room to show it, such as a claim's full detail view.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Contraindicated,
    HighRisk,
    Moderate,
    Minor,
    Unknown,
}

impl Severity {
    const fn rank(self) -> u8 {
        match self {
            Severity::Contraindicated => 0,
            Severity::HighRisk => 1,
            Severity::Moderate => 2,
            Severity::Minor => 3,
            Severity::Unknown => 4,
        }
    }

    pub const fn clinical_meaning(self) -> &'static str {
        match self {
            Severity::Contraindicated => "Absolute contraindication",
            Severity::HighRisk => "Strongly discouraged",
            Severity::Moderate => "Use with caution / monitoring required",
            Severity::Minor => "Informational / minor interaction",
            Severity::Unknown => "Severity not specified by the source",
        }
    }
}

impl PartialOrd for Severity {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Severity {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.rank().cmp(&other.rank())
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Severity::Contraindicated => "CONTRAINDICATED",
            Severity::HighRisk => "HIGH RISK",
            Severity::Moderate => "MODERATE",
            Severity::Minor => "MINOR",
            Severity::Unknown => "UNKNOWN",
        };
        f.write_str(label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contraindicated_outranks_every_other_severity() {
        assert!(Severity::Contraindicated < Severity::HighRisk);
        assert!(Severity::Contraindicated < Severity::Moderate);
        assert!(Severity::Contraindicated < Severity::Minor);
        assert!(Severity::Contraindicated < Severity::Unknown);
    }

    #[test]
    fn severities_sort_from_most_to_least_dangerous() {
        let mut severities = [
            Severity::Unknown,
            Severity::Minor,
            Severity::Contraindicated,
            Severity::Moderate,
            Severity::HighRisk,
        ];
        severities.sort();
        assert_eq!(
            severities,
            [
                Severity::Contraindicated,
                Severity::HighRisk,
                Severity::Moderate,
                Severity::Minor,
                Severity::Unknown,
            ]
        );
    }

    #[test]
    fn display_matches_the_expected_clinical_label() {
        assert_eq!(Severity::Contraindicated.to_string(), "CONTRAINDICATED");
        assert_eq!(Severity::HighRisk.to_string(), "HIGH RISK");
    }

    #[test]
    fn clinical_meaning_matches_the_four_tier_scale() {
        assert_eq!(
            Severity::Contraindicated.clinical_meaning(),
            "Absolute contraindication"
        );
        assert_eq!(
            Severity::HighRisk.clinical_meaning(),
            "Strongly discouraged"
        );
        assert_eq!(
            Severity::Moderate.clinical_meaning(),
            "Use with caution / monitoring required"
        );
        assert_eq!(
            Severity::Minor.clinical_meaning(),
            "Informational / minor interaction"
        );
    }
}
