//! Chemical reference data for a drug from PubChem
//! (`pubchem.ncbi.nlm.nih.gov`): its Compound ID (CID), molecular formula,
//! molecular weight, and IUPAC name. This is reference information, not a
//! clinical assertion, so it does not go through the `Claim`/`Source`
//! model the way an interaction or a label warning does: nothing here has
//! a severity, and there is nothing to resolve a disagreement about.
//!
//! Molecular weight is kept as PubChem's own decimal string ("180.16")
//! rather than parsed into a float. MenSung only ever displays this
//! value, never compares or computes with it, so a float round-trip would
//! only risk showing a slightly different string than PubChem's own
//! (`308.3` vs `308.29999999999995`) for no benefit; the same reasoning
//! that keeps a `Claim`'s rationale a `String` instead of parsing it.

use crate::DomainError;

/// A PubChem Compound ID, a plain positive integer in every real record
/// checked (`pubchem.ncbi.nlm.nih.gov/rest/pug/...`), not assumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PubchemCid(u32);

impl PubchemCid {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn value(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for PubchemCid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChemicalProperties {
    cid: PubchemCid,
    molecular_formula: String,
    molecular_weight: String,
    iupac_name: Option<String>,
}

impl ChemicalProperties {
    pub fn new(
        cid: PubchemCid,
        molecular_formula: impl Into<String>,
        molecular_weight: impl Into<String>,
        iupac_name: Option<String>,
    ) -> Result<Self, DomainError> {
        let molecular_formula = molecular_formula.into();
        if molecular_formula.trim().is_empty() {
            return Err(DomainError::EmptyMolecularFormula(cid.value()));
        }

        let molecular_weight = molecular_weight.into();
        if molecular_weight.trim().parse::<f64>().is_err() {
            return Err(DomainError::InvalidMolecularWeight {
                cid: cid.value(),
                raw: molecular_weight,
            });
        }

        Ok(Self {
            cid,
            molecular_formula,
            molecular_weight,
            iupac_name: iupac_name.filter(|name| !name.trim().is_empty()),
        })
    }

    pub fn cid(&self) -> PubchemCid {
        self.cid
    }

    pub fn molecular_formula(&self) -> &str {
        &self.molecular_formula
    }

    pub fn molecular_weight(&self) -> &str {
        &self.molecular_weight
    }

    pub fn iupac_name(&self) -> Option<&str> {
        self.iupac_name.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_from_real_pubchem_values() {
        let props = ChemicalProperties::new(
            PubchemCid::new(54678486),
            "C19H16O4",
            "308.3",
            Some("4-hydroxy-3-(3-oxo-1-phenylbutyl)chromen-2-one".to_string()),
        )
        .unwrap();
        assert_eq!(props.molecular_formula(), "C19H16O4");
        assert_eq!(props.molecular_weight(), "308.3");
        assert_eq!(
            props.iupac_name(),
            Some("4-hydroxy-3-(3-oxo-1-phenylbutyl)chromen-2-one")
        );
    }

    #[test]
    fn rejects_an_empty_formula() {
        let err = ChemicalProperties::new(PubchemCid::new(1), "  ", "1.0", None).unwrap_err();
        assert_eq!(err, DomainError::EmptyMolecularFormula(1));
    }

    #[test]
    fn rejects_a_non_numeric_weight() {
        let err =
            ChemicalProperties::new(PubchemCid::new(1), "H2O", "not-a-number", None).unwrap_err();
        assert_eq!(
            err,
            DomainError::InvalidMolecularWeight {
                cid: 1,
                raw: "not-a-number".to_string()
            }
        );
    }

    #[test]
    fn treats_a_blank_iupac_name_as_absent() {
        let props =
            ChemicalProperties::new(PubchemCid::new(1), "H2O", "18.02", Some("   ".to_string()))
                .unwrap();
        assert_eq!(props.iupac_name(), None);
    }
}
