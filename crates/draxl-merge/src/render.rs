use crate::model::ConflictSide;
use draxl_patch::{PatchDest, PatchOp, RankedDest, SlotOwner, SlotRef};

pub(crate) fn summarize_side(op_index: usize, op: &PatchOp) -> ConflictSide {
    ConflictSide {
        op_index,
        op_kind: op_kind(op),
        target: op_target(op),
        description: op_description(op),
    }
}

pub(crate) fn op_kind(op: &PatchOp) -> &'static str {
    match op {
        PatchOp::Insert { .. } => "insert",
        PatchOp::Put { .. } => "put",
        PatchOp::Replace { .. } => "replace",
        PatchOp::Delete { .. } => "delete",
        PatchOp::Move { .. } => "move",
        PatchOp::Set { .. } => "set",
        PatchOp::Clear { .. } => "clear",
        PatchOp::Attach { .. } => "attach",
        PatchOp::Detach { .. } => "detach",
    }
}

pub(crate) fn op_target(op: &PatchOp) -> String {
    match op {
        PatchOp::Insert { dest, .. } => ranked_dest_label(dest),
        PatchOp::Put { slot, .. } => slot_ref_label(slot),
        PatchOp::Replace { target_id, .. }
        | PatchOp::Delete { target_id }
        | PatchOp::Move { target_id, .. } => node_label(target_id),
        PatchOp::Set { path, .. } | PatchOp::Clear { path } => {
            path_label(&path.node_id, &path.segments)
        }
        PatchOp::Attach { node_id, target_id } => {
            format!("{} -> {}", node_label(node_id), node_label(target_id))
        }
        PatchOp::Detach { node_id } => node_label(node_id),
    }
}

pub(crate) fn op_description(op: &PatchOp) -> String {
    match op {
        PatchOp::Insert { dest, .. } => {
            format!("writes ranked destination {}", ranked_dest_label(dest))
        }
        PatchOp::Put { slot, .. } => format!("writes single-child slot {}", slot_ref_label(slot)),
        PatchOp::Replace { target_id, .. } => {
            format!("rewrites node shell {}", node_label(target_id))
        }
        PatchOp::Delete { target_id } => format!("deletes {}", node_label(target_id)),
        PatchOp::Move { target_id, dest } => match dest {
            PatchDest::Ranked(dest) => format!(
                "moves {} into ranked destination {}",
                node_label(target_id),
                ranked_dest_label(dest)
            ),
            PatchDest::Slot(slot) => format!(
                "moves {} into single-child slot {}",
                node_label(target_id),
                slot_ref_label(slot)
            ),
        },
        PatchOp::Set { path, .. } => format!(
            "writes scalar path {}",
            path_label(&path.node_id, &path.segments)
        ),
        PatchOp::Clear { path } => format!(
            "clears scalar path {}",
            path_label(&path.node_id, &path.segments)
        ),
        PatchOp::Attach { node_id, target_id } => format!(
            "attaches {} to {}",
            node_label(node_id),
            node_label(target_id)
        ),
        PatchOp::Detach { node_id } => format!("detaches {}", node_label(node_id)),
    }
}

pub(crate) fn slot_ref_label(slot: &SlotRef) -> String {
    format!("{}.{}", slot_owner_label(&slot.owner), slot.slot)
}

pub(crate) fn ranked_dest_label(dest: &RankedDest) -> String {
    format!("{}[{}]", slot_ref_label(&dest.slot), dest.rank)
}

pub(crate) fn path_label(node_id: &str, segments: &[String]) -> String {
    let mut label = node_label(node_id);
    for segment in segments {
        label.push('.');
        label.push_str(segment);
    }
    label
}

pub(crate) fn node_label(node_id: &str) -> String {
    format!("@{node_id}")
}

fn slot_owner_label(owner: &SlotOwner) -> String {
    match owner {
        SlotOwner::File => "file".to_owned(),
        SlotOwner::Node(id) => node_label(id),
    }
}
