//! The INN (International Nonproprietary Name) drug name: the only drug
//! naming form MenSung stores or displays, per MEDICAL_DATA_POLICY.md. Brand
//! names never reach this type. The allowed character set is ASCII letters,
//! digits, spaces, and `'()-,./`, chosen by checking DDInter's real 1939
//! drug names rather than guessing: real pharmaceutical nomenclature needs
//! digits (`Interferon beta-1a`), parenthesized route/form qualifiers
//! (`Dexamethasone (topical)`, clinically distinct from the systemic form
//! and never safe to strip), and commas in reordered generic names
//! (`Thyroid, porcine`). A stricter, letters-only rule silently rejected
//! 252 of those 1939 real drugs, which is a zero-false-negative violation
//! by omission, not a formatting nicety.

use crate::DomainError;

const MAX_LEN: usize = 128;
const ALLOWED_PUNCTUATION: &str = "'(),-./";

fn is_allowed_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == ' ' || ALLOWED_PUNCTUATION.contains(c)
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InnName(String);

impl InnName {
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        let trimmed = raw.trim();

        if trimmed.is_empty() {
            return Err(DomainError::EmptyInnName);
        }

        let char_count = trimmed.chars().count();
        if char_count > MAX_LEN {
            return Err(DomainError::InnNameTooLong {
                name: trimmed.to_string(),
                max: MAX_LEN,
                actual: char_count,
            });
        }

        if let Some(invalid_char) = trimmed.chars().find(|c| !is_allowed_char(*c)) {
            return Err(DomainError::InvalidInnNameCharacter {
                name: trimmed.to_string(),
                invalid_char,
            });
        }

        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for InnName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_simple_name() {
        assert_eq!(InnName::parse("Warfarin").unwrap().as_str(), "Warfarin");
    }

    #[test]
    fn accepts_spaces_and_hyphens() {
        assert!(InnName::parse("Acetylsalicylic acid").is_ok());
        assert!(InnName::parse("Co-trimoxazole").is_ok());
    }

    #[test]
    fn accepts_real_ddinter_names_that_a_letters_only_rule_would_have_rejected() {
        assert!(InnName::parse("Interferon beta-1a").is_ok());
        assert!(InnName::parse("Dexamethasone (topical)").is_ok());
        assert!(InnName::parse("Thyroid, porcine").is_ok());
        assert!(InnName::parse("Polyethylene glycol (3350 with electrolytes)").is_ok());
        assert!(InnName::parse("Sodium phosphate, monobasic (p32)").is_ok());
    }

    #[test]
    fn trims_surrounding_whitespace() {
        assert_eq!(InnName::parse("  Aspirin  ").unwrap().as_str(), "Aspirin");
    }

    #[test]
    fn rejects_empty_input() {
        assert_eq!(
            InnName::parse("   ").unwrap_err(),
            DomainError::EmptyInnName
        );
    }

    #[test]
    fn rejects_characters_outside_the_verified_ddinter_set() {
        for bad in ["Aspirin!", "Aspirin@", "Aspirin#", "Aspirin;", "Aspirin\""] {
            assert!(
                matches!(
                    InnName::parse(bad).unwrap_err(),
                    DomainError::InvalidInnNameCharacter { .. }
                ),
                "expected {bad:?} to be rejected"
            );
        }
    }

    #[test]
    fn rejects_names_over_the_length_limit() {
        let too_long = "a".repeat(MAX_LEN + 1);
        assert!(matches!(
            InnName::parse(&too_long).unwrap_err(),
            DomainError::InnNameTooLong { .. }
        ));
    }

    #[test]
    fn accepts_a_name_at_exactly_the_length_limit() {
        let exact = "a".repeat(MAX_LEN);
        assert!(InnName::parse(&exact).is_ok());
    }
}
