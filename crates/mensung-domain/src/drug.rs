//! A single drug identified by its INN name, with optional cross-reference
//! data: an RxNorm `Rxcui` for systems that key on that identifier instead
//! of a name string, and PubChem `ChemicalProperties` for its molecular
//! formula, weight, and IUPAC name. Both are optional because they come
//! from separate lookups (`mensung-builder::rxnorm`,
//! `mensung-builder::pubchem`) that not every drug has a confirmed match
//! for; a drug with neither is not an error.

use crate::{ChemicalProperties, DrugId, InnName, Rxcui};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drug {
    id: DrugId,
    inn_name: InnName,
    rxcui: Option<Rxcui>,
    chemical_properties: Option<ChemicalProperties>,
}

impl Drug {
    pub fn new(id: DrugId, inn_name: InnName) -> Self {
        Self {
            id,
            inn_name,
            rxcui: None,
            chemical_properties: None,
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
}
