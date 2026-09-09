//! Tier 1 (deterministic) and tier 2 (learned) of §1.2's routing.
//!
//! Between them these should resolve 70–80% of a typical Downloads folder with zero
//! inference. Whatever is left is the only thing Phase 2 sends to a CLI.
//!
//! Rule precedence, highest first:
//!   1. user rules    — an explicit instruction always wins
//!   2. learned rules — crystallised from a decision the user approved
//!   3. built-in      — the extension map, which only fires for well-known keys
//!
//! Within a tier, the most specific matcher wins (see [`Matcher::specificity`]), so
//! `glob:Invoice_*.pdf` beats `ext:pdf` regardless of insertion order.

use crate::config::{Config, Destination};
use crate::error::{Error, Result};
use crate::plan::{Action, Plan, PlanEntry, ResolvedBy};
use crate::scan::{Scan, ScanItem};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleSource {
    Builtin,
    User,
    /// §1.2 tier 2: written by the app when the user approves an LLM decision.
    Learned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    /// `ext:pdf`, `glob:Invoice_*.pdf`, `origin:billing.example.com`.
    #[serde(rename = "match")]
    pub match_expr: String,
    pub destination_key: String,
    pub source: RuleSource,
    pub created_at: DateTime<Utc>,
    /// How many files this rule has filed. Shown in Settings so a rule that never
    /// fires is easy to spot and delete.
    #[serde(default)]
    pub hits: u64,
    #[serde(default = "yes")]
    pub enabled: bool,
}

fn yes() -> bool {
    true
}

impl Rule {
    pub fn new(
        match_expr: impl Into<String>,
        destination_key: impl Into<String>,
        source: RuleSource,
    ) -> Self {
        Rule {
            id: uuid::Uuid::new_v4().to_string(),
            match_expr: match_expr.into(),
            destination_key: destination_key.into(),
            source,
            created_at: Utc::now(),
            hits: 0,
            enabled: true,
        }
    }
}

/// A parsed matcher. Parsing up front means an unrecognised prefix is a config error
/// rather than a rule that silently never matches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Matcher {
    Ext(String),
    Glob(String),
    /// Substring match against `kMDItemWhereFroms`. Inert until Phase 2 populates it.
    Origin(String),
}

impl Matcher {
    pub fn parse(expr: &str) -> Result<Matcher> {
        let (kind, value) = expr
            .split_once(':')
            .ok_or_else(|| Error::config(format!("rule `{expr}` is missing a `kind:` prefix")))?;
        let value = value.trim();
        if value.is_empty() {
            return Err(Error::config(format!("rule `{expr}` has an empty pattern")));
        }
        match kind {
            "ext" => Ok(Matcher::Ext(
                value.trim_start_matches('.').to_ascii_lowercase(),
            )),
            "glob" => Ok(Matcher::Glob(value.to_string())),
            "origin" => Ok(Matcher::Origin(value.to_ascii_lowercase())),
            // `regex:` is in §1.2 but needs the `regex` crate, which is not in the
            // approved dependency set. Rejected loudly rather than silently ignored.
            "regex" => Err(Error::config(
                "regex rules are not supported yet (needs the `regex` crate)".to_string(),
            )),
            other => Err(Error::config(format!(
                "unknown rule kind `{other}` in `{expr}`"
            ))),
        }
    }

    fn matches(&self, item: &ScanItem) -> bool {
        match self {
            Matcher::Ext(e) => item.entry.ext == *e,
            Matcher::Glob(p) => glob_match(p, &item.entry.name),
            Matcher::Origin(needle) => item
                .entry
                .origin
                .as_deref()
                .map(|o| o.to_ascii_lowercase().contains(needle))
                .unwrap_or(false),
        }
    }

    /// Higher wins. An origin match is the strongest signal we have (§4.1), a glob is
    /// narrower than a bare extension, and length breaks ties between two globs.
    fn specificity(&self) -> usize {
        match self {
            Matcher::Origin(v) => 3000 + v.len(),
            Matcher::Glob(v) => 2000 + v.len(),
            Matcher::Ext(_) => 1000,
        }
    }

    fn human(&self) -> String {
        match self {
            Matcher::Ext(e) => format!("a .{e} file"),
            Matcher::Glob(p) => format!("the name matches {p}"),
            Matcher::Origin(o) => format!("it was downloaded from {o}"),
        }
    }
}

/// Shell-style glob over a basename: `*` (any run), `?` (one char), `[abc]` / `[a-z]`
/// / `[!abc]`. Case-insensitive, because macOS filenames are.
///
/// Iterative backtracking rather than recursion, so a pathological pattern like
/// `*a*a*a*a*b` cannot blow the stack on user input.
pub fn glob_match(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let n: Vec<char> = name.to_lowercase().chars().collect();

    let (mut pi, mut ni) = (0usize, 0usize);
    // Position to resume from if the current `*` expansion turns out to be wrong.
    let (mut star, mut resume) = (usize::MAX, 0usize);

    while ni < n.len() {
        if pi < p.len() && p[pi] == '*' {
            star = pi;
            pi += 1;
            resume = ni;
        } else if pi < p.len() && char_class_matches(&p, &mut pi, n[ni]) {
            ni += 1;
        } else if star != usize::MAX {
            // Backtrack: let the last `*` swallow one more character.
            pi = star + 1;
            resume += 1;
            ni = resume;
        } else {
            return false;
        }
    }

    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

/// Matches one pattern element against `c`, advancing `pi` past it. Handles `?`,
/// `[...]` classes and literals.
fn char_class_matches(p: &[char], pi: &mut usize, c: char) -> bool {
    match p[*pi] {
        '?' => {
            *pi += 1;
            true
        }
        '[' => {
            let start = *pi;
            let mut i = *pi + 1;
            let negated = i < p.len() && (p[i] == '!' || p[i] == '^');
            if negated {
                i += 1;
            }
            let mut hit = false;
            let mut closed = false;
            while i < p.len() {
                if p[i] == ']' && i > start + 1 + usize::from(negated) {
                    closed = true;
                    break;
                }
                // Range, e.g. a-z. The `]` guard above keeps `[a-]` literal.
                if i + 2 < p.len() && p[i + 1] == '-' && p[i + 2] != ']' {
                    if p[i] <= c && c <= p[i + 2] {
                        hit = true;
                    }
                    i += 3;
                } else {
                    if p[i] == c {
                        hit = true;
                    }
                    i += 1;
                }
            }
            if !closed {
                // Unterminated class: treat the `[` as a literal rather than failing.
                *pi = start + 1;
                return c == '[';
            }
            *pi = i + 1;
            hit != negated
        }
        lit => {
            *pi += 1;
            lit == c
        }
    }
}

/// The built-in extension map.
///
/// It maps an extension to a *category key*, not to a path — Menlo cannot know what the
/// user called their folders. A built-in rule fires only when the user has a destination
/// whose key equals the category, which is why the Setup screen offers these keys as
/// one-tap suggestions.
pub const BUILTIN_CATEGORIES: &[(&str, &[&str])] = &[
    (
        "documents",
        &[
            "pdf", "doc", "docx", "odt", "rtf", "txt", "md", "pages", "tex",
        ],
    ),
    (
        "spreadsheets",
        &["xls", "xlsx", "csv", "tsv", "ods", "numbers"],
    ),
    ("presentations", &["ppt", "pptx", "odp", "key"]),
    (
        "images",
        &[
            "jpg", "jpeg", "png", "gif", "webp", "heic", "tiff", "tif", "bmp", "svg", "avif",
        ],
    ),
    (
        "video",
        &[
            "mp4", "mov", "mkv", "avi", "webm", "m4v", "mpg", "mpeg", "wmv",
        ],
    ),
    (
        "audio",
        &["mp3", "m4a", "wav", "flac", "aac", "ogg", "aiff", "aif"],
    ),
    (
        "archives",
        &["zip", "tar", "gz", "tgz", "bz2", "xz", "7z", "rar", "zst"],
    ),
    (
        "code",
        &[
            "rs", "ts", "tsx", "js", "jsx", "mjs", "py", "go", "rb", "java", "kt", "swift", "c",
            "h", "cpp", "cs", "php", "lua", "sh", "zsh", "html", "css", "scss", "vue", "svelte",
            "json", "yaml", "yml", "toml", "xml", "sql", "ipynb",
        ],
    ),
    (
        "design",
        &["sketch", "fig", "psd", "ai", "xd", "afdesign", "afphoto"],
    ),
    ("ebooks", &["epub", "mobi", "azw3", "djvu"]),
    ("fonts", &["otf", "ttf", "woff", "woff2"]),
    ("installers", &["dmg", "pkg", "mpkg", "iso", "app"]),
];

/// Built-in rules restricted to the destination keys the user actually created.
pub fn builtin_rules(destinations: &[Destination]) -> Vec<Rule> {
    let mut rules = Vec::new();
    for (category, exts) in BUILTIN_CATEGORIES {
        if !destinations.iter().any(|d| d.key == *category) {
            continue;
        }
        for ext in *exts {
            rules.push(Rule {
                id: format!("builtin:{category}:{ext}"),
                match_expr: format!("ext:{ext}"),
                destination_key: (*category).to_string(),
                source: RuleSource::Builtin,
                created_at: DateTime::<Utc>::from(std::time::UNIX_EPOCH),
                hits: 0,
                enabled: true,
            });
        }
    }
    rules
}

/// User + learned rules, persisted alongside the config.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuleSet {
    #[serde(default)]
    pub rules: Vec<Rule>,
}

impl RuleSet {
    pub fn learned(&self) -> impl Iterator<Item = &Rule> {
        self.rules
            .iter()
            .filter(|r| r.source == RuleSource::Learned)
    }

    /// Add a rule, ignoring an exact duplicate. Returns whether anything was added.
    pub fn insert(&mut self, rule: Rule) -> bool {
        let dup = self
            .rules
            .iter()
            .any(|r| r.match_expr == rule.match_expr && r.destination_key == rule.destination_key);
        if dup {
            return false;
        }
        self.rules.push(rule);
        true
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.rules.len();
        self.rules.retain(|r| r.id != id);
        self.rules.len() != before
    }
}

fn rules_path() -> Result<std::path::PathBuf> {
    Ok(crate::config::app_dir()?.join("rules.json"))
}

pub fn load_rules() -> Result<RuleSet> {
    let path = rules_path()?;
    match std::fs::read_to_string(&path) {
        Ok(s) => Ok(serde_json::from_str(&s)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(RuleSet::default()),
        Err(e) => Err(Error::io(&path, e)),
    }
}

pub fn save_rules(set: &RuleSet) -> Result<()> {
    crate::config::write_atomic(
        &rules_path()?,
        serde_json::to_string_pretty(set)?.as_bytes(),
    )
}

/// A rule paired with its parsed matcher and tier weight, ready to compete.
struct Candidate<'a> {
    rule: &'a Rule,
    matcher: Matcher,
}

fn tier_weight(source: RuleSource) -> usize {
    match source {
        RuleSource::User => 3,
        RuleSource::Learned => 2,
        RuleSource::Builtin => 1,
    }
}

/// Run tiers 1 and 2 over a scan and produce a full [`Plan`].
///
/// Every scanned file gets an entry. Unmatched files become `needs_review` with
/// confidence 0 — they are exactly the residue Phase 2's LLM pass will pick up.
pub fn build_plan(scan: &Scan, config: &Config, ruleset: &RuleSet) -> Plan {
    let mut all: Vec<Rule> = builtin_rules(&config.destinations);
    all.extend(ruleset.rules.iter().filter(|r| r.enabled).cloned());

    // Drop rules pointing at destinations that no longer exist, and rules whose
    // matcher does not parse, rather than failing the whole run over one bad rule.
    let candidates: Vec<Candidate> = all
        .iter()
        .filter(|r| config.destination(&r.destination_key).is_some())
        .filter_map(|r| {
            Matcher::parse(&r.match_expr)
                .ok()
                .map(|matcher| Candidate { rule: r, matcher })
        })
        .collect();

    let entries = scan
        .items
        .iter()
        .map(|item| {
            let best = candidates
                .iter()
                .filter(|c| c.matcher.matches(item))
                .max_by_key(|c| (tier_weight(c.rule.source), c.matcher.specificity()));

            match best {
                Some(c) => {
                    let resolved_by = match c.rule.source {
                        RuleSource::Learned => ResolvedBy::Learned,
                        _ => ResolvedBy::Deterministic,
                    };
                    let label = config
                        .destination(&c.rule.destination_key)
                        .map(|d| d.label.clone())
                        .unwrap_or_else(|| c.rule.destination_key.clone());
                    let reason = match c.rule.source {
                        RuleSource::Learned => {
                            format!("Learned from an earlier approval: {}.", c.matcher.human())
                        }
                        RuleSource::User => format!("Your rule: {}.", c.matcher.human()),
                        RuleSource::Builtin => {
                            format!("{} → {label}.", capitalise(&c.matcher.human()))
                        }
                    };
                    PlanEntry {
                        id: item.entry.id.clone(),
                        name: item.entry.name.clone(),
                        ext: item.entry.ext.clone(),
                        size_bytes: item.entry.size_bytes,
                        action: Action::Move,
                        destination_key: Some(c.rule.destination_key.clone()),
                        rename_to: None,
                        confidence: 1.0,
                        reason,
                        suggest_rule: None,
                        resolved_by,
                        included: true,
                        overridden: false,
                    }
                }
                None => PlanEntry {
                    id: item.entry.id.clone(),
                    name: item.entry.name.clone(),
                    ext: item.entry.ext.clone(),
                    size_bytes: item.entry.size_bytes,
                    action: Action::NeedsReview,
                    destination_key: None,
                    rename_to: None,
                    confidence: 0.0,
                    reason: "No rule matched.".to_string(),
                    suggest_rule: None,
                    resolved_by: ResolvedBy::Deterministic,
                    included: false,
                    overridden: false,
                },
            }
        })
        .collect();

    Plan::new(config.source.clone().unwrap_or_default(), entries, scan)
}

fn capitalise(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// §1.2 tier 2, the core product loop: crystallise the approved decisions.
///
/// Only entries the user left alone are learned from — an overridden destination means
/// the suggestion was wrong, and learning it would bake in the mistake.
pub fn learn_from_approved(plan: &Plan, ruleset: &mut RuleSet) -> Vec<Rule> {
    let mut added = Vec::new();
    for e in plan.entries.iter() {
        if !e.is_actionable() || e.overridden {
            continue;
        }
        let Some(suggestion) = &e.suggest_rule else {
            continue;
        };
        // A suggestion that does not parse is a suggestion we refuse to persist.
        if Matcher::parse(&suggestion.match_expr).is_err() {
            continue;
        }
        // Only ever learn a rule that points where the user actually approved.
        if Some(&suggestion.destination_key) != e.destination_key.as_ref() {
            continue;
        }
        let rule = Rule::new(
            &suggestion.match_expr,
            &suggestion.destination_key,
            RuleSource::Learned,
        );
        if ruleset.insert(rule.clone()) {
            added.push(rule);
        }
    }
    added
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Settings;
    use crate::plan::SuggestRule;
    use crate::testutil::TempDir;
    use std::path::PathBuf;

    #[test]
    fn glob_basics() {
        assert!(glob_match("Invoice_*.pdf", "Invoice_Aug2026.pdf"));
        assert!(!glob_match("Invoice_*.pdf", "Receipt_Aug2026.pdf"));
        assert!(glob_match("*.pdf", "a.pdf"));
        assert!(glob_match("IMG_????.jpg", "IMG_0042.jpg"));
        assert!(!glob_match("IMG_????.jpg", "IMG_42.jpg"));
        assert!(glob_match("*", "anything"));
    }

    #[test]
    fn glob_is_case_insensitive_like_the_filesystem() {
        assert!(glob_match("*.PDF", "report.pdf"));
        assert!(glob_match("invoice*", "Invoice_2026.pdf"));
    }

    #[test]
    fn glob_character_classes() {
        assert!(glob_match("[abc]at.txt", "bat.txt"));
        assert!(!glob_match("[abc]at.txt", "hat.txt"));
        assert!(glob_match("[a-z]*.txt", "quarterly.txt"));
        assert!(glob_match("[!0-9]*.txt", "report.txt"));
        assert!(!glob_match("[!0-9]*.txt", "2026.txt"));
    }

    #[test]
    fn glob_backtracks_without_blowing_up() {
        // The pathological case that naive recursive matchers choke on.
        assert!(!glob_match("*a*a*a*a*a*a*b", &"a".repeat(64)));
        assert!(glob_match("*a*b", "aaaaab"));
    }

    #[test]
    fn matcher_parsing_rejects_junk_and_regex() {
        assert!(Matcher::parse("ext:pdf").is_ok());
        assert!(Matcher::parse("glob:*.pdf").is_ok());
        assert!(Matcher::parse("no-prefix").is_err());
        assert!(Matcher::parse("ext:").is_err());
        assert!(Matcher::parse("wat:x").is_err());
        // Documented gap, not a silent no-op.
        assert!(Matcher::parse(r"regex:^IMG_\d+").is_err());
    }

    #[test]
    fn ext_matcher_normalises_a_leading_dot() {
        assert_eq!(
            Matcher::parse("ext:.PDF").unwrap(),
            Matcher::Ext("pdf".into())
        );
    }

    fn fixture(names: &[&str], dest_keys: &[&str]) -> (TempDir, Scan, Config) {
        let tmp = TempDir::new();
        for n in names {
            tmp.write(n, b"x");
        }
        let scan = crate::scan::scan(tmp.path()).unwrap();
        let config = Config {
            source: Some(tmp.path().to_path_buf()),
            destinations: dest_keys
                .iter()
                .map(|k| Destination {
                    key: (*k).to_string(),
                    label: capitalise(k),
                    path: PathBuf::from(format!("/tmp/{k}")),
                    brief: String::new(),
                })
                .collect(),
            settings: Settings::default(),
        };
        (tmp, scan, config)
    }

    #[test]
    fn builtin_map_fires_only_for_keys_the_user_created() {
        let (_t, scan, config) = fixture(&["a.pdf", "b.jpg"], &["documents"]);
        let plan = build_plan(&scan, &config, &RuleSet::default());

        let pdf = plan.entries.iter().find(|e| e.name == "a.pdf").unwrap();
        assert_eq!(pdf.action, Action::Move);
        assert_eq!(pdf.destination_key.as_deref(), Some("documents"));

        // No `images` destination exists, so the jpg has nowhere to go.
        let jpg = plan.entries.iter().find(|e| e.name == "b.jpg").unwrap();
        assert_eq!(jpg.action, Action::NeedsReview);
        assert!(
            !jpg.included,
            "needs_review must not be included by default"
        );
    }

    #[test]
    fn user_rules_beat_builtins() {
        let (_t, scan, config) = fixture(&["Invoice_Aug.pdf"], &["documents", "finance"]);
        let mut set = RuleSet::default();
        set.insert(Rule::new("glob:Invoice_*.pdf", "finance", RuleSource::User));

        let plan = build_plan(&scan, &config, &set);
        assert_eq!(plan.entries[0].destination_key.as_deref(), Some("finance"));
    }

    #[test]
    fn more_specific_matcher_wins_within_a_tier() {
        let (_t, scan, config) = fixture(&["Invoice_Aug.pdf"], &["documents", "finance"]);
        let mut set = RuleSet::default();
        // Inserted least-specific-last on purpose: order must not decide this.
        set.insert(Rule::new("glob:Invoice_*.pdf", "finance", RuleSource::User));
        set.insert(Rule::new("ext:pdf", "documents", RuleSource::User));

        let plan = build_plan(&scan, &config, &set);
        assert_eq!(plan.entries[0].destination_key.as_deref(), Some("finance"));
    }

    #[test]
    fn rules_for_deleted_destinations_are_ignored() {
        let (_t, scan, config) = fixture(&["a.pdf"], &["documents"]);
        let mut set = RuleSet::default();
        set.insert(Rule::new(
            "ext:pdf",
            "a-key-that-no-longer-exists",
            RuleSource::Learned,
        ));

        // Falls through to the builtin rather than producing a dangling destination.
        let plan = build_plan(&scan, &config, &set);
        assert_eq!(
            plan.entries[0].destination_key.as_deref(),
            Some("documents")
        );
    }

    #[test]
    fn a_malformed_rule_does_not_sink_the_run() {
        let (_t, scan, config) = fixture(&["a.pdf"], &["documents"]);
        let mut set = RuleSet::default();
        set.rules.push(Rule::new(
            "this is not a matcher",
            "documents",
            RuleSource::User,
        ));

        let plan = build_plan(&scan, &config, &set);
        assert_eq!(
            plan.entries[0].destination_key.as_deref(),
            Some("documents")
        );
    }

    #[test]
    fn disabled_rules_do_not_fire() {
        let (_t, scan, config) = fixture(&["a.pdf"], &["documents", "finance"]);
        let mut set = RuleSet::default();
        let mut r = Rule::new("ext:pdf", "finance", RuleSource::User);
        r.enabled = false;
        set.insert(r);

        let plan = build_plan(&scan, &config, &set);
        assert_eq!(
            plan.entries[0].destination_key.as_deref(),
            Some("documents")
        );
    }

    #[test]
    fn learns_only_from_decisions_the_user_left_alone() {
        let (_t, scan, config) = fixture(&["a.pdf", "b.pdf", "c.pdf"], &["finance"]);
        let mut plan = build_plan(&scan, &config, &RuleSet::default());

        // a: approved with a suggestion → learn it.
        plan.entries[0].action = Action::Move;
        plan.entries[0].destination_key = Some("finance".into());
        plan.entries[0].included = true;
        plan.entries[0].suggest_rule = Some(SuggestRule {
            match_expr: "glob:a*.pdf".into(),
            destination_key: "finance".into(),
        });

        // b: user overrode the destination → do not learn.
        plan.entries[1].action = Action::Move;
        plan.entries[1].destination_key = Some("finance".into());
        plan.entries[1].included = true;
        plan.entries[1].overridden = true;
        plan.entries[1].suggest_rule = Some(SuggestRule {
            match_expr: "glob:b*.pdf".into(),
            destination_key: "finance".into(),
        });

        // c: excluded from the batch → do not learn.
        plan.entries[2].action = Action::Move;
        plan.entries[2].destination_key = Some("finance".into());
        plan.entries[2].included = false;
        plan.entries[2].suggest_rule = Some(SuggestRule {
            match_expr: "glob:c*.pdf".into(),
            destination_key: "finance".into(),
        });

        let mut set = RuleSet::default();
        let added = learn_from_approved(&plan, &mut set);

        assert_eq!(added.len(), 1);
        assert_eq!(added[0].match_expr, "glob:a*.pdf");
        assert_eq!(set.learned().count(), 1);
    }

    #[test]
    fn refuses_to_learn_a_rule_pointing_somewhere_else() {
        let (_t, scan, config) = fixture(&["a.pdf"], &["finance", "documents"]);
        let mut plan = build_plan(&scan, &config, &RuleSet::default());
        plan.entries[0].action = Action::Move;
        plan.entries[0].destination_key = Some("finance".into());
        plan.entries[0].included = true;
        // Suggestion disagrees with what the user approved.
        plan.entries[0].suggest_rule = Some(SuggestRule {
            match_expr: "ext:pdf".into(),
            destination_key: "documents".into(),
        });

        let mut set = RuleSet::default();
        assert!(learn_from_approved(&plan, &mut set).is_empty());
    }

    #[test]
    fn learning_is_idempotent() {
        let (_t, scan, config) = fixture(&["a.pdf"], &["finance"]);
        let mut plan = build_plan(&scan, &config, &RuleSet::default());
        plan.entries[0].action = Action::Move;
        plan.entries[0].destination_key = Some("finance".into());
        plan.entries[0].included = true;
        plan.entries[0].suggest_rule = Some(SuggestRule {
            match_expr: "ext:pdf".into(),
            destination_key: "finance".into(),
        });

        let mut set = RuleSet::default();
        assert_eq!(learn_from_approved(&plan, &mut set).len(), 1);
        assert_eq!(learn_from_approved(&plan, &mut set).len(), 0);
        assert_eq!(set.rules.len(), 1);
    }

    #[test]
    fn resolved_without_ai_counts_the_deterministic_tiers() {
        let (_t, scan, config) = fixture(&["a.pdf", "b.pdf", "c.xyz"], &["documents"]);
        let plan = build_plan(&scan, &config, &RuleSet::default());
        assert_eq!(plan.resolved_without_ai(), 2);
    }
}
