mod attach;
mod delete;
mod insert;
mod r#move;
mod put;
mod replace;
mod set_clear;
mod support;

use crate::error::PatchError;
use crate::model::PatchOp;
use draxl_ast::{File, LowerLanguage};

/// Applies a single patch operation to a file.
pub fn apply_op(file: &mut File, op: PatchOp) -> Result<(), PatchError> {
    apply_op_for_language(LowerLanguage::Rust, file, op)
}

/// Applies a single patch operation to a file using the selected lower language.
pub fn apply_op_for_language(
    language: LowerLanguage,
    file: &mut File,
    op: PatchOp,
) -> Result<(), PatchError> {
    match op {
        PatchOp::Insert { dest, node } => insert::apply_insert(language, file, dest, node),
        PatchOp::Put { slot, node } => put::apply_put(language, file, slot, node),
        PatchOp::Replace {
            target_id,
            replacement,
        } => replace::apply_replace(file, &target_id, replacement),
        PatchOp::Delete { target_id } => delete::apply_delete(language, file, &target_id),
        PatchOp::Move { target_id, dest } => r#move::apply_move(language, file, &target_id, dest),
        PatchOp::Set { path, value } => set_clear::apply_set(language, file, path, value),
        PatchOp::Clear { path } => set_clear::apply_clear(language, file, path),
        PatchOp::Attach { node_id, target_id } => {
            attach::apply_attach(language, file, &node_id, &target_id)
        }
        PatchOp::Detach { node_id } => attach::apply_detach(language, file, &node_id),
    }
}

/// Applies a sequence of patch operations in order.
pub fn apply_ops(
    file: &mut File,
    ops: impl IntoIterator<Item = PatchOp>,
) -> Result<(), PatchError> {
    apply_ops_for_language(LowerLanguage::Rust, file, ops)
}

/// Applies a sequence of patch operations in order using the selected lower language.
pub fn apply_ops_for_language(
    language: LowerLanguage,
    file: &mut File,
    ops: impl IntoIterator<Item = PatchOp>,
) -> Result<(), PatchError> {
    for op in ops {
        apply_op_for_language(language, file, op)?;
    }
    Ok(())
}
