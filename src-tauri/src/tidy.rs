//! Tidying a folder in place, and proposing new subfolders.
//!
//! A folder can be both a source and a destination — Documents → Documents. Its loose
//! files are then the ones to sort: into one of its subfolders where one fits, into
//! another destination where the type says so, and otherwise left exactly where they
//! are. A file is never "moved" into the folder it is already in.
//!
//! What nothing fits yet is grouped — by a word the names share, by a model's
//! suggestion, or (for a folder tidied in place) by file type — and offered as new
//! subfolders. Nothing is created until the user accepts one in the preview.

use crate::config::{Config, Destination};
use crate::memory::tokens;
use crate::plan::{Action, FolderProposal, Plan, ResolvedBy};
use crate::rules::BUILTIN_CATEGORIES;
use crate::scan::Scan;
use std::collections::{BTreeMap, HashSet};
use std::path::Path;

/// Files a group needs before it is worth a folder.
const MIN_GROUP: usize = 3;
/// A model's suggestion needs fewer: it has read the names, not just counted words.
const MIN_SUGGESTED: usize = 2;
const MAX_PROPOSALS: usize = 6;

/// The destination a source folder *is*, when it is being tidied in place.
pub fn in_place<'a>(config: &'a Config, source: &Path) -> Option<&'a Destination> {
    config.destinations.iter().find(|d| d.path == source)
}

/// Turn every "move into the folder it is already in" into "stays where it is".
pub fn settle_in_place(plan: &mut Plan, scan: &Scan, config: &Config) {
    for e in plan.entries.iter_mut() {
        let Some(item) = scan.get(&e.id) else { continue };
        let Some(home) = in_place(config, &item.source) else {
            continue;
        };
        if e.action == Action::Move && e.destination_key.as_deref() == Some(home.key.as_str()) {
            e.action = Action::NeedsReview;
            e.destination_key = None;
            e.included = false;
            e.confidence = 0.0;
            e.rename_to = None;
            e.suggest_rule = None;
            e.resolved_by = ResolvedBy::Deterministic;
            e.reason = format!("Already in {}; none of its folders fit yet.", home.label);
        } else if e.action == Action::NeedsReview && e.reason == "No rule matched." {
            e.reason = format!("Loose in {}; none of its folders fit yet.", home.label);
        }
    }
}

fn category_of(ext: &str) -> Option<&'static str> {
    BUILTIN_CATEGORIES
        .iter()
        .find(|(_, exts)| exts.contains(&ext))
        .map(|(c, _)| *c)
}

fn title(word: &str) -> String {
    let mut c = word.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// A file a new folder could be made for, and the folder it would go inside.
struct Candidate<'a> {
    id: &'a str,
    name: &'a str,
    ext: &'a str,
    parent: &'a Destination,
    /// Loose in its own folder, rather than headed into a folder with no subfolder fit.
    loose: bool,
    suggested: Option<String>,
}

/// New subfolders worth making, for files nothing fits yet.
///
/// Two kinds of file are considered: one loose in a folder being tidied in place that
/// nothing placed, and one headed into a top-level folder with no subfolder for it.
pub fn propose(plan: &Plan, scan: &Scan, config: &Config) -> Vec<FolderProposal> {
    let mut pool: Vec<Candidate> = Vec::new();
    for e in &plan.entries {
        let Some(item) = scan.get(&e.id) else { continue };
        let (parent, loose) = match (e.action, e.destination_key.as_deref()) {
            (Action::NeedsReview, _) => match in_place(config, &item.source) {
                Some(home) if !home.is_subfolder() => (home, true),
                _ => continue,
            },
            (Action::Move, Some(k)) => match config.destination(k) {
                Some(d) if !d.is_subfolder() && e.duplicate.is_none() => (d, false),
                _ => continue,
            },
            _ => continue,
        };
        pool.push(Candidate {
            id: &e.id,
            name: &e.name,
            ext: &e.ext,
            parent,
            loose,
            suggested: e.suggest_folder.clone(),
        });
    }

    let mut taken: HashSet<&str> = HashSet::new();
    let mut out: Vec<FolderProposal> = Vec::new();

    // Names already used inside each parent, lowercased, so nothing is proposed twice.
    let used = |parent: &Destination, out: &[FolderProposal], name: &str| -> bool {
        let n = name.to_lowercase();
        let n_stem = n.trim_end_matches('s');
        config.subfolders_of(&parent.key).any(|s| {
            let l = s.label.to_lowercase();
            l == n || l.trim_end_matches('s') == n_stem
        }) || out
            .iter()
            .any(|p| p.parent_key == parent.key && p.name.to_lowercase() == n)
    };

    // 1. A model's suggestions.
    let mut suggested: BTreeMap<(String, String), Vec<&Candidate>> = BTreeMap::new();
    for c in &pool {
        if let Some(name) = &c.suggested {
            let name = name.trim().to_string();
            if crate::plan::validate_rename(&name).is_ok() {
                suggested.entry((c.parent.key.clone(), name)).or_default().push(c);
            }
        }
    }
    for ((_, name), group) in suggested {
        let parent = group[0].parent;
        if group.len() < MIN_SUGGESTED || used(parent, &out, &name) {
            continue;
        }
        // A rule only if the group shares a word the folder is named for.
        let word = tokens(&name).into_iter().find(|w| group.iter().all(|c| tokens(c.name).contains(w)));
        out.push(FolderProposal {
            parent_key: parent.key.clone(),
            name: name.clone(),
            entry_ids: group.iter().map(|c| c.id.to_string()).collect(),
            reason: format!("{} files the model thinks belong together.", group.len()),
            match_expr: word.map(|w| format!("glob:*{w}*")),
        });
        taken.extend(group.iter().map(|c| c.id));
    }

    // 2. A word the names share, biggest group first.
    loop {
        let mut counts: BTreeMap<(&str, String), Vec<&Candidate>> = BTreeMap::new();
        for c in pool.iter().filter(|c| !taken.contains(c.id)) {
            let parent_words = tokens(&c.parent.label);
            for t in tokens(c.name) {
                if !parent_words.contains(&t) {
                    counts.entry((c.parent.key.as_str(), t)).or_default().push(c);
                }
            }
        }
        let best = counts
            .into_iter()
            .filter(|((_, w), g)| g.len() >= MIN_GROUP && !used(g[0].parent, &out, &title(w)))
            .max_by_key(|((_, w), g)| (g.len(), w.len()));
        let Some(((_, word), group)) = best else { break };
        let parent = group[0].parent;
        out.push(FolderProposal {
            parent_key: parent.key.clone(),
            name: title(&word),
            entry_ids: group.iter().map(|c| c.id.to_string()).collect(),
            reason: format!("{} files have “{word}” in the name.", group.len()),
            match_expr: Some(format!("glob:*{word}*")),
        });
        taken.extend(group.iter().map(|c| c.id));
    }

    // 3. Loose files of one kind that do not match their folder's own kind — a pile of
    //    images in Documents. No rule: "every image goes to Documents/Images" would
    //    pull photos out of Downloads that belong in Pictures.
    let mut kinds: BTreeMap<(&str, &str), Vec<&Candidate>> = BTreeMap::new();
    for c in pool.iter().filter(|c| c.loose && !taken.contains(c.id)) {
        if let Some(cat) = category_of(c.ext) {
            if cat != c.parent.key {
                kinds.entry((c.parent.key.as_str(), cat)).or_default().push(c);
            }
        }
    }
    for ((_, cat), group) in kinds {
        let parent = group[0].parent;
        let name = title(cat);
        if group.len() < MIN_GROUP || used(parent, &out, &name) {
            continue;
        }
        out.push(FolderProposal {
            parent_key: parent.key.clone(),
            name,
            entry_ids: group.iter().map(|c| c.id.to_string()).collect(),
            reason: format!("{} {cat} files loose in {}.", group.len(), parent.label),
            match_expr: None,
        });
    }

    out.sort_by(|a, b| b.entry_ids.len().cmp(&a.entry_ids.len()));
    out.truncate(MAX_PROPOSALS);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Destination;
    use crate::rules::{build_plan, RuleSet};
    use crate::testutil::TempDir;

    /// Documents, tidied in place, with a Contracts subfolder.
    fn documents(names: &[&str]) -> (TempDir, Scan, Config) {
        let tmp = TempDir::new();
        let docs = tmp.path().join("Documents");
        std::fs::create_dir_all(docs.join("Contracts")).unwrap();
        for n in names {
            std::fs::write(docs.join(n), b"x").unwrap();
        }
        let docs = docs.canonicalize().unwrap();
        let mut config = Config::default();
        config.sources = vec![docs.clone()];
        config.destinations = vec![Destination {
            key: "documents".into(),
            label: "Documents".into(),
            path: docs.clone(),
            brief: String::new(),
            parent: None,
        }];
        assert!(config.sync_subfolders("documents"));
        config.validate().unwrap();
        let scan = crate::scan::scan(&docs).unwrap();
        (tmp, scan, config)
    }

    #[test]
    fn a_file_is_never_moved_into_the_folder_it_is_in() {
        let (_t, scan, config) = documents(&["notes.pdf", "Contracts lease.pdf"]);
        let mut plan = build_plan(&scan, &config, &RuleSet::default());
        settle_in_place(&mut plan, &scan, &config);
        let by = |n: &str| plan.entries.iter().find(|e| e.name == n).unwrap();
        // Named like a subfolder → into it.
        assert_eq!(by("Contracts lease.pdf").destination_key.as_deref(), Some("documents-contracts"));
        // Nothing fits → stays put, not "moved" onto itself.
        assert_eq!(by("notes.pdf").action, Action::NeedsReview);
        assert!(by("notes.pdf").destination_key.is_none());
    }

    #[test]
    fn loose_files_that_share_a_word_get_a_folder_proposed() {
        let (_t, scan, config) = documents(&[
            "Invoice_Aug.pdf",
            "invoice-sep.pdf",
            "Swiggy Invoice Oct.pdf",
            "random.pdf",
            "a.png",
            "b.png",
            "c.jpg",
        ]);
        let mut plan = build_plan(&scan, &config, &RuleSet::default());
        settle_in_place(&mut plan, &scan, &config);
        let props = propose(&plan, &scan, &config);
        let invoices = props.iter().find(|p| p.name == "Invoice").expect("an Invoice folder");
        assert_eq!(invoices.entry_ids.len(), 3);
        assert_eq!(invoices.match_expr.as_deref(), Some("glob:*invoice*"));
        let images = props.iter().find(|p| p.name == "Images").expect("an Images folder");
        assert_eq!(images.entry_ids.len(), 3);
        assert!(images.match_expr.is_none());
        assert!(!props.iter().any(|p| p.name == "Contracts"));
    }
}
