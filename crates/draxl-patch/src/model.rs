use draxl_ast::{Field, Item, MatchArm, NodeId, Param, Rank, SlotName, Stmt, Variant};

/// Parent selector for insertion operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchParent {
    /// Inserts into the root file slot.
    File,
    /// Inserts into a slot owned by a node id.
    Node {
        /// Stable id of the parent node that owns the destination slot.
        id: NodeId,
    },
}

/// Replaceable or insertable ranked slot child.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchNode {
    /// Item child for `file_items` and `items` slots.
    Item(Item),
    /// Field child for the `fields` slot.
    Field(Field),
    /// Variant child for the `variants` slot.
    Variant(Variant),
    /// Parameter child for the `params` slot.
    Param(Param),
    /// Statement child for the `body` slot.
    Stmt(Stmt),
    /// Match arm child for the `arms` slot.
    MatchArm(MatchArm),
}

/// Structured patch operation over Draxl slot children.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchOp {
    /// Inserts a new child into a ranked slot.
    Insert {
        /// File root or node id that owns the destination slot.
        parent: PatchParent,
        /// Slot name to insert into.
        slot: SlotName,
        /// Rank to assign to the inserted semantic node.
        rank: Rank,
        /// Node to insert.
        node: PatchNode,
    },
    /// Replaces an existing ranked slot child by id.
    Replace {
        /// Stable id of the slot child to replace.
        target_id: NodeId,
        /// Replacement node.
        replacement: PatchNode,
    },
    /// Deletes an existing ranked slot child by id.
    Delete {
        /// Stable id of the slot child to delete.
        target_id: NodeId,
    },
}
