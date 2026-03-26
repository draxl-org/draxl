mod error;
mod parse;
mod resolve;
mod surface;

use crate::model::PatchOp;
use draxl_ast::{File, LowerLanguage};

pub use error::PatchTextError;
pub use surface::{
    SurfaceDest, SurfaceFragment, SurfaceNodeRef, SurfacePatchOp, SurfacePath, SurfacePathSegment,
    SurfaceRankedDest, SurfaceSlotOwner, SurfaceSlotRef, SurfaceValue, SurfaceValueKind,
};

/// Parses canonical textual patch syntax into unresolved surface ops.
pub fn parse_patch_ops(source: &str) -> Result<Vec<SurfacePatchOp>, PatchTextError> {
    parse::parse_patch_ops(source)
}

/// Resolves textual patch ops against the current file into ordinary `PatchOp`s.
pub fn resolve_patch_ops(file: &File, source: &str) -> Result<Vec<PatchOp>, PatchTextError> {
    resolve_patch_ops_for_language(LowerLanguage::Rust, file, source)
}

/// Resolves textual patch ops against the current file into ordinary `PatchOp`s using the selected
/// lower language.
pub fn resolve_patch_ops_for_language(
    language: LowerLanguage,
    file: &File,
    source: &str,
) -> Result<Vec<PatchOp>, PatchTextError> {
    let surface_ops = parse_patch_ops(source)?;
    let resolved = resolve::resolve_patch_ops(language, file, source, &surface_ops)?;
    Ok(resolved.into_iter().map(|op| op.op).collect())
}

/// Parses, resolves, and applies textual patch ops in order.
pub fn apply_patch_text(file: &mut File, source: &str) -> Result<(), PatchTextError> {
    apply_patch_text_for_language(LowerLanguage::Rust, file, source)
}

/// Parses, resolves, and applies textual patch ops in order using the selected lower language.
pub fn apply_patch_text_for_language(
    language: LowerLanguage,
    file: &mut File,
    source: &str,
) -> Result<(), PatchTextError> {
    let surface_ops = parse_patch_ops(source)?;
    for surface_op in &surface_ops {
        let resolved = resolve::resolve_op(language, file, source, surface_op)?;
        crate::apply_op(file, resolved.op)
            .map_err(|error| error::patch_text_error(source, resolved.span, &error.message))?;
    }
    Ok(())
}
