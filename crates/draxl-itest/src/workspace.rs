use crate::error::{Result, ToolError};
use crate::support::{
    build_replace_fragment, build_simple_stmt_fragment, check_expected_fingerprint,
    collect_node_infos, display_relative, fingerprint_for_source, format_validation_errors,
    format_value, legal_info, read_source, resolve_dx_path, sibling_rank_infos,
    snapshot_for_source,
};
use crate::types::{
    ApplyPatchTextRequest, ConflictCheckResult, FileInspection, InsertAfterStmtRequest, NodeDetail,
    NodeSummary, PatchApplicationResult, ReplaceNodeRequest, SetPathValueRequest,
};
use draxl::{
    apply_patch_text_for_language, check_conflicts_json_for_language, format_file_for_language,
    parse_and_validate_for_language, resolve_patch_ops_for_language, validate_file, LowerLanguage,
};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ToolWorkspace {
    root: PathBuf,
}

impl ToolWorkspace {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        let root = root.canonicalize().map_err(|err| {
            ToolError::new(format!("failed to canonicalize {}: {err}", root.display()))
        })?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn inspect_file(&self, relative_path: &str) -> Result<FileInspection> {
        let absolute_path = resolve_dx_path(&self.root, relative_path)?;
        let source = read_source(&absolute_path)?;
        let snapshot = snapshot_for_source(&source)?;
        let nodes = collect_node_infos(&snapshot.ast_json)
            .into_iter()
            .map(|info| NodeSummary {
                id: info.id,
                kind: info.kind,
                parent_id: info.parent_id,
                slot: info.slot,
                rank: info.rank,
                summary: info.summary,
            })
            .collect::<Vec<_>>();

        Ok(FileInspection {
            ok: true,
            path: display_relative(&self.root, &absolute_path),
            fingerprint: snapshot.fingerprint,
            source,
            node_count: nodes.len(),
            nodes,
        })
    }

    pub fn get_node(&self, relative_path: &str, node_id: &str) -> Result<NodeDetail> {
        let absolute_path = resolve_dx_path(&self.root, relative_path)?;
        let source = read_source(&absolute_path)?;
        let snapshot = snapshot_for_source(&source)?;
        let info = collect_node_infos(&snapshot.ast_json)
            .into_iter()
            .find(|info| info.id == node_id)
            .ok_or_else(|| ToolError::new(format!("node @{node_id} was not found")))?;

        Ok(NodeDetail {
            ok: true,
            path: display_relative(&self.root, &absolute_path),
            fingerprint: snapshot.fingerprint,
            node_id: info.id,
            kind: info.kind.clone(),
            parent_id: info.parent_id,
            slot: info.slot,
            rank: info.rank,
            legal: legal_info(&info.kind),
            node: info.node,
        })
    }

    pub fn replace_node(&self, request: ReplaceNodeRequest<'_>) -> Result<PatchApplicationResult> {
        let fragment = build_replace_fragment(request.target_id, request.fragment_source);
        let patch_text = format!("replace @{}: {}\n", request.target_id, fragment);
        self.apply_patch_text(ApplyPatchTextRequest {
            path: request.path,
            patch_text: &patch_text,
            expected_fingerprint: request.expected_fingerprint,
            apply: request.apply,
        })
    }

    pub fn insert_after_stmt(
        &self,
        request: InsertAfterStmtRequest<'_>,
    ) -> Result<PatchApplicationResult> {
        let absolute_path = resolve_dx_path(&self.root, request.path)?;
        let source = read_source(&absolute_path)?;
        let snapshot = snapshot_for_source(&source)?;
        check_expected_fingerprint(request.expected_fingerprint, &snapshot.fingerprint)?;

        let anchor_info = collect_node_infos(&snapshot.ast_json)
            .into_iter()
            .find(|info| info.id == request.anchor_id)
            .ok_or_else(|| ToolError::new(format!("node @{} was not found", request.anchor_id)))?;

        if anchor_info.slot.as_deref() != Some("body")
            || anchor_info.rank.is_none()
            || anchor_info.parent_id.is_none()
        {
            return Err(ToolError::new(
                "anchor_id must refer to a ranked statement inside a body slot",
            ));
        }

        let parent_id = anchor_info.parent_id.as_deref().expect("checked above");
        let siblings = sibling_rank_infos(&snapshot.ast_json, parent_id, "body");
        let anchor_index = siblings
            .iter()
            .position(|info| info.id == request.anchor_id)
            .ok_or_else(|| {
                ToolError::new("anchor statement was not found among ranked siblings")
            })?;
        let next_rank = siblings
            .get(anchor_index + 1)
            .and_then(|info| info.rank.as_deref());
        let anchor_rank = anchor_info.rank.as_deref().expect("checked above");
        let new_rank = crate::support::allocate_rank_between(anchor_rank, next_rank)?;
        let fragment = build_simple_stmt_fragment(request.stmt_source, &source)?;
        let patch_text = format!("insert @{}.body[{}]: {}\n", parent_id, new_rank, fragment);

        self.apply_patch_text(ApplyPatchTextRequest {
            path: request.path,
            patch_text: &patch_text,
            expected_fingerprint: request.expected_fingerprint,
            apply: request.apply,
        })
    }

    pub fn set_path_value(
        &self,
        request: SetPathValueRequest<'_>,
    ) -> Result<PatchApplicationResult> {
        let patch_text = format!(
            "set @{}.{} = {}\n",
            request.node_id,
            request.field,
            format_value(request.value_kind, request.value)?
        );
        self.apply_patch_text(ApplyPatchTextRequest {
            path: request.path,
            patch_text: &patch_text,
            expected_fingerprint: request.expected_fingerprint,
            apply: request.apply,
        })
    }

    pub fn apply_patch_text(
        &self,
        request: ApplyPatchTextRequest<'_>,
    ) -> Result<PatchApplicationResult> {
        let absolute_path = resolve_dx_path(&self.root, request.path)?;
        let source = read_source(&absolute_path)?;
        let snapshot = snapshot_for_source(&source)?;
        check_expected_fingerprint(request.expected_fingerprint, &snapshot.fingerprint)?;

        let mut file = parse_and_validate_for_language(LowerLanguage::Rust, &source)
            .map_err(|err| ToolError::new(err.to_string()))?;
        apply_patch_text_for_language(LowerLanguage::Rust, &mut file, request.patch_text)
            .map_err(|err| ToolError::new(err.to_string()))?;
        validate_file(&file).map_err(format_validation_errors)?;
        let preview_dx = format_file_for_language(LowerLanguage::Rust, &file);

        if request.apply {
            fs::write(&absolute_path, &preview_dx).map_err(|err| {
                ToolError::new(format!(
                    "failed to write {}: {err}",
                    absolute_path.display()
                ))
            })?;
        }

        let after_fingerprint = fingerprint_for_source(&preview_dx)?;

        Ok(PatchApplicationResult {
            ok: true,
            path: display_relative(&self.root, &absolute_path),
            applied: request.apply,
            before_fingerprint: snapshot.fingerprint,
            after_fingerprint,
            patch_text: request.patch_text.to_owned(),
            preview_dx,
        })
    }

    pub fn check_conflicts(
        &self,
        request: crate::types::CheckConflictsRequest<'_>,
    ) -> Result<ConflictCheckResult> {
        let absolute_path = resolve_dx_path(&self.root, request.path)?;
        let source = read_source(&absolute_path)?;
        let file = parse_and_validate_for_language(LowerLanguage::Rust, &source)
            .map_err(|err| ToolError::new(err.to_string()))?;
        let left_ops =
            resolve_patch_ops_for_language(LowerLanguage::Rust, &file, request.left_patch_text)
                .map_err(|err| ToolError::new(err.to_string()))?;
        let right_ops =
            resolve_patch_ops_for_language(LowerLanguage::Rust, &file, request.right_patch_text)
                .map_err(|err| ToolError::new(err.to_string()))?;
        let conflicts = serde_json::from_str::<Value>(&check_conflicts_json_for_language(
            LowerLanguage::Rust,
            &file,
            &left_ops,
            &right_ops,
        ))
        .map_err(|err| ToolError::new(format!("failed to decode conflict json: {err}")))?;

        Ok(ConflictCheckResult {
            ok: true,
            path: display_relative(&self.root, &absolute_path),
            conflicts,
        })
    }
}
