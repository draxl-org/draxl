use crate::error::{Result, ToolError};
use crate::types::{LegalInfo, ScalarValue, ValueKind};
use draxl::{dump_json_file, parse_and_validate_for_language, LowerLanguage};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone)]
pub(crate) struct Snapshot {
    pub(crate) ast_json: Value,
    pub(crate) fingerprint: String,
}

#[derive(Debug, Clone)]
pub(crate) struct NodeInfo {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) parent_id: Option<String>,
    pub(crate) slot: Option<String>,
    pub(crate) rank: Option<String>,
    pub(crate) summary: Option<String>,
    pub(crate) node: Value,
}

pub(crate) fn resolve_dx_path(root: &Path, relative_path: &str) -> Result<PathBuf> {
    if relative_path.is_empty() {
        return Err(ToolError::new("path must be a non-empty string"));
    }

    let normalized = normalize_path(&root.join(relative_path));
    if !normalized.starts_with(root) {
        return Err(ToolError::new(
            "path must stay inside the configured workspace",
        ));
    }
    if normalized.extension().and_then(|ext| ext.to_str()) != Some("dx")
        || !normalized
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".rs.dx"))
    {
        return Err(ToolError::new("path must point to a .rs.dx file"));
    }
    Ok(normalized)
}

pub(crate) fn display_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

pub(crate) fn read_source(path: &Path) -> Result<String> {
    fs::read_to_string(path)
        .map_err(|err| ToolError::new(format!("failed to read {}: {err}", path.display())))
}

pub(crate) fn snapshot_for_source(source: &str) -> Result<Snapshot> {
    let file = parse_and_validate_for_language(LowerLanguage::Rust, source)
        .map_err(|err| ToolError::new(err.to_string()))?;
    let ast_json = serde_json::from_str::<Value>(&dump_json_file(&file))
        .map_err(|err| ToolError::new(format!("failed to decode ast json: {err}")))?;
    let encoded = serde_json::to_vec(&ast_json)
        .map_err(|err| ToolError::new(format!("failed to encode ast json: {err}")))?;
    let fingerprint = format!("sha256:{:x}", Sha256::digest(encoded));
    Ok(Snapshot {
        ast_json,
        fingerprint,
    })
}

pub(crate) fn fingerprint_for_source(source: &str) -> Result<String> {
    snapshot_for_source(source).map(|snapshot| snapshot.fingerprint)
}

pub(crate) fn collect_node_infos(root: &Value) -> Vec<NodeInfo> {
    let mut out = Vec::new();
    collect_node_infos_inner(root, None, &mut out);
    out
}

pub(crate) fn sibling_rank_infos(root: &Value, parent_id: &str, slot: &str) -> Vec<NodeInfo> {
    let mut siblings = collect_node_infos(root)
        .into_iter()
        .filter(|info| {
            info.parent_id.as_deref() == Some(parent_id)
                && info.slot.as_deref() == Some(slot)
                && info.rank.is_some()
        })
        .collect::<Vec<_>>();
    siblings.sort_by(|left, right| {
        compare_ranks(
            left.rank.as_deref().unwrap_or(""),
            right.rank.as_deref().unwrap_or(""),
        )
    });
    siblings
}

pub(crate) fn legal_info(kind: &str) -> LegalInfo {
    match kind {
        "Mod" => LegalInfo {
            ranked_slots: vec!["items"],
            single_slots: Vec::new(),
            set_paths: vec!["name"],
        },
        "Struct" => LegalInfo {
            ranked_slots: vec!["fields"],
            single_slots: Vec::new(),
            set_paths: vec!["name"],
        },
        "Enum" => LegalInfo {
            ranked_slots: vec!["variants"],
            single_slots: Vec::new(),
            set_paths: vec!["name"],
        },
        "Fn" => LegalInfo {
            ranked_slots: vec!["params", "body"],
            single_slots: vec!["ret"],
            set_paths: vec!["name"],
        },
        "Field" => LegalInfo {
            ranked_slots: Vec::new(),
            single_slots: vec!["ty"],
            set_paths: vec!["name"],
        },
        "Variant" => LegalInfo {
            ranked_slots: Vec::new(),
            single_slots: Vec::new(),
            set_paths: vec!["name"],
        },
        "Param" => LegalInfo {
            ranked_slots: Vec::new(),
            single_slots: vec!["ty"],
            set_paths: vec!["name"],
        },
        "Let" => LegalInfo {
            ranked_slots: Vec::new(),
            single_slots: vec!["pat", "init"],
            set_paths: Vec::new(),
        },
        "Expr" => LegalInfo {
            ranked_slots: Vec::new(),
            single_slots: vec!["expr"],
            set_paths: vec!["semi"],
        },
        "Binary" => LegalInfo {
            ranked_slots: Vec::new(),
            single_slots: vec!["lhs", "rhs"],
            set_paths: vec!["op"],
        },
        "Unary" => LegalInfo {
            ranked_slots: Vec::new(),
            single_slots: vec!["expr"],
            set_paths: vec!["op"],
        },
        "Call" => LegalInfo {
            ranked_slots: Vec::new(),
            single_slots: vec!["callee"],
            set_paths: Vec::new(),
        },
        "Match" => LegalInfo {
            ranked_slots: vec!["arms"],
            single_slots: vec!["scrutinee"],
            set_paths: Vec::new(),
        },
        "MatchArm" => LegalInfo {
            ranked_slots: Vec::new(),
            single_slots: vec!["pat", "guard", "body"],
            set_paths: Vec::new(),
        },
        "Ident" => LegalInfo {
            ranked_slots: Vec::new(),
            single_slots: Vec::new(),
            set_paths: vec!["name"],
        },
        "Wild" => LegalInfo {
            ranked_slots: Vec::new(),
            single_slots: Vec::new(),
            set_paths: Vec::new(),
        },
        "Group" => LegalInfo {
            ranked_slots: Vec::new(),
            single_slots: vec!["expr"],
            set_paths: Vec::new(),
        },
        "Doc" => LegalInfo {
            ranked_slots: Vec::new(),
            single_slots: Vec::new(),
            set_paths: vec!["text"],
        },
        "Comment" => LegalInfo {
            ranked_slots: Vec::new(),
            single_slots: Vec::new(),
            set_paths: vec!["text"],
        },
        "Path" | "Lit" => LegalInfo {
            ranked_slots: Vec::new(),
            single_slots: Vec::new(),
            set_paths: Vec::new(),
        },
        _ => LegalInfo {
            ranked_slots: Vec::new(),
            single_slots: Vec::new(),
            set_paths: Vec::new(),
        },
    }
}

pub(crate) fn format_value(value_kind: ValueKind, value: ScalarValue<'_>) -> Result<String> {
    match (value_kind, value) {
        (ValueKind::Ident, ScalarValue::Ident(value)) => Ok(value.to_owned()),
        (ValueKind::String, ScalarValue::String(value)) => serde_json::to_string(value)
            .map_err(|err| ToolError::new(format!("failed to encode string value: {err}"))),
        (ValueKind::Bool, ScalarValue::Bool(value)) => {
            Ok(if value { "true" } else { "false" }.to_owned())
        }
        (ValueKind::Int, ScalarValue::Int(value)) => Ok(value.to_string()),
        (ValueKind::Ident, _)
        | (ValueKind::String, _)
        | (ValueKind::Bool, _)
        | (ValueKind::Int, _) => Err(ToolError::new(
            "value_kind did not match the provided value",
        )),
    }
}

pub(crate) fn build_simple_stmt_fragment(
    stmt_source: &str,
    current_source: &str,
) -> Result<String> {
    let trimmed = stmt_source.trim();
    if trimmed.starts_with('@') {
        return Ok(trimmed.to_owned());
    }

    let name = parse_zero_arg_call_stmt(trimmed).ok_or_else(|| {
        ToolError::new(
            "plain stmt_source currently supports only simple zero-arg call statements like `trace();`; use a full Draxl stmt fragment starting with `@` for more complex inserts",
        )
    })?;
    let ids = allocate_ids(current_source, &["s", "e"]);
    Ok(format!(
        "@{} @{} {}();",
        ids.get("s").expect("allocated id should exist"),
        ids.get("e").expect("allocated id should exist"),
        name
    ))
}

pub(crate) fn build_replace_fragment(target_id: &str, fragment_source: &str) -> String {
    let trimmed = fragment_source.trim();
    if trimmed.starts_with('@') {
        trimmed.to_owned()
    } else {
        format!("@{target_id} {trimmed}")
    }
}

pub(crate) fn check_expected_fingerprint(expected: Option<&str>, actual: &str) -> Result<()> {
    if let Some(expected) = expected {
        if expected != actual {
            return Err(ToolError::new(format!(
                "fingerprint mismatch: expected {expected}, current file is {actual}"
            )));
        }
    }
    Ok(())
}

pub(crate) fn format_validation_errors(errors: Vec<draxl::validate::ValidationError>) -> ToolError {
    let mut message = String::from("validation failed:");
    for error in errors {
        message.push('\n');
        message.push_str("- ");
        message.push_str(&error.message);
    }
    ToolError::new(message)
}

pub(crate) fn allocate_rank_between(
    previous_rank: &str,
    next_rank: Option<&str>,
) -> Result<String> {
    if previous_rank.is_empty() {
        return Err(ToolError::new("previous rank must be a non-empty string"));
    }
    let Some(next_rank) = next_rank else {
        return Ok(format!("{previous_rank}m"));
    };
    if compare_ranks(previous_rank, next_rank) != Ordering::Less {
        return Err(ToolError::new(format!(
            "cannot allocate a rank between {previous_rank} and {next_rank}"
        )));
    }
    if !next_rank.starts_with(previous_rank) {
        return Ok(format!("{previous_rank}m"));
    }

    let suffix = allocate_suffix_below(&next_rank[previous_rank.len()..]).ok_or_else(|| {
        ToolError::new(format!(
            "cannot allocate a rank between {previous_rank} and {next_rank}"
        ))
    })?;
    Ok(format!("{previous_rank}{suffix}"))
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn collect_node_infos_inner(value: &Value, parent_id: Option<&str>, out: &mut Vec<NodeInfo>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_node_infos_inner(item, parent_id, out);
            }
        }
        Value::Object(map) => {
            let mut next_parent = parent_id.map(str::to_owned);
            if let (Some(kind), Some(meta)) = (
                map.get("kind").and_then(Value::as_str),
                map.get("meta").and_then(Value::as_object),
            ) {
                if let Some(id) = meta.get("id").and_then(Value::as_str) {
                    out.push(NodeInfo {
                        id: id.to_owned(),
                        kind: kind.to_owned(),
                        parent_id: parent_id.map(str::to_owned),
                        slot: meta.get("slot").and_then(Value::as_str).map(str::to_owned),
                        rank: meta.get("rank").and_then(Value::as_str).map(str::to_owned),
                        summary: summarize_node(map),
                        node: value.clone(),
                    });
                    next_parent = Some(id.to_owned());
                }
            }

            for child in map.values() {
                collect_node_infos_inner(child, next_parent.as_deref(), out);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn summarize_node(map: &serde_json::Map<String, Value>) -> Option<String> {
    let kind = map.get("kind").and_then(Value::as_str)?;
    match kind {
        "Fn" | "Mod" | "Struct" | "Enum" | "Variant" | "Field" | "Param" | "Ident" => {
            map.get("name").and_then(Value::as_str).map(str::to_owned)
        }
        "Doc" | "Comment" | "Lit" | "Path" => map.get("text").and_then(Value::as_str).map(|text| {
            const LIMIT: usize = 32;
            if text.len() <= LIMIT {
                text.to_owned()
            } else {
                format!("{}...", &text[..LIMIT])
            }
        }),
        "Binary" | "Unary" => map.get("op").and_then(Value::as_str).map(str::to_owned),
        _ => None,
    }
}

fn parse_zero_arg_call_stmt(source: &str) -> Option<&str> {
    let stripped = source.strip_suffix(';')?.trim_end();
    let name = stripped.strip_suffix("()")?.trim();
    (!name.is_empty() && is_plain_ident(name)).then_some(name)
}

fn is_plain_ident(value: &str) -> bool {
    let mut chars = value.chars();
    let first = chars
        .next()
        .filter(|ch| ch.is_ascii_alphabetic() || *ch == '_');
    first.is_some() && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn allocate_ids(source: &str, prefixes: &[&str]) -> BTreeMap<String, String> {
    let mut max_by_prefix = BTreeMap::<String, u64>::new();
    let bytes = source.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] != b'@' {
            index += 1;
            continue;
        }

        let mut cursor = index + 1;
        if cursor >= bytes.len() || !bytes[cursor].is_ascii_alphabetic() {
            index += 1;
            continue;
        }

        let prefix_start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_alphabetic() {
            cursor += 1;
        }
        let digits_start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }

        if digits_start == cursor {
            index = cursor;
            continue;
        }

        let prefix = &source[prefix_start..digits_start];
        let value = source[digits_start..cursor].parse::<u64>().unwrap_or(0);
        max_by_prefix
            .entry(prefix.to_owned())
            .and_modify(|current| *current = (*current).max(value))
            .or_insert(value);
        index = cursor;
    }

    let mut allocated = BTreeMap::new();
    for prefix in prefixes {
        let next = max_by_prefix.get(*prefix).copied().unwrap_or(0) + 1;
        allocated.insert((*prefix).to_owned(), format!("{prefix}{next}"));
    }
    allocated
}

fn compare_ranks(left: &str, right: &str) -> Ordering {
    left.cmp(right)
}

fn allocate_suffix_below(upper_suffix: &str) -> Option<String> {
    let mut chars = upper_suffix.chars();
    let first = chars.next()?;
    if let Some(lower) = preferred_char_below(first) {
        return Some(lower.to_string());
    }

    let tail = allocate_suffix_below(chars.as_str())?;
    let mut out = String::new();
    out.push(first);
    out.push_str(&tail);
    Some(out)
}

fn preferred_char_below(ch: char) -> Option<char> {
    match ch {
        'b'..='z' => Some(char::from_u32((ch as u32) - 1).expect("ascii lower letter")),
        'a' => Some('M'),
        'B'..='Z' => Some(char::from_u32((ch as u32) - 1).expect("ascii upper letter")),
        'A' => Some('5'),
        '_' => Some('Z'),
        '1'..='9' => Some(char::from_u32((ch as u32) - 1).expect("ascii digit")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{allocate_rank_between, parse_zero_arg_call_stmt};

    #[test]
    fn allocate_rank_between_handles_the_rank_example() {
        assert_eq!(allocate_rank_between("a", Some("am")).unwrap(), "al");
        assert_eq!(allocate_rank_between("am", Some("b")).unwrap(), "amm");
        assert_eq!(allocate_rank_between("b", None).unwrap(), "bm");
    }

    #[test]
    fn simple_stmt_parser_accepts_zero_arg_calls() {
        assert_eq!(parse_zero_arg_call_stmt("trace();"), Some("trace"));
        assert_eq!(parse_zero_arg_call_stmt("trace ( );"), None);
        assert_eq!(parse_zero_arg_call_stmt("trace(1);"), None);
    }
}
