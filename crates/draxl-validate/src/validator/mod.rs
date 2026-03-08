use crate::ValidationError;
use std::collections::BTreeMap;

mod collect;
mod rules;

#[derive(Default)]
pub(super) struct Validator {
    pub(super) seen_ids: BTreeMap<String, &'static str>,
    pub(super) errors: Vec<ValidationError>,
}

impl Validator {
    pub(super) fn push(&mut self, message: String) {
        self.errors.push(ValidationError { message });
    }
}
