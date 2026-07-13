//! A fuzzy-match candidate offered when a typed drug name has no exact
//! match: the candidate drug and a similarity score in `[0.0, 1.0]`. A
//! `Candidate` is never applied automatically, only ever presented for
//! confirmation, per the no-silent-correction rule in
//! MEDICAL_DATA_POLICY.md.

use mensung_db::DrugRecord;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candidate<'a> {
    drug: DrugRecord<'a>,
    similarity: f32,
}

impl<'a> Candidate<'a> {
    pub(crate) fn new(drug: DrugRecord<'a>, similarity: f32) -> Self {
        Self { drug, similarity }
    }

    pub fn drug(&self) -> DrugRecord<'a> {
        self.drug
    }

    pub fn similarity(&self) -> f32 {
        self.similarity
    }
}
