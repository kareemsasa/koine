//! Offline evidence contracts. No SQL, GUI, filesystem discovery or process execution.
pub mod canonical;
pub mod domain;
pub mod reconcile;
pub mod rules;
pub mod validation;
pub use domain::*;
