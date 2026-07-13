//! How confident a claim's own source is in the claim, independent of the
//! source's general trust tier. A `SourceTier::Regulatory` source can still
//! publish a claim it labels low confidence (an early signal, not yet
//! confirmed); this is a per-claim enum rather than a continuous score so
//! it stays exact to compare and free of float-precision or NaN concerns,
//! matching how `Severity` and `EvidenceLevel` are already modeled.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Confidence {
    Low,
    Medium,
    High,
}

impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Confidence::Low => "Low",
            Confidence::Medium => "Medium",
            Confidence::High => "High",
        };
        f.write_str(label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orders_from_low_to_high() {
        assert!(Confidence::Low < Confidence::Medium);
        assert!(Confidence::Medium < Confidence::High);
    }
}
