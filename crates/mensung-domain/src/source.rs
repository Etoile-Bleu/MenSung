//! A named data source contributing clinical claims (DDInter, an FDA drug
//! label, a clinical guideline body, and so on), and the trust tier that
//! ranks it against other sources when claims disagree. Lower tiers are
//! more authoritative, the same convention `Severity` already uses for its
//! most-to-least-dangerous ordering, so the two orderings read the same way
//! throughout the codebase: rank 0 means "listen to this first."
//!
//! A tier ranks a *source*, not a claim. Two claims from the same tier can
//! still disagree; see `InteractionFact::primary_claim` in
//! `interaction_fact.rs` for how that disagreement is resolved without
//! discarding either claim.

use crate::DomainError;

const MAX_ID_LEN: usize = 64;
const MAX_NAME_LEN: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceTier {
    /// Official regulatory sources: FDA, EMA, ANSM, an official SmPC/RCP
    /// label.
    Regulatory,
    /// Clinical practice guidelines and expert body recommendations.
    ClinicalGuideline,
    /// Curated pharmaceutical databases: DDInter, DailyMed, OpenFDA.
    CuratedDatabase,
    /// Anything else: secondary or reference material.
    Secondary,
}

/// A short, stable, machine-readable identifier for a source, e.g.
/// `"ddinter"` or `"openfda-label"`. Lowercase ASCII letters, digits, and
/// hyphens only, so it is safe to use as a lookup key or a file/URL
/// fragment without escaping.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceId(String);

impl SourceId {
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        let trimmed = raw.trim();
        let valid = !trimmed.is_empty()
            && trimmed.len() <= MAX_ID_LEN
            && trimmed
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');

        if !valid {
            return Err(DomainError::InvalidSourceId(raw.to_string()));
        }

        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SourceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Source {
    id: SourceId,
    name: String,
    tier: SourceTier,
}

impl Source {
    pub fn new(
        id: SourceId,
        name: impl Into<String>,
        tier: SourceTier,
    ) -> Result<Self, DomainError> {
        let name = name.into();
        let trimmed = name.trim();
        if trimmed.is_empty() || trimmed.chars().count() > MAX_NAME_LEN {
            return Err(DomainError::EmptySourceName(id.to_string()));
        }

        Ok(Self {
            id,
            name: trimmed.to_string(),
            tier,
        })
    }

    pub fn id(&self) -> &SourceId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn tier(&self) -> SourceTier {
        self.tier
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiers_order_from_most_to_least_authoritative() {
        assert!(SourceTier::Regulatory < SourceTier::ClinicalGuideline);
        assert!(SourceTier::ClinicalGuideline < SourceTier::CuratedDatabase);
        assert!(SourceTier::CuratedDatabase < SourceTier::Secondary);
    }

    #[test]
    fn accepts_a_lowercase_slug() {
        assert_eq!(SourceId::parse("ddinter").unwrap().as_str(), "ddinter");
        assert_eq!(
            SourceId::parse("openfda-label").unwrap().as_str(),
            "openfda-label"
        );
    }

    #[test]
    fn rejects_uppercase_and_other_characters() {
        assert!(SourceId::parse("DDInter").is_err());
        assert!(SourceId::parse("open fda").is_err());
        assert!(SourceId::parse("open_fda").is_err());
        assert!(SourceId::parse("").is_err());
    }

    #[test]
    fn builds_a_source_with_a_name_and_tier() {
        let id = SourceId::parse("ddinter").unwrap();
        let source = Source::new(id, "DDInter", SourceTier::CuratedDatabase).unwrap();
        assert_eq!(source.name(), "DDInter");
        assert_eq!(source.tier(), SourceTier::CuratedDatabase);
    }

    #[test]
    fn rejects_an_empty_name() {
        let id = SourceId::parse("ddinter").unwrap();
        assert!(Source::new(id, "   ", SourceTier::CuratedDatabase).is_err());
    }
}
