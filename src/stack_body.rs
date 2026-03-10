/// One upstream ancestor branch and its PR info (if any).
pub struct AncestorPr {
    pub branch: String,
    pub pr_number: Option<u64>,
    pub pr_url: Option<String>, // pre-resolved, e.g. "https://github.com/org/repo/pull/42"
}

/// Returns the markdown stack section, or None if no ancestors have a PR number.
/// Ancestors are expected in trunk-closest-first order.
/// Only ancestors with a pr_number are listed; ancestors without one are skipped.
pub fn build_stack_section(ancestors: &[AncestorPr]) -> Option<String> {
    let linked: Vec<String> = ancestors
        .iter()
        .filter_map(|a| a.pr_number.map(|num| (a, num)))
        .enumerate()
        .map(|(i, (a, num))| match &a.pr_url {
            Some(url) => format!("{}. [{} #{}]({})", i + 1, a.branch, num, url),
            None => format!("{}. {} #{}", i + 1, a.branch, num),
        })
        .collect();

    if linked.is_empty() {
        None
    } else {
        Some(format!("**Stack:**\n{}", linked.join("\n")))
    }
}

/// Returns the full PR body: user_body, then (if any ancestors have PRs) a
/// separator and the stack section appended.
pub fn build_stack_body(ancestors: &[AncestorPr], user_body: &str) -> String {
    match build_stack_section(ancestors) {
        Some(section) => format!("{}\n\n---\n\n{}", user_body, section),
        None => user_body.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anc(branch: &str, number: Option<u64>, url: Option<&str>) -> AncestorPr {
        AncestorPr {
            branch: branch.to_string(),
            pr_number: number,
            pr_url: url.map(|s| s.to_string()),
        }
    }

    // --- build_stack_section ---

    #[test]
    fn section_empty_ancestors_returns_none() {
        assert!(build_stack_section(&[]).is_none());
    }

    #[test]
    fn section_ancestor_without_pr_returns_none() {
        let ancestors = vec![anc("feat/a", None, None)];
        assert!(build_stack_section(&ancestors).is_none());
    }

    #[test]
    fn section_one_ancestor_with_url() {
        let ancestors = vec![anc(
            "feat/a",
            Some(101),
            Some("https://github.com/org/repo/pull/101"),
        )];
        let section = build_stack_section(&ancestors).unwrap();
        assert_eq!(
            section,
            "**Stack:**\n1. [feat/a #101](https://github.com/org/repo/pull/101)"
        );
    }

    #[test]
    fn section_one_ancestor_without_url() {
        let ancestors = vec![anc("feat/a", Some(101), None)];
        let section = build_stack_section(&ancestors).unwrap();
        assert_eq!(section, "**Stack:**\n1. feat/a #101");
    }

    #[test]
    fn section_skips_ancestors_without_pr_number() {
        let ancestors = vec![
            anc(
                "feat/a",
                Some(101),
                Some("https://github.com/org/repo/pull/101"),
            ),
            anc("feat/b", None, None),
            anc(
                "feat/c",
                Some(103),
                Some("https://github.com/org/repo/pull/103"),
            ),
        ];
        let section = build_stack_section(&ancestors).unwrap();
        assert_eq!(
            section,
            "**Stack:**\n1. [feat/a #101](https://github.com/org/repo/pull/101)\n2. [feat/c #103](https://github.com/org/repo/pull/103)"
        );
    }

    #[test]
    fn section_numbers_are_sequential_for_linked_only() {
        let ancestors = vec![
            anc("feat/a", None, None),
            anc(
                "feat/b",
                Some(102),
                Some("https://github.com/org/repo/pull/102"),
            ),
            anc(
                "feat/c",
                Some(103),
                Some("https://github.com/org/repo/pull/103"),
            ),
        ];
        let section = build_stack_section(&ancestors).unwrap();
        assert!(section.contains("1. [feat/b"));
        assert!(section.contains("2. [feat/c"));
    }

    // --- build_stack_body ---

    #[test]
    fn body_no_ancestors_returns_user_body_unchanged() {
        let result = build_stack_body(&[], "My PR description.");
        assert_eq!(result, "My PR description.");
    }

    #[test]
    fn body_ancestors_without_prs_returns_user_body_unchanged() {
        let ancestors = vec![anc("feat/a", None, None)];
        let result = build_stack_body(&ancestors, "My PR description.");
        assert_eq!(result, "My PR description.");
    }

    #[test]
    fn body_appends_section_after_separator() {
        let ancestors = vec![anc(
            "feat/a",
            Some(101),
            Some("https://github.com/org/repo/pull/101"),
        )];
        let result = build_stack_body(&ancestors, "My PR description.");
        assert_eq!(
            result,
            "My PR description.\n\n---\n\n**Stack:**\n1. [feat/a #101](https://github.com/org/repo/pull/101)"
        );
    }

    #[test]
    fn body_preserves_user_body_above_section() {
        let ancestors = vec![anc("feat/a", Some(101), None)];
        let body = "This PR adds X.\n\nMore details here.";
        let result = build_stack_body(&ancestors, body);
        assert!(result.starts_with("This PR adds X.\n\nMore details here."));
        assert!(result.contains("\n\n---\n\n**Stack:**"));
    }
}
