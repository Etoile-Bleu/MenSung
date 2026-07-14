//! An RxNorm Concept Unique Identifier (RxCUI): the stable numeric id
//! RxNorm assigns to a drug concept, letting MenSung's INN-named drugs be
//! cross-referenced against RxNorm, DailyMed, and other systems that key
//! on RxCUI rather than a name string. Verified against real RxNorm API
//! responses (`rxnav.nlm.nih.gov/REST/rxcui.json`), not assumed: every
//! RxCUI observed there is a plain ASCII numeral string ("11289", "1191",
//! "1154539"), so that is the only shape accepted here.

use crate::DomainError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rxcui(String);

impl Rxcui {
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() || !trimmed.bytes().all(|b| b.is_ascii_digit()) {
            return Err(DomainError::InvalidRxcui(raw.to_string()));
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Rxcui {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_real_rxcui() {
        assert_eq!(Rxcui::parse("11289").unwrap().as_str(), "11289");
        assert_eq!(Rxcui::parse("1154539").unwrap().as_str(), "1154539");
    }

    #[test]
    fn trims_surrounding_whitespace() {
        assert_eq!(Rxcui::parse("  1191  ").unwrap().as_str(), "1191");
    }

    #[test]
    fn rejects_non_numeric_input() {
        assert!(Rxcui::parse("abc123").is_err());
        assert!(Rxcui::parse("12.5").is_err());
        assert!(Rxcui::parse("-11289").is_err());
    }

    #[test]
    fn rejects_empty_input() {
        assert!(Rxcui::parse("").is_err());
        assert!(Rxcui::parse("   ").is_err());
    }
}
