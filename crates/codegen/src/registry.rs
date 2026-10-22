//! The generators, in the order they run. Each checkpoint that adds a
//! generated interface adds its generator here (CR10): the IDL's at M1-CP3,
//! the property table's and the static atoms' at M1-CP4.

use crate::Generator;

/// Every generator.
#[must_use]
pub fn generators() -> Vec<Box<dyn Generator>> {
    Vec::new()
}
