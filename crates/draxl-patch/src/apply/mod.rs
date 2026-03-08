mod delete;
mod insert;
mod replace;
mod support;

use crate::error::PatchError;
use crate::model::PatchOp;
use draxl_ast::File;

/// Applies a single patch operation to a file.
pub fn apply_op(file: &mut File, op: PatchOp) -> Result<(), PatchError> {
    match op {
        PatchOp::Insert {
            parent,
            slot,
            rank,
            node,
        } => insert::apply_insert(file, parent, &slot, &rank, node),
        PatchOp::Replace {
            target_id,
            replacement,
        } => replace::apply_replace(file, &target_id, replacement),
        PatchOp::Delete { target_id } => delete::apply_delete(file, &target_id),
    }
}

/// Applies a sequence of patch operations in order.
pub fn apply_ops(
    file: &mut File,
    ops: impl IntoIterator<Item = PatchOp>,
) -> Result<(), PatchError> {
    for op in ops {
        apply_op(file, op)?;
    }
    Ok(())
}
