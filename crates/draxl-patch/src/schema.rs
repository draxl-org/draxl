use draxl_ast::{Expr, File, Item, LowerLanguage, Stmt};
pub(crate) use draxl_rust::patch_schema::{
    AttachmentContainerKind, FragmentKind, NodeKind, PathSpec, SlotSpec, ValueKind,
};

pub(crate) fn slot_spec(language: LowerLanguage, owner: NodeKind, slot: &str) -> Option<SlotSpec> {
    match language {
        LowerLanguage::Rust => draxl_rust::patch_schema::slot_spec(owner, slot),
    }
}

pub(crate) fn ranked_slot_spec(
    language: LowerLanguage,
    owner: NodeKind,
    slot: &str,
) -> Option<SlotSpec> {
    match language {
        LowerLanguage::Rust => draxl_rust::patch_schema::ranked_slot_spec(owner, slot),
    }
}

pub(crate) fn single_slot_spec(
    language: LowerLanguage,
    owner: NodeKind,
    slot: &str,
) -> Option<SlotSpec> {
    match language {
        LowerLanguage::Rust => draxl_rust::patch_schema::single_slot_spec(owner, slot),
    }
}

pub(crate) fn removable_slot_spec(
    language: LowerLanguage,
    owner: NodeKind,
    slot: &str,
) -> Option<SlotSpec> {
    match language {
        LowerLanguage::Rust => draxl_rust::patch_schema::removable_slot_spec(owner, slot),
    }
}

pub(crate) fn path_spec(language: LowerLanguage, kind: NodeKind, path: &str) -> Option<PathSpec> {
    match language {
        LowerLanguage::Rust => draxl_rust::patch_schema::path_spec(kind, path),
    }
}

pub(crate) fn clearable_path_spec(
    language: LowerLanguage,
    kind: NodeKind,
    path: &str,
) -> Option<PathSpec> {
    match language {
        LowerLanguage::Rust => draxl_rust::patch_schema::clearable_path_spec(kind, path),
    }
}

pub(crate) fn replace_fragment_kind(language: LowerLanguage, kind: NodeKind) -> FragmentKind {
    match language {
        LowerLanguage::Rust => draxl_rust::patch_schema::replace_fragment_kind(kind),
    }
}

pub(crate) fn item_kind(language: LowerLanguage, item: &Item) -> NodeKind {
    match language {
        LowerLanguage::Rust => draxl_rust::patch_schema::item_kind(item),
    }
}

pub(crate) fn stmt_kind(language: LowerLanguage, stmt: &Stmt) -> NodeKind {
    match language {
        LowerLanguage::Rust => draxl_rust::patch_schema::stmt_kind(stmt),
    }
}

pub(crate) fn expr_kind(language: LowerLanguage, expr: &Expr) -> NodeKind {
    match language {
        LowerLanguage::Rust => draxl_rust::patch_schema::expr_kind(expr),
    }
}

pub(crate) fn node_kind_label(language: LowerLanguage, kind: NodeKind) -> &'static str {
    match language {
        LowerLanguage::Rust => draxl_rust::patch_schema::node_kind_label(kind),
    }
}

pub(crate) fn value_kind_label(language: LowerLanguage, value_kind: ValueKind) -> &'static str {
    match language {
        LowerLanguage::Rust => draxl_rust::patch_schema::value_kind_label(value_kind),
    }
}

pub(crate) fn attachment_container_kind_for_owner(
    language: LowerLanguage,
    kind: NodeKind,
) -> Option<AttachmentContainerKind> {
    match language {
        LowerLanguage::Rust => draxl_rust::patch_schema::attachment_container_kind_for_owner(kind),
    }
}

pub(crate) fn attachment_closure_allowed(
    language: LowerLanguage,
    owner_kind: NodeKind,
    slot: &str,
    closure_kind: AttachmentContainerKind,
) -> bool {
    match language {
        LowerLanguage::Rust => {
            draxl_rust::patch_schema::attachment_closure_allowed(owner_kind, slot, closure_kind)
        }
    }
}

pub(crate) fn is_attachable_kind(language: LowerLanguage, kind: NodeKind) -> bool {
    match language {
        LowerLanguage::Rust => draxl_rust::patch_schema::is_attachable_kind(kind),
    }
}

pub(crate) fn invalid_ranked_slot_message(
    language: LowerLanguage,
    owner_label: &str,
    slot: &str,
) -> String {
    match language {
        LowerLanguage::Rust => {
            draxl_rust::patch_schema::invalid_ranked_slot_message(owner_label, slot)
        }
    }
}

pub(crate) fn invalid_single_slot_message(
    language: LowerLanguage,
    owner_label: &str,
    slot: &str,
) -> String {
    match language {
        LowerLanguage::Rust => {
            draxl_rust::patch_schema::invalid_single_slot_message(owner_label, slot)
        }
    }
}

pub(crate) fn invalid_set_path_message(
    language: LowerLanguage,
    node_id: &str,
    path: &str,
    kind: NodeKind,
) -> String {
    match language {
        LowerLanguage::Rust => {
            draxl_rust::patch_schema::invalid_set_path_message(node_id, path, kind)
        }
    }
}

pub(crate) fn invalid_clear_path_message(
    language: LowerLanguage,
    node_id: &str,
    path: &str,
    kind: NodeKind,
) -> String {
    match language {
        LowerLanguage::Rust => {
            draxl_rust::patch_schema::invalid_clear_path_message(node_id, path, kind)
        }
    }
}

pub(crate) fn required_slot_error_message(
    language: LowerLanguage,
    action: &str,
    target_id: &str,
    slot: &str,
) -> String {
    match language {
        LowerLanguage::Rust => {
            draxl_rust::patch_schema::required_slot_error_message(action, target_id, slot)
        }
    }
}

pub(crate) fn unsupported_slot_error_message(
    language: LowerLanguage,
    action: &str,
    target_id: &str,
    slot: &str,
) -> String {
    match language {
        LowerLanguage::Rust => {
            draxl_rust::patch_schema::unsupported_slot_error_message(action, target_id, slot)
        }
    }
}

pub(crate) fn trivia_move_target_message(language: LowerLanguage) -> &'static str {
    match language {
        LowerLanguage::Rust => draxl_rust::patch_schema::trivia_move_target_message(),
    }
}

pub(crate) fn single_slot_attachment_closure_message(language: LowerLanguage) -> &'static str {
    match language {
        LowerLanguage::Rust => draxl_rust::patch_schema::single_slot_attachment_closure_message(),
    }
}

pub(crate) fn invalid_attachment_closure_destination_message(
    language: LowerLanguage,
    closure_kind: AttachmentContainerKind,
) -> &'static str {
    match language {
        LowerLanguage::Rust => {
            draxl_rust::patch_schema::invalid_attachment_closure_destination_message(closure_kind)
        }
    }
}

pub(crate) fn invalid_attachment_container_owner_message(
    language: LowerLanguage,
    owner_label: &str,
    closure_kind: AttachmentContainerKind,
) -> String {
    match language {
        LowerLanguage::Rust => {
            draxl_rust::patch_schema::invalid_attachment_container_owner_message(
                owner_label,
                closure_kind,
            )
        }
    }
}

pub(crate) fn attach_target_not_sibling_message(
    language: LowerLanguage,
    target_id: &str,
    node_id: &str,
) -> String {
    match language {
        LowerLanguage::Rust => {
            draxl_rust::patch_schema::attach_target_not_sibling_message(target_id, node_id)
        }
    }
}

pub(crate) fn detach_requires_following_sibling_message(
    language: LowerLanguage,
    node_id: &str,
) -> String {
    match language {
        LowerLanguage::Rust => {
            draxl_rust::patch_schema::detach_requires_following_sibling_message(node_id)
        }
    }
}

pub(crate) fn find_node_kind(
    language: LowerLanguage,
    file: &File,
    node_id: &str,
) -> Option<NodeKind> {
    match language {
        LowerLanguage::Rust => draxl_rust::patch_schema::find_node_kind(file, node_id),
    }
}
