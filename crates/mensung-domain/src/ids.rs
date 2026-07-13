//! Newtype identifiers for drugs and interactions, kept distinct at the type
//! level so a `DrugId` and an `InteractionId` can never be swapped by accident.

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
}
