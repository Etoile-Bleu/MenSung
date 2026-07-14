//! A single drug identified by its INN name, with optional cross-reference
//! data: an RxNorm `Rxcui` for systems that key on that identifier instead
//! of a name string, PubChem `ChemicalProperties` for its molecular
//! formula, weight, and IUPAC name, and zero or more WHO `AtcCode`s for
//! its therapeutic classification. All are optional/empty by default
//! because they come from separate lookups
//! (`mensung-builder::{rxnorm,pubchem,atc}`) that not every drug has a
//! confirmed match for; a drug with none of them is not an error.

use crate::{AtcCode, ChemicalProperties, DrugId, InnName, Rxcui};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drug {
    id: DrugId,
    inn_name: InnName,
    rxcui: Option<Rxcui>,
    chemical_properties: Option<ChemicalProperties>,
    atc_codes: Vec<AtcCode>,
}

impl Drug {
    pub fn new(id: DrugId, inn_name: InnName) -> Self {
        Self {
            id,
            inn_name,
            rxcui: None,
            chemical_properties: None,
            atc_codes: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_rxcui(mut self, rxcui: Rxcui) -> Self {
        self.rxcui = Some(rxcui);
        self
    }

    #[must_use]
    pub fn with_chemical_properties(mut self, properties: ChemicalProperties) -> Self {
        self.chemical_properties = Some(properties);
        self
    }

    #[must_use]
    pub fn with_atc_codes(mut self, atc_codes: Vec<AtcCode>) -> Self {
        self.atc_codes = atc_codes;
        self
    }

    pub fn id(&self) -> DrugId {
        self.id
    }

    pub fn inn_name(&self) -> &InnName {
        &self.inn_name
    }

    pub fn rxcui(&self) -> Option<&Rxcui> {
        self.rxcui.as_ref()
    }

    pub fn chemical_properties(&self) -> Option<&ChemicalProperties> {
        self.chemical_properties.as_ref()
    }

    pub fn atc_codes(&self) -> &[AtcCode] {
        &self.atc_codes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_its_id_and_name() {
        let drug = Drug::new(DrugId::new(1), InnName::parse("Warfarin").unwrap());
        assert_eq!(drug.id(), DrugId::new(1));
        assert_eq!(drug.inn_name().as_str(), "Warfarin");
    }

    #[test]
    fn has_no_rxcui_by_default() {
        let drug = Drug::new(DrugId::new(1), InnName::parse("Warfarin").unwrap());
        assert_eq!(drug.rxcui(), None);
    }

    #[test]
    fn with_rxcui_attaches_the_cross_reference() {
        let drug = Drug::new(DrugId::new(1), InnName::parse("Warfarin").unwrap())
            .with_rxcui(Rxcui::parse("11289").unwrap());
        assert_eq!(drug.rxcui().map(Rxcui::as_str), Some("11289"));
    }

    #[test]
    fn has_no_chemical_properties_by_default() {
        let drug = Drug::new(DrugId::new(1), InnName::parse("Warfarin").unwrap());
        assert_eq!(drug.chemical_properties(), None);
    }

    #[test]
    fn with_chemical_properties_attaches_them() {
        let properties =
            ChemicalProperties::new(crate::PubchemCid::new(54678486), "C19H16O4", "308.3", None)
                .unwrap();
        let drug = Drug::new(DrugId::new(1), InnName::parse("Warfarin").unwrap())
            .with_chemical_properties(properties);
        assert_eq!(
            drug.chemical_properties()
                .map(ChemicalProperties::molecular_formula),
            Some("C19H16O4")
        );
    }

    #[test]
    fn has_no_atc_codes_by_default() {
        let drug = Drug::new(DrugId::new(1), InnName::parse("Warfarin").unwrap());
        assert!(drug.atc_codes().is_empty());
    }

    #[test]
    fn with_atc_codes_attaches_all_of_them() {
        let codes = vec![
            AtcCode::new("B01AA", "Vitamin K antagonists").unwrap(),
            AtcCode::new("N02BA", "Salicylic acid and derivatives").unwrap(),
        ];
        let drug =
            Drug::new(DrugId::new(1), InnName::parse("Aspirin").unwrap()).with_atc_codes(codes);
        assert_eq!(drug.atc_codes().len(), 2);
        assert_eq!(drug.atc_codes()[0].code(), "B01AA");
    }
}
