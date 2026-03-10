use draxl_ast::{NodeId, Span};

/// Unresolved textual patch op parsed from the canonical patch surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfacePatchOp {
    Insert {
        dest: SurfaceRankedDest,
        fragment: SurfaceFragment,
        span: Span,
    },
    Put {
        slot: SurfaceSlotRef,
        fragment: SurfaceFragment,
        span: Span,
    },
    Replace {
        target: SurfaceNodeRef,
        fragment: SurfaceFragment,
        span: Span,
    },
    Delete {
        target: SurfaceNodeRef,
        span: Span,
    },
    Move {
        target: SurfaceNodeRef,
        dest: SurfaceDest,
        span: Span,
    },
    Set {
        path: SurfacePath,
        value: SurfaceValue,
        span: Span,
    },
    Clear {
        path: SurfacePath,
        span: Span,
    },
    Attach {
        node: SurfaceNodeRef,
        target: SurfaceNodeRef,
        span: Span,
    },
    Detach {
        node: SurfaceNodeRef,
        span: Span,
    },
}

impl SurfacePatchOp {
    /// Returns the source span for the full patch op entry.
    pub fn span(&self) -> Span {
        match self {
            Self::Insert { span, .. }
            | Self::Put { span, .. }
            | Self::Replace { span, .. }
            | Self::Delete { span, .. }
            | Self::Move { span, .. }
            | Self::Set { span, .. }
            | Self::Clear { span, .. }
            | Self::Attach { span, .. }
            | Self::Detach { span, .. } => *span,
        }
    }
}

/// Parsed `@id` node reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceNodeRef {
    pub id: NodeId,
    pub span: Span,
}

/// Parsed owner for a slot ref.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceSlotOwner {
    File { span: Span },
    Node(SurfaceNodeRef),
}

/// Parsed slot reference before schema resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceSlotRef {
    pub owner: SurfaceSlotOwner,
    pub slot: String,
    pub slot_span: Span,
    pub span: Span,
}

/// Parsed ranked destination before schema resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceRankedDest {
    pub slot: SurfaceSlotRef,
    pub rank: String,
    pub rank_span: Span,
    pub span: Span,
}

/// Parsed move destination before schema resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceDest {
    Ranked(SurfaceRankedDest),
    Slot(SurfaceSlotRef),
}

/// Parsed scalar path before schema resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfacePath {
    pub node: SurfaceNodeRef,
    pub segments: Vec<SurfacePathSegment>,
    pub span: Span,
}

/// Parsed scalar path segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfacePathSegment {
    pub name: String,
    pub span: Span,
}

/// Parsed scalar value before type resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceValue {
    pub kind: SurfaceValueKind,
    pub span: Span,
}

/// Surface scalar value variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceValueKind {
    Ident(String),
    Str(String),
    Int(i64),
    Bool(bool),
}

/// Fragment source carried verbatim until schema resolution picks a parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceFragment {
    pub source: String,
    pub span: Span,
}
