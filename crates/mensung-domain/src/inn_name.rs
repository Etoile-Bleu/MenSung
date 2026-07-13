//! The INN (International Nonproprietary Name) drug name: the only drug
//! naming form MenSung stores or displays, per MEDICAL_DATA_POLICY.md. Brand
//! names never reach this type; validation rejects anything that is not a
//! plausible INN (letters, spaces, and hyphens only, non-empty, bounded
//! length).

use crate::DomainError;

const MAX_LEN: usize = 128;

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

        if let Some(invalid_char) = trimmed
            .chars()
            .find(|c| !(c.is_ascii_alphabetic() || *c == ' ' || *c == '-'))
        {
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
    fn rejects_digits() {
        assert!(matches!(
            InnName::parse("Drug123").unwrap_err(),
            DomainError::InvalidInnNameCharacter { .. }
        ));
    }

    #[test]
    fn rejects_punctuation() {
        assert!(matches!(
            InnName::parse("Aspirin!").unwrap_err(),
            DomainError::InvalidInnNameCharacter { .. }
        ));
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
