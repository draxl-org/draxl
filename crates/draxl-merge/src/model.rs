use serde::Serialize;
use std::fmt;

/// Outcome of checking two patch streams for merge conflicts.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct ConflictReport {
    /// Conflicts found while comparing the two patch streams.
    pub conflicts: Vec<Conflict>,
}

impl ConflictReport {
    /// Returns true when no conflicts were found.
    pub fn is_clean(&self) -> bool {
        self.conflicts.is_empty()
    }

    /// Returns true when at least one conflict was found.
    pub fn has_conflicts(&self) -> bool {
        !self.is_clean()
    }

    /// Emits deterministic JSON for the conflict report.
    pub fn to_json_pretty(&self) -> String {
        let mut out =
            serde_json::to_string_pretty(self).expect("conflict report JSON serialization");
        out.push('\n');
        out
    }
}

impl fmt::Display for ConflictReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_json_pretty())
    }
}

/// Structured conflict explanation suitable for humans and agents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Conflict {
    /// Broad conflict class.
    pub class: ConflictClass,
    /// Stable conflict code.
    pub code: ConflictCode,
    /// Semantic owner when the conflict is tied to one.
    pub owner: Option<ConflictOwner>,
    /// Meaning-bearing regions touched by the left-side operations.
    pub left_regions: Vec<ConflictRegion>,
    /// Meaning-bearing regions touched by the right-side operations.
    pub right_regions: Vec<ConflictRegion>,
    /// Short one-line summary.
    #[serde(skip_serializing)]
    pub summary: String,
    /// Richer explanation of why the conflict exists.
    #[serde(skip_serializing)]
    pub detail: String,
    /// Relevant left-side operations.
    pub left: Vec<ConflictSide>,
    /// Relevant right-side operations.
    pub right: Vec<ConflictSide>,
    /// Suggested next step.
    #[serde(skip_serializing)]
    pub remediation: Option<String>,
}

impl fmt::Display for Conflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "[{:?}:{:?}] {}", self.class, self.code, self.summary)?;
        writeln!(f, "{}", self.detail)?;

        if !self.left.is_empty() {
            writeln!(f, "left:")?;
            for side in &self.left {
                writeln!(f, "- {}", side)?;
            }
        }

        if !self.right.is_empty() {
            writeln!(f, "right:")?;
            for side in &self.right {
                writeln!(f, "- {}", side)?;
            }
        }

        if let Some(remediation) = &self.remediation {
            write!(f, "next: {remediation}")?;
        }

        Ok(())
    }
}

impl Conflict {
    /// Emits deterministic JSON for one conflict.
    pub fn to_json_pretty(&self) -> String {
        let mut out = serde_json::to_string_pretty(self).expect("conflict JSON serialization");
        out.push('\n');
        out
    }
}

/// Broad conflict class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictClass {
    /// Hard conflicts stop deterministic auto-merge.
    Hard,
    /// Semantic conflicts are structurally mergeable but still need review.
    Semantic,
}

/// Stable conflict code for reporting and downstream handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictCode {
    /// Both sides wrote the same node shell in incompatible ways.
    SameNodeWrite,
    /// Both sides wrote the same scalar path in incompatible ways.
    SameScalarPathWrite,
    /// Both sides wrote the same single-child slot in incompatible ways.
    SameSingleSlotWrite,
    /// Both sides targeted the same ranked destination position.
    SameRankedPosition,
    /// Replay failed before convergence could be established.
    ReplayFailure,
    /// Both replay orders succeeded but the final ASTs diverged.
    NonConvergentReplay,
    /// One side renames a `let` binding while the other changes its initializer.
    BindingRenameVsInitializerChange,
    /// One side changes a parameter contract while the other changes body interpretation.
    ParameterTypeVsBodyInterpretationChange,
}

/// Semantic owner for machine-oriented conflict reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConflictOwner {
    Binding {
        let_id: String,
        binding_id: String,
    },
    Parameter {
        fn_id: String,
        param_id: String,
        param_name: String,
    },
}

/// Semantic region for machine-oriented conflict reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictRegion {
    BindingName,
    BindingInitializer,
    ParameterTypeContract,
    ParameterBodyInterpretation,
}

/// Relevant operation from one side of the conflict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConflictSide {
    /// Index inside the original patch stream.
    pub op_index: usize,
    /// Public patch op kind such as `replace` or `insert`.
    pub op_kind: &'static str,
    /// Compact target label such as `@e2` or `@f1.body[ah]`.
    pub target: String,
    /// Richer description of the operation.
    #[serde(skip_serializing)]
    pub description: String,
}

impl fmt::Display for ConflictSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "op {} `{}` on {}: {}",
            self.op_index, self.op_kind, self.target, self.description
        )
    }
}

/// Replay order used for hard-conflict checking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayOrder {
    /// Apply left, then right.
    LeftThenRight,
    /// Apply right, then left.
    RightThenLeft,
}

/// Replay stage that failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayStage {
    /// Failed while applying the indexed left-side op.
    LeftOp(usize),
    /// Failed while applying the indexed right-side op.
    RightOp(usize),
    /// Failed after patch replay during structural validation.
    Validation,
}

/// Internal replay failure used while classifying conflicts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReplayFailure {
    pub order: ReplayOrder,
    pub stage: ReplayStage,
    pub message: String,
}

/// Backward-compatible alias for the old hard-only report name.
pub type HardConflictReport = ConflictReport;
