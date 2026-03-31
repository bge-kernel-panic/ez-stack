use std::collections::HashSet;

use crate::stack::ScopeMode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeDecision {
    NoConfig,
    InBounds(ScopeReport),
    OutOfBounds(ScopeReport),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeReport {
    pub mode: ScopeMode,
    pub patterns: Vec<String>,
    pub in_scope_files: Vec<String>,
    pub out_of_scope_files: Vec<String>,
}

pub fn evaluate_scope(
    patterns: &[String],
    mode: ScopeMode,
    staged_files: &[String],
    matched_files: &[String],
) -> ScopeDecision {
    if patterns.is_empty() {
        return ScopeDecision::NoConfig;
    }

    let matched: HashSet<&str> = matched_files.iter().map(String::as_str).collect();
    let in_scope_files: Vec<String> = staged_files
        .iter()
        .filter(|file| matched.contains(file.as_str()))
        .cloned()
        .collect();
    let out_of_scope_files: Vec<String> = staged_files
        .iter()
        .filter(|file| !matched.contains(file.as_str()))
        .cloned()
        .collect();

    let report = ScopeReport {
        mode,
        patterns: patterns.to_vec(),
        in_scope_files,
        out_of_scope_files,
    };

    if report.out_of_scope_files.is_empty() {
        ScopeDecision::InBounds(report)
    } else {
        ScopeDecision::OutOfBounds(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn to_strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn evaluate_scope_without_patterns_returns_no_scope() {
        let decision = evaluate_scope(&[], ScopeMode::Warn, &to_strings(&["src/a.rs"]), &[]);
        assert_eq!(decision, ScopeDecision::NoConfig);
    }

    #[test]
    fn evaluate_scope_with_all_files_in_scope_returns_in_scope() {
        let patterns = to_strings(&["src/auth/**"]);
        let staged = to_strings(&["src/auth/a.rs", "src/auth/b.rs"]);
        let matched = staged.clone();
        let decision = evaluate_scope(&patterns, ScopeMode::Warn, &staged, &matched);

        match decision {
            ScopeDecision::InBounds(report) => {
                assert_eq!(report.in_scope_files, staged);
                assert!(report.out_of_scope_files.is_empty());
            }
            other => panic!("expected InBounds, got {other:?}"),
        }
    }

    #[test]
    fn evaluate_scope_with_mixed_files_returns_out_of_scope() {
        let patterns = to_strings(&["src/auth/**"]);
        let staged = to_strings(&["src/auth/a.rs", "src/billing/b.rs"]);
        let matched = to_strings(&["src/auth/a.rs"]);
        let decision = evaluate_scope(&patterns, ScopeMode::Strict, &staged, &matched);

        match decision {
            ScopeDecision::OutOfBounds(report) => {
                assert_eq!(report.in_scope_files, to_strings(&["src/auth/a.rs"]));
                assert_eq!(report.out_of_scope_files, to_strings(&["src/billing/b.rs"]));
                assert_eq!(report.mode, ScopeMode::Strict);
            }
            other => panic!("expected OutOfBounds, got {other:?}"),
        }
    }
}
