//! Newtype identifiers for drugs, interactions, and single-drug facts, kept
//! distinct at the type level so one kind of id can never be swapped for
//! another by accident.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DrugId(u32);

impl DrugId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn value(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InteractionId(u32);

impl InteractionId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn value(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DrugFactId(u32);

impl DrugFactId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn value(self) -> u32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drug_id_roundtrips_its_value() {
        assert_eq!(DrugId::new(42).value(), 42);
    }

    #[test]
    fn distinct_values_are_not_equal() {
        assert_ne!(DrugId::new(1), DrugId::new(2));
    }

    #[test]
    fn ids_order_by_their_underlying_value() {
        assert!(DrugId::new(1) < DrugId::new(2));
    }

    #[test]
    fn drug_fact_id_roundtrips_its_value() {
        assert_eq!(DrugFactId::new(7).value(), 7);
    }
}
