use super::error::{map_fragment_parse_error, patch_text_error, PatchTextError};
use super::surface::{
    SurfaceDest, SurfaceFragment, SurfaceNodeRef, SurfacePatchOp, SurfacePath, SurfaceRankedDest,
    SurfaceSlotOwner, SurfaceSlotRef, SurfaceValue, SurfaceValueKind,
};
use crate::model::{PatchDest, PatchNode, PatchOp, PatchPath, PatchValue, RankedDest, SlotOwner};
use crate::schema::{
    clearable_path_spec, find_node_kind, invalid_clear_path_message, invalid_ranked_slot_message,
    invalid_set_path_message, invalid_single_slot_message, node_kind_label, path_spec,
    ranked_slot_spec, replace_fragment_kind, single_slot_spec, value_kind_label, FragmentKind,
    NodeKind, ValueKind,
};
use draxl_ast::{File, Span};

#[derive(Debug, Clone)]
pub(super) struct ResolvedPatchOp {
    pub op: PatchOp,
    pub span: Span,
}

pub(super) fn resolve_patch_ops(
    file: &File,
    source: &str,
    surface_ops: &[SurfacePatchOp],
) -> Result<Vec<ResolvedPatchOp>, PatchTextError> {
    let mut working = file.clone();
    let mut resolved = Vec::with_capacity(surface_ops.len());
    for surface_op in surface_ops {
        let op = resolve_op(&working, source, surface_op)?;
        crate::apply_op(&mut working, op.op.clone())
            .map_err(|err| patch_text_error(source, surface_op.span(), &err.message))?;
        resolved.push(op);
    }
    Ok(resolved)
}

pub(super) fn resolve_op(
    file: &File,
    source: &str,
    surface_op: &SurfacePatchOp,
) -> Result<ResolvedPatchOp, PatchTextError> {
    let span = surface_op.span();
    let op = match surface_op {
        SurfacePatchOp::Insert { dest, fragment, .. } => {
            let resolved_dest = resolve_ranked_dest(file, source, dest)?;
            let owner_kind = dest_owner_kind(file, source, &dest.slot.owner)?;
            let fragment_kind = ranked_slot_spec(owner_kind, &dest.slot.slot)
                .map(|spec| spec.fragment_kind)
                .expect("ranked destination fragment kind must already be validated before use");
            PatchOp::Insert {
                dest: resolved_dest,
                node: parse_fragment(source, fragment, fragment_kind)?,
            }
        }
        SurfacePatchOp::Put { slot, fragment, .. } => {
            let owner_kind = dest_owner_kind(file, source, &slot.owner)?;
            let fragment_kind = single_slot_spec(owner_kind, &slot.slot)
                .map(|spec| spec.fragment_kind)
                .ok_or_else(|| {
                    patch_text_error(
                        source,
                        slot.slot_span,
                        &invalid_single_slot_message(&owner_label(&slot.owner), &slot.slot),
                    )
                })?;
            PatchOp::Put {
                slot: resolve_slot_ref(file, source, slot)?,
                node: parse_fragment(source, fragment, fragment_kind)?,
            }
        }
        SurfacePatchOp::Replace {
            target, fragment, ..
        } => {
            let target_kind = resolve_node_kind(file, source, target)?;
            PatchOp::Replace {
                target_id: target.id.clone(),
                replacement: parse_fragment(source, fragment, replace_fragment_kind(target_kind))?,
            }
        }
        SurfacePatchOp::Delete { target, .. } => PatchOp::Delete {
            target_id: ensure_node_exists(file, source, target)?.id,
        },
        SurfacePatchOp::Move { target, dest, .. } => PatchOp::Move {
            target_id: ensure_node_exists(file, source, target)?.id,
            dest: resolve_move_dest(file, source, dest)?,
        },
        SurfacePatchOp::Set { path, value, .. } => PatchOp::Set {
            path: resolve_path(file, source, path)?,
            value: resolve_value(file, source, path, value)?,
        },
        SurfacePatchOp::Clear { path, .. } => {
            let node_kind = resolve_node_kind(file, source, &path.node)?;
            ensure_single_segment_path(source, path)?;
            let segment = &path.segments[0];
            if clearable_path_spec(node_kind, &segment.name).is_none() {
                return Err(patch_text_error(
                    source,
                    segment.span,
                    &invalid_clear_path_message(&path.node.id, &segment.name, node_kind),
                ));
            }
            PatchOp::Clear {
                path: resolve_path(file, source, path)?,
            }
        }
        SurfacePatchOp::Attach { node, target, .. } => PatchOp::Attach {
            node_id: ensure_node_exists(file, source, node)?.id,
            target_id: ensure_node_exists(file, source, target)?.id,
        },
        SurfacePatchOp::Detach { node, .. } => PatchOp::Detach {
            node_id: ensure_node_exists(file, source, node)?.id,
        },
    };

    Ok(ResolvedPatchOp { op, span })
}

fn resolve_move_dest(
    file: &File,
    source: &str,
    dest: &SurfaceDest,
) -> Result<PatchDest, PatchTextError> {
    match dest {
        SurfaceDest::Ranked(dest) => {
            Ok(PatchDest::Ranked(resolve_ranked_dest(file, source, dest)?))
        }
        SurfaceDest::Slot(slot) => {
            let owner_kind = dest_owner_kind(file, source, &slot.owner)?;
            if single_slot_spec(owner_kind, &slot.slot).is_none() {
                return Err(patch_text_error(
                    source,
                    slot.slot_span,
                    &format!(
                        "slot `{}.{}` is not a single-child move destination on {}",
                        owner_label(&slot.owner),
                        slot.slot,
                        node_kind_label(owner_kind)
                    ),
                ));
            }
            Ok(PatchDest::Slot(resolve_slot_ref(file, source, slot)?))
        }
    }
}

fn resolve_ranked_dest(
    file: &File,
    source: &str,
    dest: &SurfaceRankedDest,
) -> Result<RankedDest, PatchTextError> {
    let owner_kind = dest_owner_kind(file, source, &dest.slot.owner)?;
    if ranked_slot_spec(owner_kind, &dest.slot.slot).is_none() {
        return Err(patch_text_error(
            source,
            dest.slot.slot_span,
            &invalid_ranked_slot_message(&owner_label(&dest.slot.owner), &dest.slot.slot),
        ));
    }
    Ok(RankedDest {
        slot: resolve_slot_ref(file, source, &dest.slot)?,
        rank: dest.rank.clone(),
    })
}

fn resolve_slot_ref(
    file: &File,
    source: &str,
    slot: &SurfaceSlotRef,
) -> Result<crate::model::SlotRef, PatchTextError> {
    Ok(crate::model::SlotRef {
        owner: match &slot.owner {
            SurfaceSlotOwner::File { .. } => SlotOwner::File,
            SurfaceSlotOwner::Node(node) => {
                ensure_node_exists(file, source, node)?;
                SlotOwner::Node(node.id.clone())
            }
        },
        slot: slot.slot.clone(),
    })
}

fn resolve_path(
    file: &File,
    source: &str,
    path: &SurfacePath,
) -> Result<PatchPath, PatchTextError> {
    ensure_single_segment_path(source, path)?;
    ensure_node_exists(file, source, &path.node)?;
    Ok(PatchPath {
        node_id: path.node.id.clone(),
        segments: path
            .segments
            .iter()
            .map(|segment| segment.name.clone())
            .collect(),
    })
}

fn resolve_value(
    file: &File,
    source: &str,
    path: &SurfacePath,
    value: &SurfaceValue,
) -> Result<PatchValue, PatchTextError> {
    let node_kind = resolve_node_kind(file, source, &path.node)?;
    ensure_single_segment_path(source, path)?;
    let segment = &path.segments[0];
    let value_kind = path_spec(node_kind, &segment.name)
        .map(|spec| spec.value_kind)
        .ok_or_else(|| {
            patch_text_error(
                source,
                segment.span,
                &invalid_set_path_message(&path.node.id, &segment.name, node_kind),
            )
        })?;
    match (&value.kind, value_kind) {
        (SurfaceValueKind::Ident(inner), ValueKind::Ident) => Ok(PatchValue::Ident(inner.clone())),
        (SurfaceValueKind::Str(inner), ValueKind::Str) => Ok(PatchValue::Str(inner.clone())),
        (SurfaceValueKind::Bool(inner), ValueKind::Bool) => Ok(PatchValue::Bool(*inner)),
        (_, expected) => Err(patch_text_error(
            source,
            value.span,
            &format!(
                "path `@{}.{}` expects {}",
                path.node.id,
                segment.name,
                value_kind_label(expected)
            ),
        )),
    }
}

fn parse_fragment(
    source: &str,
    fragment: &SurfaceFragment,
    fragment_kind: FragmentKind,
) -> Result<PatchNode, PatchTextError> {
    match fragment_kind {
        FragmentKind::Item => draxl_parser::parse_item_fragment(&fragment.source)
            .map(PatchNode::Item)
            .map_err(|error| map_fragment_parse_error(source, fragment.span.start, error)),
        FragmentKind::Field => draxl_parser::parse_field_fragment(&fragment.source)
            .map(PatchNode::Field)
            .map_err(|error| map_fragment_parse_error(source, fragment.span.start, error)),
        FragmentKind::Variant => draxl_parser::parse_variant_fragment(&fragment.source)
            .map(PatchNode::Variant)
            .map_err(|error| map_fragment_parse_error(source, fragment.span.start, error)),
        FragmentKind::Param => draxl_parser::parse_param_fragment(&fragment.source)
            .map(PatchNode::Param)
            .map_err(|error| map_fragment_parse_error(source, fragment.span.start, error)),
        FragmentKind::Stmt => draxl_parser::parse_stmt_fragment(&fragment.source)
            .map(PatchNode::Stmt)
            .map_err(|error| map_fragment_parse_error(source, fragment.span.start, error)),
        FragmentKind::MatchArm => draxl_parser::parse_match_arm_fragment(&fragment.source)
            .map(PatchNode::MatchArm)
            .map_err(|error| map_fragment_parse_error(source, fragment.span.start, error)),
        FragmentKind::Expr => draxl_parser::parse_expr_fragment(&fragment.source)
            .map(PatchNode::Expr)
            .map_err(|error| map_fragment_parse_error(source, fragment.span.start, error)),
        FragmentKind::Type => draxl_parser::parse_type_fragment(&fragment.source)
            .map(PatchNode::Type)
            .map_err(|error| map_fragment_parse_error(source, fragment.span.start, error)),
        FragmentKind::Pattern => draxl_parser::parse_pattern_fragment(&fragment.source)
            .map(PatchNode::Pattern)
            .map_err(|error| map_fragment_parse_error(source, fragment.span.start, error)),
        FragmentKind::Doc => draxl_parser::parse_doc_fragment(&fragment.source)
            .map(PatchNode::Doc)
            .map_err(|error| map_fragment_parse_error(source, fragment.span.start, error)),
        FragmentKind::Comment => draxl_parser::parse_comment_fragment(&fragment.source)
            .map(PatchNode::Comment)
            .map_err(|error| map_fragment_parse_error(source, fragment.span.start, error)),
    }
}

fn dest_owner_kind(
    file: &File,
    source: &str,
    owner: &SurfaceSlotOwner,
) -> Result<NodeKind, PatchTextError> {
    match owner {
        SurfaceSlotOwner::File { .. } => Ok(NodeKind::File),
        SurfaceSlotOwner::Node(node) => resolve_node_kind(file, source, node),
    }
}

fn ensure_single_segment_path(source: &str, path: &SurfacePath) -> Result<(), PatchTextError> {
    if path.segments.len() == 1 {
        Ok(())
    } else {
        Err(patch_text_error(
            source,
            path.span,
            "only single-segment scalar patch paths are supported in the current Rust profile",
        ))
    }
}

fn ensure_node_exists(
    file: &File,
    source: &str,
    node: &SurfaceNodeRef,
) -> Result<SurfaceNodeRef, PatchTextError> {
    if find_node_kind(file, &node.id).is_some() {
        Ok(node.clone())
    } else {
        Err(patch_text_error(
            source,
            node.span,
            &format!("node `@{}` was not found", node.id),
        ))
    }
}

fn resolve_node_kind(
    file: &File,
    source: &str,
    node: &SurfaceNodeRef,
) -> Result<NodeKind, PatchTextError> {
    find_node_kind(file, &node.id).ok_or_else(|| {
        patch_text_error(
            source,
            node.span,
            &format!("node `@{}` was not found", node.id),
        )
    })
}

fn owner_label(owner: &SurfaceSlotOwner) -> String {
    match owner {
        SurfaceSlotOwner::File { .. } => "file".to_owned(),
        SurfaceSlotOwner::Node(node) => format!("@{}", node.id),
    }
}
