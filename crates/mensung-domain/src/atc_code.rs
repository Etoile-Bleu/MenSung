//! A WHO ATC (Anatomical Therapeutic Chemical) classification code, e.g.
//! `B01AA` for "Vitamin K antagonists". A single drug can carry more than
//! one ATC code (aspirin is classified both as a platelet aggregation
//! inhibitor, B01AC, and as a salicylate analgesic, N02BA, depending on
//! use), so this is not a single value the way `Rxcui` is; see
//! `Drug::atc_codes`.
//!
//! Verified against real classification data (WHO's own ATC/DDD Index has
//! no bulk API; this project reaches ATC codes through NLM's RxClass API,
//! which cross-references RxNorm concepts to ATC, see
//! `mensung-builder::atc`): every real code observed is exactly one
//! uppercase letter, two digits, then two uppercase letters (`B01AA`,
//! `A01AD`, `N02BA`), the WHO-documented five-character chemical
//! substance level of the ATC hierarchy.

use crate::DomainError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AtcCode {
    code: String,
    class_name: String,
}

fn is_valid_code_shape(code: &str) -> bool {
    let bytes = code.as_bytes();
    bytes.len() == 5
        && bytes[0].is_ascii_uppercase()
        && bytes[1].is_ascii_digit()
        && bytes[2].is_ascii_digit()
        && bytes[3].is_ascii_uppercase()
        && bytes[4].is_ascii_uppercase()
}

impl AtcCode {
    pub fn new(
        code: impl Into<String>,
        class_name: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let code = code.into();
        if !is_valid_code_shape(&code) {
            return Err(DomainError::InvalidAtcCode(code));
        }

        let class_name = class_name.into();
        if class_name.trim().is_empty() {
            return Err(DomainError::EmptyAtcClassName(code));
        }

        Ok(Self { code, class_name })
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn class_name(&self) -> &str {
        &self.class_name
    }
}

impl std::fmt::Display for AtcCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.code, self.class_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_real_atc_code() {
        let atc = AtcCode::new("B01AA", "Vitamin K antagonists").unwrap();
        assert_eq!(atc.code(), "B01AA");
        assert_eq!(atc.class_name(), "Vitamin K antagonists");
    }

    #[test]
    fn displays_code_and_class_name_together() {
        let atc = AtcCode::new("B01AA", "Vitamin K antagonists").unwrap();
        assert_eq!(atc.to_string(), "B01AA (Vitamin K antagonists)");
    }

    #[test]
    fn rejects_the_wrong_shape() {
        assert!(AtcCode::new("B01A", "too short").is_err());
        assert!(AtcCode::new("b01AA", "lowercase first letter").is_err());
        assert!(AtcCode::new("B01aa", "lowercase suffix").is_err());
        assert!(AtcCode::new("BB1AA", "digit position is a letter").is_err());
    }

    #[test]
    fn rejects_an_empty_class_name() {
        assert!(AtcCode::new("B01AA", "  ").is_err());
    }
}
