use draxl_ast::{
    CommentNode, DocNode, Expr, Field, Item, MatchArm, NodeId, Param, Pattern, Rank, SlotName,
    Stmt, Type, Variant,
};

/// Slot owner used by slot refs and destinations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotOwner {
    /// The root file node.
    File,
    /// A node identified by stable id.
    Node(NodeId),
}

/// Public slot reference in the Draxl patch surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotRef {
    /// File root or node id that owns the destination slot.
    pub owner: SlotOwner,
    /// Stable profile-defined slot name.
    pub slot: SlotName,
}

/// Ranked destination for ordered slots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankedDest {
    /// Slot that owns the ranked child.
    pub slot: SlotRef,
    /// Rank to assign to the inserted or moved child.
    pub rank: Rank,
}

/// Destination grammar shared by `insert` and `move`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchDest {
    /// Ranked destination for ordered slots.
    Ranked(RankedDest),
    /// Single-child slot destination.
    Slot(SlotRef),
}

/// Replaceable, movable, or insertable semantic fragment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchNode {
    /// Top-level or nested item fragment.
    Item(Item),
    /// Struct field fragment.
    Field(Field),
    /// Enum variant fragment.
    Variant(Variant),
    /// Function parameter fragment.
    Param(Param),
    /// Statement fragment.
    Stmt(Stmt),
    /// Match arm fragment.
    MatchArm(MatchArm),
    /// Expression fragment.
    Expr(Expr),
    /// Type fragment.
    Type(Type),
    /// Pattern fragment.
    Pattern(Pattern),
    /// Doc comment fragment.
    Doc(DocNode),
    /// Line comment fragment.
    Comment(CommentNode),
}

/// Path address for scalar field or metadata updates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchPath {
    /// Stable id of the root node that owns the path.
    pub node_id: NodeId,
    /// Field path segments below the node.
    pub segments: Vec<String>,
}

/// Scalar value used by `set`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchValue {
    /// Bare identifier-like values such as names or enum variants.
    Ident(String),
    /// Quoted string values.
    Str(String),
    /// Numeric values.
    Int(i64),
    /// Boolean values.
    Bool(bool),
}

/// Structured patch operation over the Draxl semantic model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchOp {
    /// Inserts a new child into a ranked slot.
    Insert {
        /// Ranked destination that owns the inserted child.
        dest: RankedDest,
        /// Node to insert.
        node: PatchNode,
    },
    /// Fills or replaces a single-child slot.
    Put {
        /// Slot to fill.
        slot: SlotRef,
        /// Node to install into the slot.
        node: PatchNode,
    },
    /// Rewrites an existing node payload while preserving its outer shell.
    Replace {
        /// Stable id of the node to rewrite.
        target_id: NodeId,
        /// Replacement fragment for the node body.
        replacement: PatchNode,
    },
    /// Removes an existing node.
    Delete {
        /// Stable id of the node to delete.
        target_id: NodeId,
    },
    /// Relocates an existing node while preserving its identity.
    Move {
        /// Stable id of the node to move.
        target_id: NodeId,
        /// New destination for the node.
        dest: PatchDest,
    },
    /// Updates a scalar field or metadata path.
    Set {
        /// Path to update.
        path: PatchPath,
        /// New scalar value.
        value: PatchValue,
    },
    /// Clears an optional scalar field or metadata path.
    Clear {
        /// Path to clear.
        path: PatchPath,
    },
    /// Attaches a doc or comment node to a semantic target.
    Attach {
        /// Attachable node id.
        node_id: NodeId,
        /// Target node id.
        target_id: NodeId,
    },
    /// Detaches a doc or comment node.
    Detach {
        /// Attachable node id.
        node_id: NodeId,
    },
}
