#![forbid(unsafe_code)]
//! Merge analysis for Draxl patch streams.
//!
//! This crate compares two patch sets against a shared base tree and reports
//! hard conflicts. The initial hard-conflict rule is replay-based:
//!
//! - if either replay order fails, that is a hard conflict
//! - if both replay orders succeed but produce different final ASTs, that is a
//!   hard conflict

use draxl_ast::File;
use draxl_patch::{apply_op, PatchOp};
use draxl_printer::canonicalize_file;
use draxl_validate::validate_file;

/// Outcome of checking two patch streams for hard conflicts.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HardConflictReport {
    /// Hard conflicts found while replaying the two patch streams.
    pub conflicts: Vec<HardConflict>,
}

impl HardConflictReport {
    /// Returns true when the two patch streams do not have any hard conflicts.
    pub fn is_clean(&self) -> bool {
        self.conflicts.is_empty()
    }

    /// Returns true when at least one hard conflict was found.
    pub fn has_conflicts(&self) -> bool {
        !self.is_clean()
    }
}

/// Hard conflict produced while comparing two patch streams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HardConflict {
    /// One replay order failed to apply or validate.
    ReplayFailed {
        /// Replay order that failed.
        order: ReplayOrder,
        /// Stage that failed inside the replay order.
        stage: ReplayStage,
        /// Human-readable description of the failure.
        message: String,
    },
    /// Both replay orders succeeded, but they did not converge.
    NonConvergentResults,
}

/// Replay order used when checking two patch streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayOrder {
    /// Apply left, then right.
    LeftThenRight,
    /// Apply right, then left.
    RightThenLeft,
}

/// Stage inside a replay order that can fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayStage {
    /// Failed while applying the indexed op from the left patch stream.
    LeftOp(usize),
    /// Failed while applying the indexed op from the right patch stream.
    RightOp(usize),
    /// Failed after patch application during structural validation.
    Validation,
}

/// Checks whether two patch streams have hard conflicts against the same base.
pub fn check_hard_conflicts(
    base: &File,
    left: &[PatchOp],
    right: &[PatchOp],
) -> HardConflictReport {
    let left_then_right = replay(base, ReplayOrder::LeftThenRight, left, right);
    let right_then_left = replay(base, ReplayOrder::RightThenLeft, right, left);

    let mut conflicts = Vec::new();

    match &left_then_right {
        Ok(_) => {}
        Err(conflict) => conflicts.push(conflict.clone()),
    }

    match &right_then_left {
        Ok(_) => {}
        Err(conflict) => conflicts.push(conflict.clone()),
    }

    if let (Ok(left_then_right), Ok(right_then_left)) = (&left_then_right, &right_then_left) {
        if canonicalize_file(left_then_right).without_spans()
            != canonicalize_file(right_then_left).without_spans()
        {
            conflicts.push(HardConflict::NonConvergentResults);
        }
    }

    HardConflictReport { conflicts }
}

fn replay(
    base: &File,
    order: ReplayOrder,
    first: &[PatchOp],
    second: &[PatchOp],
) -> Result<File, HardConflict> {
    let mut file = base.clone();
    apply_sequence(&mut file, order, first, first_stage)?;
    apply_sequence(&mut file, order, second, second_stage)?;
    validate_file(&file).map_err(|errors| HardConflict::ReplayFailed {
        order,
        stage: ReplayStage::Validation,
        message: format_validation_errors(&errors),
    })?;
    Ok(file)
}

fn apply_sequence(
    file: &mut File,
    order: ReplayOrder,
    ops: &[PatchOp],
    stage_for_index: impl Fn(usize) -> ReplayStage,
) -> Result<(), HardConflict> {
    for (index, op) in ops.iter().cloned().enumerate() {
        apply_op(file, op).map_err(|error| HardConflict::ReplayFailed {
            order,
            stage: stage_for_index(index),
            message: error.to_string(),
        })?;
    }
    Ok(())
}

fn first_stage(index: usize) -> ReplayStage {
    ReplayStage::LeftOp(index)
}

fn second_stage(index: usize) -> ReplayStage {
    ReplayStage::RightOp(index)
}

fn format_validation_errors(errors: &[draxl_validate::ValidationError]) -> String {
    let mut message = String::from("validation failed:");
    for error in errors {
        message.push_str("\n- ");
        message.push_str(&error.message);
    }
    message
}
