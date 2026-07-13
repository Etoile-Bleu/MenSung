//! A single drug identified by its INN name.

use crate::{DrugId, InnName};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drug {
    id: DrugId,
    inn_name: InnName,
}

impl Drug {
    pub fn new(id: DrugId, inn_name: InnName) -> Self {
        Self { id, inn_name }
    }

    pub fn id(&self) -> DrugId {
        self.id
    }

    pub fn inn_name(&self) -> &InnName {
        &self.inn_name
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
}
