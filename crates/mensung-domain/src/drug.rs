//! A single drug identified by its INN name, with an optional RxNorm
//! cross-reference (`Rxcui`) for systems that key on that identifier
//! instead of a name string. The RxCUI is optional because it comes from
//! a separate lookup (`mensung-builder::rxnorm`) that not every drug has
//! a confirmed match for; a drug with no RxCUI is not an error.

use crate::{DrugId, InnName, Rxcui};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drug {
    id: DrugId,
    inn_name: InnName,
    rxcui: Option<Rxcui>,
}

impl Drug {
    pub fn new(id: DrugId, inn_name: InnName) -> Self {
        Self {
            id,
            inn_name,
            rxcui: None,
        }
    }

    #[must_use]
    pub fn with_rxcui(mut self, rxcui: Rxcui) -> Self {
        self.rxcui = Some(rxcui);
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
}
