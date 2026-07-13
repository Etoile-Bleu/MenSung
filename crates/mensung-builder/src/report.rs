//! validation-report.json: the machine-readable summary every .men build
//! produces, per MEDICAL_DATA_POLICY.md's validation pipeline. A report
//! with `errors > 0` means the build must not ship.

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ValidationReport {
    pub errors: usize,
    pub warnings: usize,
    pub interactions: usize,
}

impl ValidationReport {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self)
            .expect("ValidationReport holds only plain integers, which always serialize")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_to_the_documented_shape() {
        let report = ValidationReport {
            errors: 0,
            warnings: 12,
            interactions: 150_000,
        };
        let json = report.to_json();
        assert!(json.contains("\"errors\": 0"));
        assert!(json.contains("\"warnings\": 12"));
        assert!(json.contains("\"interactions\": 150000"));
    }
}
