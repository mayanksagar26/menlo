//! How Menlo remembers: turning approved decisions into learned rules, carefully.
//!
//! A learned rule outranks the built-in extension map, so a careless one does real
//! damage — `ext:pdf → Finance`, learned from one invoice, would send every PDF there.
//! So every rule is checked before it is kept:
//!
//! - **Normalised.** `glob:*.PDF` becomes `ext:pdf`; patterns are lowercased so the
//!   same idea is never stored twice in two spellings.
//! - **Not too broad.** A bare common type (`ext:pdf`, `ext:jpg`) or a pattern with no
//!   distinctive word in it (`glob:IMG_*`) is refused.
//! - **Latest decision wins.** The same pattern approved into a different folder
//!   moves the old rule rather than keeping two that disagree.
//! - **No repeats.** A pattern an existing rule for the same folder already catches
//!   is not added.
//! - **Generalised.** Three or more learned rules for one folder that share a word
//!   collapse into one rule for that word: `Invoice_Aug*`, `Invoice_Sep*` and
//!   `invoice-2026*` become `*invoice*`.

use crate::plan::{Plan, ResolvedBy};
use crate::rules::{glob_match, Matcher, Rule, RuleSet, RuleSource, BUILTIN_CATEGORIES};
use std::collections::{BTreeMap, HashMap, HashSet};

/// Words that say nothing about what a file is.
const GENERIC: &[&str] = &[
    "img", "image", "images", "photo", "pic", "screenshot", "screen", "shot", "scan", "scanned",
    "copy", "final", "draft", "new", "old", "file", "files", "document", "doc", "docs",
    "untitled", "download", "downloads", "export", "edited", "version", "ver", "rev", "the",
    "and", "for", "with", "from", "page", "pages", "part", "temp", "tmp", "test", "jan", "feb",
    "mar", "apr", "may", "jun", "jul", "aug", "sep", "sept", "oct", "nov", "dec", "january",
    "february", "march", "april", "june", "july", "august", "september", "october",
    "november", "december", "mon", "tue", "wed", "thu", "fri", "sat", "sun", "pdf", "jpg",
    "jpeg", "png", "heic", "mov", "mp4", "zip", "dmg", "txt", "docx", "xlsx", "csv", "pptx",
    "whatsapp", "at", "am", "pm", "of", "in", "on", "to",
];

/// Distinctive words in a filename: letters only, three or more of them, lowercased,
/// split at punctuation, digits and camelCase humps, with generic words dropped.
pub fn tokens(name: &str) -> Vec<String> {
    let stem = match name.rsplit_once('.') {
        Some((s, ext)) if !s.is_empty() && ext.len() <= 5 => s,
        _ => name,
    };
    let mut words: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut prev_lower = false;
    for c in stem.chars() {
        if c.is_alphabetic() {
            if c.is_uppercase() && prev_lower && !cur.is_empty() {
                words.push(std::mem::take(&mut cur));
            }
            prev_lower = c.is_lowercase();
            cur.extend(c.to_lowercase());
        } else {
            prev_lower = false;
            if !cur.is_empty() {
                words.push(std::mem::take(&mut cur));
            }
        }
    }
    if !cur.is_empty() {
        words.push(cur);
    }
    let mut seen = HashSet::new();
    words
        .into_iter()
        .filter(|w| w.chars().count() >= 3 && !GENERIC.contains(&w.as_str()))
        .filter(|w| seen.insert(w.clone()))
        .collect()
}

fn is_common_ext(ext: &str) -> bool {
    BUILTIN_CATEGORIES.iter().any(|(_, exts)| exts.contains(&ext))
}

/// The literal text of a glob, with its wildcards taken out.
fn glob_core(pattern: &str) -> String {
    pattern.chars().filter(|c| !matches!(c, '*' | '?' | '[' | ']' | '!')).collect()
}

/// One spelling per idea, or `None` for a pattern that does not parse.
pub fn normalise(expr: &str) -> Option<String> {
    match Matcher::parse(expr).ok()? {
        Matcher::Ext(e) => Some(format!("ext:{e}")),
        Matcher::Origin(o) => Some(format!("origin:{o}")),
        Matcher::Glob(p) => {
            let p = p.to_lowercase();
            // `*.pdf` is an extension rule wearing a glob.
            if let Some(ext) = p.strip_prefix("*.") {
                if !ext.is_empty() && ext.chars().all(|c| c.is_ascii_alphanumeric()) {
                    return Some(format!("ext:{ext}"));
                }
            }
            Some(format!("glob:{p}"))
        }
    }
}

/// Would this rule catch far more than the decision it came from?
pub fn too_broad(expr: &str) -> bool {
    match Matcher::parse(expr) {
        Ok(Matcher::Ext(e)) => is_common_ext(&e),
        Ok(Matcher::Glob(p)) => tokens(&glob_core(&p)).is_empty(),
        Ok(Matcher::Origin(o)) => o.len() < 4,
        Err(_) => true,
    }
}

/// Does an existing rule for `dest` already catch what `expr` would?
fn already_caught(set: &RuleSet, expr: &str, dest: &str) -> bool {
    let Some(rest) = expr.strip_prefix("glob:") else {
        return false;
    };
    // A representative name the new pattern matches: wildcards dropped.
    let example = glob_core(rest);
    set.rules
        .iter()
        .filter(|r| r.enabled && r.destination_key == dest && r.source != RuleSource::Builtin)
        .filter_map(|r| r.match_expr.strip_prefix("glob:"))
        .any(|existing| glob_match(existing, &example))
}

/// Keep one learned rule, if it is worth keeping. Returns every rule added or changed,
/// including a broader rule it may have been folded into.
pub fn remember(set: &mut RuleSet, expr: &str, dest: &str) -> Vec<Rule> {
    let Some(expr) = normalise(expr) else {
        return Vec::new();
    };
    if too_broad(&expr) {
        return Vec::new();
    }
    // The user's own rule for this pattern stands, whatever we learned.
    if set
        .rules
        .iter()
        .any(|r| r.source == RuleSource::User && r.match_expr == expr)
    {
        return Vec::new();
    }
    // Same pattern, approved somewhere else this time: the latest decision wins.
    if let Some(r) = set
        .rules
        .iter_mut()
        .find(|r| r.source == RuleSource::Learned && r.match_expr == expr)
    {
        if r.destination_key == dest {
            return Vec::new();
        }
        r.destination_key = dest.to_string();
        r.created_at = chrono::Utc::now();
        r.hits = 0;
        return vec![r.clone()];
    }
    if already_caught(set, &expr, dest) {
        return Vec::new();
    }
    let rule = Rule::new(&expr, dest, RuleSource::Learned);
    set.rules.push(rule.clone());
    let mut changed = vec![rule];
    if let Some(general) = generalise(set, dest) {
        changed.retain(|r| set.rules.iter().any(|k| k.id == r.id));
        changed.push(general);
    }
    changed
}

/// Fold three or more learned globs for `dest` that share a word into one rule.
fn generalise(set: &mut RuleSet, dest: &str) -> Option<Rule> {
    let globs: Vec<(String, Vec<String>)> = set
        .rules
        .iter()
        .filter(|r| r.source == RuleSource::Learned && r.destination_key == dest)
        .filter_map(|r| {
            let p = r.match_expr.strip_prefix("glob:")?;
            Some((r.id.clone(), tokens(&glob_core(p))))
        })
        .collect();
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, toks) in &globs {
        for t in toks {
            *counts.entry(t.as_str()).or_default() += 1;
        }
    }
    let (word, _) = counts
        .into_iter()
        .filter(|(_, n)| *n >= 3)
        .max_by_key(|(w, n)| (*n, w.len()))?;
    let word = word.to_string();
    let general = format!("glob:*{word}*");
    let folded: HashSet<String> = globs
        .iter()
        .filter(|(_, toks)| toks.contains(&word))
        .map(|(id, _)| id.clone())
        .collect();
    let hits: u64 = set.rules.iter().filter(|r| folded.contains(&r.id)).map(|r| r.hits).sum();
    set.rules.retain(|r| !folded.contains(&r.id));
    if set.rules.iter().any(|r| r.match_expr == general && r.destination_key == dest) {
        return None;
    }
    let mut rule = Rule::new(general, dest, RuleSource::Learned);
    rule.hits = hits;
    set.rules.push(rule.clone());
    Some(rule)
}

/// §1.2 tier 2: crystallise what the user approved.
///
/// Two sources: the rule a model or a folder proposal suggested for a file, and —
/// where the model suggested nothing — a word shared by two or more files the model
/// placed in the same folder, as long as no approved file in another folder has it.
/// Only entries the user left alone count; an overridden destination means the
/// suggestion was wrong.
pub fn learn_from_approved(plan: &Plan, set: &mut RuleSet) -> Vec<Rule> {
    let approved: Vec<_> = plan
        .entries
        .iter()
        .filter(|e| e.is_actionable() && !e.overridden)
        .collect();

    let mut changed: Vec<Rule> = Vec::new();
    let keep = |rules: Vec<Rule>, changed: &mut Vec<Rule>| {
        for r in rules {
            changed.retain(|c| c.id != r.id);
            changed.push(r);
        }
    };

    for e in &approved {
        let (Some(s), Some(dest)) = (&e.suggest_rule, &e.destination_key) else {
            continue;
        };
        // Only ever learn a rule that points where the user actually approved.
        if &s.destination_key == dest {
            keep(remember(set, &s.match_expr, dest), &mut changed);
        }
    }

    // Words shared by model-placed files with no suggestion of their own.
    let mut by_word: HashMap<String, HashMap<String, usize>> = HashMap::new();
    for e in &approved {
        let Some(dest) = &e.destination_key else { continue };
        for t in tokens(&e.name) {
            *by_word.entry(t).or_default().entry(dest.clone()).or_default() +=
                usize::from(e.resolved_by == ResolvedBy::Llm && e.suggest_rule.is_none());
        }
    }
    let mut words: Vec<(String, String)> = by_word
        .into_iter()
        .filter(|(_, dests)| dests.len() == 1)
        .filter_map(|(w, dests)| {
            let (d, n) = dests.into_iter().next()?;
            (n >= 2).then_some((w, d))
        })
        .collect();
    words.sort();
    for (word, dest) in words {
        keep(remember(set, &format!("glob:*{word}*"), &dest), &mut changed);
    }

    // Report only what is still in the set: a rule may have been folded away.
    changed.retain(|r| set.rules.iter().any(|k| k.id == r.id));
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_find_the_words_that_matter() {
        assert_eq!(tokens("Invoice_Aug2026.pdf"), vec!["invoice"]);
        assert_eq!(tokens("HDFCBankStatement-03.pdf"), vec!["hdfcbank", "statement"]);
        assert!(tokens("IMG_4821.jpg").is_empty());
        assert!(tokens("Screenshot 2026-11-03 at 10.22.01.png").is_empty());
        assert_eq!(tokens("lease agreement (final).docx"), vec!["lease", "agreement"]);
    }

    #[test]
    fn normalises_and_refuses_broad_rules() {
        assert_eq!(normalise("glob:*.PDF").as_deref(), Some("ext:pdf"));
        assert_eq!(normalise("glob:Invoice_*").as_deref(), Some("glob:invoice_*"));
        assert!(too_broad("ext:pdf"));
        assert!(!too_broad("ext:dwg"));
        assert!(too_broad("glob:IMG_*"));
        assert!(too_broad("glob:*"));
        assert!(!too_broad("glob:invoice_*"));
    }

    #[test]
    fn broad_rules_are_never_kept() {
        let mut set = RuleSet::default();
        assert!(remember(&mut set, "ext:pdf", "finance").is_empty());
        assert!(remember(&mut set, "glob:*.jpg", "finance").is_empty());
        assert!(set.rules.is_empty());
    }

    #[test]
    fn the_latest_decision_moves_the_rule() {
        let mut set = RuleSet::default();
        remember(&mut set, "glob:Lease*", "home");
        remember(&mut set, "glob:lease*", "contracts");
        assert_eq!(set.rules.len(), 1);
        assert_eq!(set.rules[0].destination_key, "contracts");
    }

    #[test]
    fn a_pattern_already_caught_is_not_repeated() {
        let mut set = RuleSet::default();
        remember(&mut set, "glob:*invoice*", "finance");
        assert!(remember(&mut set, "glob:Invoice_Aug*", "finance").is_empty());
        assert_eq!(set.rules.len(), 1);
        // A different folder is a different decision.
        assert_eq!(remember(&mut set, "glob:Invoice_Aug*", "work").len(), 1);
    }

    #[test]
    fn three_alike_fold_into_one() {
        let mut set = RuleSet::default();
        remember(&mut set, "glob:Invoice_Aug*", "finance");
        remember(&mut set, "glob:Invoice_Sep*", "finance");
        let last = remember(&mut set, "glob:invoice-2026*", "finance");
        assert_eq!(set.rules.len(), 1);
        assert_eq!(set.rules[0].match_expr, "glob:*invoice*");
        assert!(last.iter().any(|r| r.match_expr == "glob:*invoice*"));
    }
}
