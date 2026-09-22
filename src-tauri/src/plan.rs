//! The Plan: §4.2's wire contract, §4.3's validation, and the app-side enrichment.
//!
//! [`RawPlanEntry`] mirrors §4.2 exactly and is what a CLI adapter is parsed into.
//! [`PlanEntry`] is what the app works with — the same data plus provenance and the
//! user's review state, neither of which the CLI is allowed to influence.

use crate::error::{Error, Result};
use crate::scan::Scan;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Move,
    Skip,
    NeedsReview,
}

/// A deterministic rule the classifier proposes crystallising (§1.2 tier 2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuggestRule {
    /// `glob:Invoice_*.pdf`, `ext:pdf`, `regex:^IMG_\d+`, or `origin:billing.example.com`.
    #[serde(rename = "match")]
    pub match_expr: String,
    pub destination_key: String,
}

/// Which tier resolved this file (§1.2). Set by the app, never by the CLI — it powers
/// the "resolved without AI" counter and tells the user what they are trusting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolvedBy {
    /// Tier 1: extension map, user glob, regex, or download origin.
    Deterministic,
    /// Tier 2: a rule crystallised from a decision the user previously approved.
    Learned,
    /// Tier 3: the CLI classifier.
    Llm,
}

/// Exactly §4.2. This is the shape a CLI must return; nothing more is accepted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawPlanEntry {
    pub id: String,
    pub action: Action,
    #[serde(default)]
    pub destination_key: Option<String>,
    #[serde(default)]
    pub rename_to: Option<String>,
    pub confidence: f64,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub suggest_rule: Option<SuggestRule>,
}

/// How a file relates to what is already at its destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateKind {
    /// Byte for byte the same (SHA-256). The only kind that may be sent to the Trash.
    Identical,
    /// Same name, different contents. Not a duplicate at all — moving it saves it as
    /// `name (2)` — but worth saying out loud before it happens.
    SameName,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Duplicate {
    pub kind: DuplicateKind,
    /// Basename of the file it matched. Never a path.
    pub existing_name: String,
    /// When the match is another file in this same batch — a screenshot sitting in
    /// both Downloads and Desktop — the id of the copy that will be kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub same_as_entry: Option<String>,
}

/// What the user decided about a duplicate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateChoice {
    /// Leave it where it is. Nothing is moved.
    Keep,
    /// Send the source copy to the Trash, journalled. Identical files only.
    Trash,
    /// Move it anyway; a name clash gets a ` (2)` suffix.
    MoveAnyway,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanEntry {
    pub id: String,
    /// Basename, mirrored from the manifest so the UI does not need to join.
    pub name: String,
    pub ext: String,
    pub size_bytes: u64,
    pub action: Action,
    pub destination_key: Option<String>,
    pub rename_to: Option<String>,
    pub confidence: f64,
    pub reason: String,
    pub suggest_rule: Option<SuggestRule>,
    pub resolved_by: ResolvedBy,
    /// §1.3: the user's include/exclude toggle. Defaults to included for `move`.
    pub included: bool,
    /// True once the user changes the destination, which suppresses learning from it
    /// (task 12 persists only the rules the user *didn't* override).
    pub overridden: bool,
    /// Set by the duplicate check, and recomputed on apply — the copy of this field
    /// that comes back from the frontend is never trusted.
    #[serde(default)]
    pub duplicate: Option<Duplicate>,
    /// The user's decision about `duplicate`. `None` follows the Duplicates setting.
    #[serde(default)]
    pub on_duplicate: Option<DuplicateChoice>,
}

impl PlanEntry {
    /// Files that will actually be touched on apply.
    pub fn is_actionable(&self) -> bool {
        self.included && self.action == Action::Move && self.destination_key.is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub batch_id: String,
    /// The first scanned folder. Kept because journals written before multi-source
    /// support carry only this; new code reads `sources`.
    pub source: std::path::PathBuf,
    /// Every folder this plan was built from.
    #[serde(default)]
    pub sources: Vec<std::path::PathBuf>,
    pub created_at: DateTime<Utc>,
    pub entries: Vec<PlanEntry>,
    /// Files the scan declined to consider, with reasons (§ honesty about counts).
    pub skipped: Vec<crate::scan::Skipped>,
    pub truncated: bool,
    /// Which prompt template produced the LLM portion, if any (Phase 2, task 10).
    pub prompt_version: Option<String>,
}

impl Plan {
    pub fn new(entries: Vec<PlanEntry>, scan: &Scan) -> Self {
        Plan {
            batch_id: uuid::Uuid::new_v4().to_string(),
            source: scan.sources.first().cloned().unwrap_or_default(),
            sources: scan.sources.clone(),
            created_at: Utc::now(),
            entries,
            skipped: scan.skipped.clone(),
            truncated: scan.truncated,
            prompt_version: None,
        }
    }

    /// The counter that makes the compounding value of learned rules visible (task 12).
    pub fn resolved_without_ai(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| {
                e.action == Action::Move
                    && matches!(
                        e.resolved_by,
                        ResolvedBy::Deterministic | ResolvedBy::Learned
                    )
            })
            .count()
    }

    pub fn actionable(&self) -> impl Iterator<Item = &PlanEntry> {
        self.entries.iter().filter(|e| e.is_actionable())
    }
}

/// CLIs wrap JSON in markdown fences with enthusiasm. Strip them before parsing, and
/// fall back to the outermost bracketed span if there is prose around the payload.
pub fn strip_fences(raw: &str) -> &str {
    let s = raw.trim();
    let s = if let Some(rest) = s.strip_prefix("```") {
        // Drop an optional language tag on the opening fence.
        let rest = rest.split_once('\n').map(|(_, r)| r).unwrap_or(rest);
        rest.rsplit_once("```")
            .map(|(head, _)| head)
            .unwrap_or(rest)
    } else {
        s
    };
    let s = s.trim();

    // Last resort: locate the JSON array or object inside surrounding chatter.
    let open = s.find(['[', '{']);
    let close = s.rfind([']', '}']);
    match (open, close) {
        (Some(o), Some(c)) if c > o => &s[o..=c],
        _ => s,
    }
}

/// Parse a CLI response into raw §4.2 entries.
///
/// Accepts either a bare array or an object with an `entries` / `plan` key, because
/// asking four different CLIs for the same shape and getting it is optimistic.
pub fn parse_raw(raw: &str) -> Result<Vec<RawPlanEntry>> {
    let body = strip_fences(raw);
    if let Ok(v) = serde_json::from_str::<Vec<RawPlanEntry>>(body) {
        return Ok(v);
    }
    #[derive(Deserialize)]
    struct Wrapper {
        #[serde(alias = "plan", alias = "files", alias = "results")]
        entries: Vec<RawPlanEntry>,
    }
    let w: Wrapper = serde_json::from_str(body)?;
    Ok(w.entries)
}

/// §4.3. Reject the **entire batch** if any check fails. Never partially apply a
/// malformed plan, never guess a correction.
pub fn validate(raw: &[RawPlanEntry], scan: &Scan, destination_keys: &[String]) -> Result<()> {
    let mut problems: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();

    for (i, e) in raw.iter().enumerate() {
        let at = format!("entry {i} (id `{}`)", e.id);

        if scan.get(&e.id).is_none() {
            problems.push(format!("{at}: unknown id"));
        }
        if !seen.insert(&e.id) {
            problems.push(format!("{at}: duplicate id"));
        }

        match e.action {
            Action::Move => match &e.destination_key {
                None => problems.push(format!("{at}: action `move` without a destination_key")),
                Some(k) if !destination_keys.iter().any(|d| d == k) => {
                    problems.push(format!("{at}: destination_key `{k}` was not supplied"))
                }
                Some(_) => {}
            },
            // A skip or needs_review may not smuggle in a destination.
            Action::Skip | Action::NeedsReview => {
                if e.destination_key.is_some() {
                    problems.push(format!("{at}: non-move action carries a destination_key"));
                }
            }
        }

        if let Some(name) = &e.rename_to {
            if let Err(why) = validate_rename(name) {
                problems.push(format!("{at}: {why}"));
            }
        }

        if !(0.0..=1.0).contains(&e.confidence) || e.confidence.is_nan() {
            problems.push(format!(
                "{at}: confidence {} is outside [0,1]",
                e.confidence
            ));
        }

        if let Some(rule) = &e.suggest_rule {
            if !destination_keys.contains(&rule.destination_key) {
                problems.push(format!(
                    "{at}: suggest_rule targets unsupplied destination_key `{}`",
                    rule.destination_key
                ));
            }
        }
    }

    if problems.is_empty() {
        Ok(())
    } else {
        Err(Error::Validation(problems))
    }
}

/// §4.3: `rename_to` may not contain `/`, `..`, or a leading `.`.
///
/// Also rejects NUL and the bare `.`/`..` names, which the literal rules miss but
/// which are just as capable of producing a path that is not the one we approved.
pub fn validate_rename(name: &str) -> std::result::Result<(), String> {
    if name.is_empty() {
        return Err("rename_to is empty".into());
    }
    if name.contains('/') {
        return Err("rename_to contains `/`".into());
    }
    if name.contains("..") {
        return Err("rename_to contains `..`".into());
    }
    if name.starts_with('.') {
        return Err("rename_to starts with `.`".into());
    }
    if name.contains('\0') {
        return Err("rename_to contains a NUL byte".into());
    }
    if name.len() > 255 {
        return Err("rename_to exceeds 255 bytes".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    fn scan_of(names: &[&str]) -> (TempDir, Scan) {
        let tmp = TempDir::new();
        for n in names {
            tmp.write(n, b"x");
        }
        let s = crate::scan::scan(tmp.path()).unwrap();
        (tmp, s)
    }

    fn entry(id: &str, action: Action, dest: Option<&str>, confidence: f64) -> RawPlanEntry {
        RawPlanEntry {
            id: id.into(),
            action,
            destination_key: dest.map(str::to_string),
            rename_to: None,
            confidence,
            reason: "because".into(),
            suggest_rule: None,
        }
    }

    fn keys() -> Vec<String> {
        vec!["finance".to_string(), "media".to_string()]
    }

    #[test]
    fn accepts_a_well_formed_plan() {
        let (_t, scan) = scan_of(&["a.pdf", "b.jpg"]);
        let raw = vec![
            entry("f_000", Action::Move, Some("finance"), 0.9),
            entry("f_001", Action::Skip, None, 0.1),
        ];
        assert!(validate(&raw, &scan, &keys()).is_ok());
    }

    #[test]
    fn rejects_unknown_id() {
        let (_t, scan) = scan_of(&["a.pdf"]);
        let raw = vec![entry("f_999", Action::Move, Some("finance"), 0.9)];
        assert!(validate(&raw, &scan, &keys()).is_err());
    }

    #[test]
    fn rejects_duplicate_id() {
        let (_t, scan) = scan_of(&["a.pdf"]);
        let raw = vec![
            entry("f_000", Action::Move, Some("finance"), 0.9),
            entry("f_000", Action::Move, Some("media"), 0.9),
        ];
        assert!(validate(&raw, &scan, &keys()).is_err());
    }

    #[test]
    fn rejects_unsupplied_destination_key() {
        let (_t, scan) = scan_of(&["a.pdf"]);
        let raw = vec![entry("f_000", Action::Move, Some("/etc/passwd"), 0.9)];
        assert!(validate(&raw, &scan, &keys()).is_err());
    }

    #[test]
    fn rejects_move_without_destination() {
        let (_t, scan) = scan_of(&["a.pdf"]);
        let raw = vec![entry("f_000", Action::Move, None, 0.9)];
        assert!(validate(&raw, &scan, &keys()).is_err());
    }

    #[test]
    fn rejects_confidence_out_of_range() {
        let (_t, scan) = scan_of(&["a.pdf"]);
        for c in [-0.1, 1.1, f64::NAN] {
            let raw = vec![entry("f_000", Action::Move, Some("finance"), c)];
            assert!(
                validate(&raw, &scan, &keys()).is_err(),
                "confidence {c} should be rejected"
            );
        }
    }

    #[test]
    fn rejects_dangerous_renames() {
        for bad in ["../escape.pdf", "a/b.pdf", ".hidden", "", "a..b"] {
            assert!(validate_rename(bad).is_err(), "`{bad}` should be rejected");
        }
        for good in ["Invoice 2026.pdf", "a-b_c.txt", "naïve.pdf"] {
            assert!(validate_rename(good).is_ok(), "`{good}` should be accepted");
        }
    }

    #[test]
    fn rejects_suggest_rule_with_unsupplied_key() {
        let (_t, scan) = scan_of(&["a.pdf"]);
        let mut e = entry("f_000", Action::Move, Some("finance"), 0.9);
        e.suggest_rule = Some(SuggestRule {
            match_expr: "ext:pdf".into(),
            destination_key: "elsewhere".into(),
        });
        assert!(validate(&[e], &scan, &keys()).is_err());
    }

    #[test]
    fn reports_every_problem_not_just_the_first() {
        let (_t, scan) = scan_of(&["a.pdf"]);
        let raw = vec![entry("f_404", Action::Move, Some("nope"), 5.0)];
        match validate(&raw, &scan, &keys()) {
            Err(Error::Validation(problems)) => assert_eq!(problems.len(), 3),
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    #[test]
    fn strips_markdown_fences() {
        let fenced = "```json\n[{\"id\":\"f_000\"}]\n```";
        assert_eq!(strip_fences(fenced), "[{\"id\":\"f_000\"}]");
    }

    #[test]
    fn strips_surrounding_prose() {
        let chatty = "Sure! Here is the plan:\n[{\"id\":\"f_000\"}]\nLet me know if that helps.";
        assert_eq!(strip_fences(chatty), "[{\"id\":\"f_000\"}]");
    }

    #[test]
    fn parses_bare_array_and_wrapped_object() {
        let bare =
            r#"[{"id":"f_000","action":"move","destination_key":"finance","confidence":0.9}]"#;
        assert_eq!(parse_raw(bare).unwrap().len(), 1);

        let wrapped = r#"{"entries":[{"id":"f_000","action":"skip","confidence":0.1}]}"#;
        assert_eq!(parse_raw(wrapped).unwrap().len(), 1);
    }

    #[test]
    fn unparseable_output_is_an_error_not_a_guess() {
        assert!(parse_raw("I could not determine a plan, sorry.").is_err());
    }
}
