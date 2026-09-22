//! Plain-language folder rules, and what Menlo can read out of them without a model.
//!
//! A rule is a sentence the user wrote about a folder: "Anything with the word lease
//! or agreement goes to Documents/Contracts". The sentence is the rule — it is stored
//! as written, shown as written, and handed as written to a model when one is present.
//!
//! [`read`] is the no-model reading. It pulls out the signals ordinary text matching
//! can honour — words in the name, file types, a year, a size, an age, "screenshot" —
//! and reports whether it found any. It never guesses: a sentence it cannot read is
//! reported as needing a model, and routes nothing, rather than being half-applied.
//! That is what lets Fully Local mode be useful and honest at the same time.
//!
//! Hand-rolled over a word list rather than regexes because `regex` is outside the
//! approved dependency set (rules.rs rejects `regex:` matchers for the same reason).

use crate::scan::ManifestEntry;
use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};

/// A sentence attached to one folder.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FolderRule {
    pub id: String,
    /// The folder it belongs to. A key, never a path, like everything a plan touches.
    pub destination_key: String,
    pub text: String,
    pub created_at: DateTime<Utc>,
}

impl FolderRule {
    pub fn new(destination_key: impl Into<String>, text: impl Into<String>) -> Self {
        FolderRule {
            id: uuid::Uuid::new_v4().to_string(),
            destination_key: destination_key.into(),
            text: text.into(),
            created_at: Utc::now(),
        }
    }
}

/// What the sentence is asking for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Intent {
    /// "Invoices go here." Files that match are filed into this folder.
    Route,
    /// "Only move files over 10 MB." This folder takes nothing that fails the test.
    Only,
    /// "Skip anything under 2 KB." This folder takes nothing that passes it.
    Skip,
    /// A standing instruction Menlo already follows ("never overwrite …").
    Policy,
}

/// The no-model reading of one rule.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Reading {
    pub intent: Option<Intent>,
    /// Phrases that must appear in the file name, any one of them. Each phrase is a
    /// run of tokens, so "form 16" needs `form` then `16`.
    pub keywords: Vec<Vec<String>>,
    /// Extensions, any one of them.
    pub exts: Vec<String>,
    pub screenshot: bool,
    pub year: Option<i32>,
    pub min_bytes: Option<u64>,
    pub max_bytes: Option<u64>,
    pub older_than_days: Option<i64>,
    pub newer_than_days: Option<i64>,
}

impl Reading {
    /// Whether ordinary matching can act on this rule at all.
    pub fn understood(&self) -> bool {
        self.intent == Some(Intent::Policy) || self.has_signal()
    }

    fn has_signal(&self) -> bool {
        self.has_name_or_type()
            || self.year.is_some()
            || self.min_bytes.is_some()
            || self.max_bytes.is_some()
            || self.older_than_days.is_some()
            || self.newer_than_days.is_some()
    }

    /// Names or types say *what a file is*. A year, size or age only says something
    /// about it, and would sweep in every 2026 photo if allowed to route on its own —
    /// so those are only ever used to pick a subfolder once the file's parent folder
    /// has been decided by something stronger.
    pub fn has_name_or_type(&self) -> bool {
        !self.keywords.is_empty() || !self.exts.is_empty() || self.screenshot
    }

    /// Every signal present must hold; within one kind of signal, any one will do.
    pub fn matches(&self, entry: &ManifestEntry, now: DateTime<Utc>) -> bool {
        if !self.has_signal() {
            return false;
        }
        let name_tokens = tokens(stem(&entry.name));

        if !self.keywords.is_empty() {
            let excerpt_tokens = entry.excerpt.as_deref().map(tokens).unwrap_or_default();
            let hit = self
                .keywords
                .iter()
                .any(|k| contains_phrase(&name_tokens, k) || contains_phrase(&excerpt_tokens, k));
            if !hit {
                return false;
            }
        }
        if !self.exts.is_empty() && !self.exts.contains(&entry.ext) {
            return false;
        }
        if self.screenshot && !looks_like_screenshot(&entry.name) {
            return false;
        }
        if let Some(year) = self.year {
            let in_name = name_tokens.iter().any(|t| *t == year.to_string());
            if !in_name && entry.modified_at.year() != year {
                return false;
            }
        }
        if let Some(min) = self.min_bytes {
            if entry.size_bytes < min {
                return false;
            }
        }
        if let Some(max) = self.max_bytes {
            if entry.size_bytes > max {
                return false;
            }
        }
        let age_days = (now - entry.modified_at).num_days();
        if let Some(d) = self.older_than_days {
            if age_days < d {
                return false;
            }
        }
        if let Some(d) = self.newer_than_days {
            if age_days > d {
                return false;
            }
        }
        true
    }

    /// Higher wins when two rules match the same file. A named phrase is the most
    /// deliberate thing a user can say, so it outranks a file type, which outranks
    /// the incidental signals; and a longer phrase outranks a shorter one.
    pub fn specificity(&self) -> usize {
        let mut s = 0;
        if let Some(longest) = self.keywords.iter().map(|k| k.join(" ").len()).max() {
            s += 400 + longest * 10;
        }
        if self.screenshot {
            s += 350;
        }
        if !self.exts.is_empty() {
            s += 200 + 60 / self.exts.len();
        }
        if self.year.is_some() {
            s += 100;
        }
        if self.min_bytes.is_some() || self.max_bytes.is_some() {
            s += 80;
        }
        if self.older_than_days.is_some() || self.newer_than_days.is_some() {
            s += 80;
        }
        s
    }

    /// The reading, in words, for the Rules memory page — so the user can see what
    /// Menlo took from their sentence instead of trusting that it took something.
    pub fn describe(&self) -> String {
        if self.intent == Some(Intent::Policy) {
            return "Menlo already does this — it never overwrites a file.".into();
        }
        if !self.understood() {
            return "Needs a model to apply — no names, types, dates or sizes to match on.".into();
        }
        let mut parts: Vec<String> = Vec::new();
        if !self.keywords.is_empty() {
            let words: Vec<String> = self.keywords.iter().map(|k| k.join(" ")).collect();
            parts.push(format!("names with {}", join_or(&words)));
        }
        if self.screenshot {
            parts.push("screenshots".into());
        }
        if !self.exts.is_empty() {
            let exts: Vec<String> = self.exts.iter().take(4).map(|e| format!(".{e}")).collect();
            let more = if self.exts.len() > 4 { " …" } else { "" };
            parts.push(format!("{}{more} files", exts.join(" ")));
        }
        if let Some(y) = self.year {
            parts.push(format!("from {y}"));
        }
        if let Some(b) = self.min_bytes {
            parts.push(format!("over {}", human_bytes(b)));
        }
        if let Some(b) = self.max_bytes {
            parts.push(format!("under {}", human_bytes(b)));
        }
        if let Some(d) = self.older_than_days {
            parts.push(format!("older than {d} days"));
        }
        if let Some(d) = self.newer_than_days {
            parts.push(format!("from the last {d} days"));
        }
        // Without a name or type, the sentence needs a noun to read naturally.
        let noun = if self.has_name_or_type() {
            ""
        } else {
            "files "
        };
        let what = format!("{noun}{}", parts.join(" · "));
        match self.intent {
            Some(Intent::Only) => format!("Takes only {what}"),
            Some(Intent::Skip) => format!("Never takes {what}"),
            _ => format!("Takes {what}"),
        }
    }
}

fn join_or(items: &[String]) -> String {
    match items.len() {
        0 => String::new(),
        1 => format!("“{}”", items[0]),
        _ => {
            let quoted: Vec<String> = items.iter().map(|i| format!("“{i}”")).collect();
            let (last, rest) = quoted.split_last().expect("len >= 2");
            format!("{} or {last}", rest.join(", "))
        }
    }
}

fn human_bytes(b: u64) -> String {
    const GB: u64 = 1024 * 1024 * 1024;
    const MB: u64 = 1024 * 1024;
    if b >= GB && b % GB == 0 {
        format!("{} GB", b / GB)
    } else if b >= MB && b % MB == 0 {
        format!("{} MB", b / MB)
    } else {
        format!("{} KB", b / 1024)
    }
}

// ── Reading a sentence ─────────────────────────────────────────────────────────

/// Words that introduce a list of name keywords, and how many words the cue spans.
const KEYWORD_CUES: &[&[&str]] = &[
    &["word"],
    &["words"],
    &["containing"],
    &["contains"],
    &["contain"],
    &["named"],
    &["called"],
    &["titled"],
    &["mentions"],
    &["mentioning"],
    &["read", "like"],
    &["reads", "like"],
    &["look", "like"],
    &["looks", "like"],
];

/// A keyword list ends at the first of these.
const LIST_STOPS: &[&str] = &[
    "in", "to", "into", "from", "that", "which", "go", "goes", "land", "lands", "should", "must",
    "will", "over", "under", "above", "below", "older", "newer", "larger", "smaller", "bigger",
    "than", "dated", "created", "modified", "this", "per", "by", "except", "only", "with",
    "together", "if", "when", "are", "is",
];

const FILLER: &[&str] = &["a", "an", "the", "any", "some", "files", "file"];

/// Verbs that, followed by a preposition, begin the "…goes to Documents/Contracts"
/// clause. That clause names the destination, not a condition, so it is cut before
/// the sentence is read — otherwise "Documents" would be taken for a file type.
const MOVE_VERBS: &[&str] = &[
    "go", "goes", "going", "land", "lands", "put", "move", "moves", "send", "sent", "file",
    "filed", "belong", "belongs",
];
const MOVE_PREPS: &[&str] = &["to", "in", "into", "under", "inside"];

/// File-type words, singular, and the extensions they stand for. Deliberately leaves
/// out words that are just as often folder names ("documents", "archive" as a noun
/// is kept because "zip"-type archives are what people mean by it in a rule).
const TYPE_WORDS: &[(&str, &[&str])] = &[
    ("pdf", &["pdf"]),
    (
        "image",
        &[
            "jpg", "jpeg", "png", "gif", "webp", "heic", "tiff", "tif", "bmp", "avif",
        ],
    ),
    (
        "photo",
        &[
            "jpg", "jpeg", "png", "heic", "webp", "tiff", "tif", "raw", "dng",
        ],
    ),
    (
        "picture",
        &["jpg", "jpeg", "png", "gif", "webp", "heic", "tiff", "tif"],
    ),
    (
        "video",
        &[
            "mp4", "mov", "mkv", "avi", "webm", "m4v", "mpg", "mpeg", "wmv",
        ],
    ),
    ("movie", &["mp4", "mov", "mkv", "avi", "webm", "m4v"]),
    ("recording", &["mp4", "mov", "m4v", "webm", "m4a", "wav"]),
    (
        "audio",
        &["mp3", "m4a", "wav", "flac", "aac", "ogg", "aiff", "aif"],
    ),
    ("song", &["mp3", "m4a", "wav", "flac", "aac", "ogg"]),
    (
        "music",
        &["mp3", "m4a", "wav", "flac", "aac", "ogg", "aiff"],
    ),
    ("podcast", &["mp3", "m4a", "aac"]),
    (
        "archive",
        &["zip", "tar", "gz", "tgz", "bz2", "xz", "7z", "rar", "zst"],
    ),
    ("zip", &["zip"]),
    ("installer", &["dmg", "pkg", "mpkg", "iso"]),
    (
        "spreadsheet",
        &["xls", "xlsx", "csv", "tsv", "ods", "numbers"],
    ),
    ("presentation", &["ppt", "pptx", "odp", "key"]),
    ("slide", &["ppt", "pptx", "odp", "key"]),
    ("font", &["otf", "ttf", "woff", "woff2"]),
    ("ebook", &["epub", "mobi", "azw3"]),
    (
        "code",
        &[
            "rs", "ts", "tsx", "js", "jsx", "py", "go", "rb", "java", "kt", "swift", "c", "h",
            "cpp", "cs", "php", "sh", "sql",
        ],
    ),
];

/// Extensions a user might name directly: "PDFs", ".heic", "csv files".
///
/// Leaves out the extensions that are also everyday words — `key`, `pages`,
/// `numbers`, `fig` — so "keep key documents here" is not read as Keynote files.
/// Those formats are still reachable through "presentations" and "spreadsheets".
const KNOWN_EXTS: &[&str] = &[
    "pdf", "doc", "docx", "txt", "md", "rtf", "xls", "xlsx", "csv", "ppt", "pptx", "jpg", "jpeg",
    "png", "gif", "webp", "heic", "tiff", "svg", "mp4", "mov", "mkv", "mp3", "m4a", "wav", "flac",
    "zip", "dmg", "pkg", "iso", "epub", "json", "sql", "rs", "ts", "js", "py", "psd", "sketch",
];

/// Read a sentence. Never fails: the worst case is a reading that understood nothing,
/// which is itself the honest answer.
pub fn read(text: &str) -> Reading {
    let lower = text.to_lowercase();
    let mut r = Reading::default();

    if lower.contains("overwrite") {
        r.intent = Some(Intent::Policy);
        return r;
    }

    // Quoted phrases are keywords, whatever surrounds them.
    let (unquoted, quoted) = take_quoted(&lower);
    for q in quoted {
        let phrase = phrase_tokens(&q);
        if !phrase.is_empty() {
            r.keywords.push(phrase);
        }
    }

    let mut words = words_of(&unquoted);
    cut_destination_clause(&mut words);

    r.intent = Some(intent_of(&words));

    // Keyword lists, consumed so their words are not read again as types or years.
    let mut consumed = vec![false; words.len()];
    read_keyword_lists(&words, &mut consumed, &mut r);

    // Size and age, which consume their numbers so a "2026 KB" cannot also be a year.
    read_sizes_and_ages(&words, &mut consumed, &mut r);

    for (i, w) in words.iter().enumerate() {
        if consumed[i] {
            continue;
        }
        let w = w.trim_start_matches('.');
        if w.starts_with("screenshot") || (w == "screen" && next_is(&words, i, "shot")) {
            r.screenshot = true;
            continue;
        }
        if let Ok(y) = w.parse::<i32>() {
            if (1900..=2100).contains(&y) && w.len() == 4 {
                r.year = Some(y);
                continue;
            }
        }
        let singular = singular(w);
        if let Some((_, exts)) = TYPE_WORDS.iter().find(|(word, _)| *word == singular) {
            push_exts(&mut r.exts, exts);
            continue;
        }
        if KNOWN_EXTS.contains(&singular.as_str()) {
            push_exts(&mut r.exts, &[singular.as_str()]);
        }
    }

    r
}

fn next_is(words: &[String], i: usize, want: &str) -> bool {
    words
        .get(i + 1)
        .map(|w| w.starts_with(want))
        .unwrap_or(false)
}

fn push_exts(into: &mut Vec<String>, exts: &[&str]) {
    for e in exts {
        if !into.iter().any(|x| x == e) {
            into.push((*e).to_string());
        }
    }
}

/// Split out text between double or curly quotes. Single quotes are left alone —
/// "don't" is far more common in a rule than a single-quoted phrase.
fn take_quoted(s: &str) -> (String, Vec<String>) {
    let mut rest = String::new();
    let mut quoted = Vec::new();
    let mut current: Option<String> = None;
    for c in s.chars() {
        match (c, current.as_mut()) {
            ('"' | '“' | '”', None) => current = Some(String::new()),
            ('"' | '“' | '”', Some(q)) => {
                quoted.push(std::mem::take(q));
                current = None;
                rest.push(' ');
            }
            (c, Some(q)) => q.push(c),
            (c, None) => rest.push(c),
        }
    }
    // An unclosed quote is just text.
    if let Some(q) = current {
        rest.push_str(&q);
    }
    (rest, quoted)
}

/// Whitespace words, trailing punctuation trimmed, commas kept as their own word so a
/// keyword list can split on them. Words with a `/` are paths ("Documents/Contracts")
/// and are dropped: they name a destination, never a condition.
fn words_of(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in s.split_whitespace() {
        let comma = raw.ends_with(',');
        let w = raw.trim_matches(|c: char| ",;:!?()[]“”\"'".contains(c));
        let w = w.trim_end_matches('.');
        if !w.is_empty() && !w.contains('/') {
            out.push(w.to_string());
        }
        if comma {
            out.push(",".to_string());
        }
    }
    out
}

fn cut_destination_clause(words: &mut Vec<String>) {
    let cut = words.iter().enumerate().position(|(i, w)| {
        MOVE_VERBS.contains(&w.as_str())
            && words
                .iter()
                .skip(i + 1)
                .take(2)
                .any(|n| MOVE_PREPS.contains(&n.as_str()))
    });
    if let Some(at) = cut {
        words.truncate(at);
    }
}

fn intent_of(words: &[String]) -> Intent {
    let first = words.first().map(String::as_str).unwrap_or("");
    let second = words.get(1).map(String::as_str).unwrap_or("");
    match (first, second) {
        ("only" | "just", _) => Intent::Only,
        ("skip" | "ignore" | "exclude" | "leave" | "don't" | "dont", _) => Intent::Skip,
        ("do", "not") | ("never", "move") | ("never", "file") => Intent::Skip,
        _ => Intent::Route,
    }
}

fn read_keyword_lists(words: &[String], consumed: &mut [bool], r: &mut Reading) {
    let mut i = 0;
    while i < words.len() {
        // "with lease in the name"
        if words[i] == "with" {
            if let Some(end) = find_in_the_name(words, i + 1) {
                take_list(words, i + 1, end, consumed, r);
                mark(consumed, i, end + 3);
                i = end + 3;
                continue;
            }
        }
        // "keep Form 16 and receipts together"
        if words[i] == "keep" {
            if let Some(end) = (i + 1..words.len()).find(|&j| words[j] == "together") {
                take_list(words, i + 1, end, consumed, r);
                mark(consumed, i, end + 1);
                i = end + 1;
                continue;
            }
        }
        if let Some(cue) = KEYWORD_CUES.iter().find(|cue| {
            cue.iter()
                .enumerate()
                .all(|(k, c)| words.get(i + k).map(String::as_str) == Some(*c))
        }) {
            let start = i + cue.len();
            let end = (start..words.len())
                .find(|&j| LIST_STOPS.contains(&words[j].as_str()))
                .unwrap_or(words.len());
            take_list(words, start, end, consumed, r);
            mark(consumed, i, end);
            i = end.max(i + 1);
            continue;
        }
        i += 1;
    }
}

fn find_in_the_name(words: &[String], from: usize) -> Option<usize> {
    (from..words.len()).find(|&j| {
        words.get(j).map(String::as_str) == Some("in")
            && words.get(j + 1).map(String::as_str) == Some("the")
            && matches!(
                words.get(j + 2).map(String::as_str),
                Some("name" | "title" | "filename")
            )
    })
}

fn mark(consumed: &mut [bool], from: usize, to: usize) {
    let to = to.min(consumed.len());
    for c in consumed.iter_mut().take(to).skip(from) {
        *c = true;
    }
}

/// Words `[start, end)` as a list split on "or", "and" and commas.
fn take_list(words: &[String], start: usize, end: usize, consumed: &mut [bool], r: &mut Reading) {
    let mut phrase: Vec<String> = Vec::new();
    let flush = |phrase: &mut Vec<String>, r: &mut Reading| {
        let text = phrase.join(" ");
        let toks = phrase_tokens(&text);
        if !toks.is_empty() && !r.keywords.contains(&toks) {
            r.keywords.push(toks);
        }
        phrase.clear();
    };
    for j in start..end.min(words.len()) {
        consumed[j] = true;
        let w = words[j].as_str();
        if w == "or" || w == "and" || w == "," {
            flush(&mut phrase, r);
        } else if !FILLER.contains(&w) {
            phrase.push(w.to_string());
        }
    }
    flush(&mut phrase, r);
}

/// A keyword phrase as match tokens: each word singular, then split the same way a
/// filename is, so "Form16" in a rule and "form_16" in a name agree.
fn phrase_tokens(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(singular)
        .flat_map(|w| tokens(&w))
        .collect()
}

const SIZE_MIN: &[&[&str]] = &[
    &["over"],
    &["above"],
    &["more", "than"],
    &["larger", "than"],
    &["bigger", "than"],
    &["at", "least"],
];
const SIZE_MAX: &[&[&str]] = &[
    &["under"],
    &["below"],
    &["less", "than"],
    &["smaller", "than"],
    &["at", "most"],
];

fn read_sizes_and_ages(words: &[String], consumed: &mut [bool], r: &mut Reading) {
    for i in 0..words.len() {
        if consumed[i] {
            continue;
        }
        // "older than 30 days"
        if (words[i] == "older" || words[i] == "newer")
            && words.get(i + 1).map(String::as_str) == Some("than")
        {
            if let Some((days, used)) = duration_days(words, i + 2) {
                if words[i] == "older" {
                    r.older_than_days = Some(days);
                } else {
                    r.newer_than_days = Some(days);
                }
                mark(consumed, i, i + 2 + used);
                continue;
            }
        }
        // "this week" / "this month" / "this year"
        if words[i] == "this" {
            let days = match words.get(i + 1).map(String::as_str) {
                Some("week") => Some(7),
                Some("month") => Some(31),
                Some("year") => Some(366),
                _ => None,
            };
            if let Some(d) = days {
                r.newer_than_days = Some(d);
                mark(consumed, i, i + 2);
                continue;
            }
        }
        for (cues, is_min) in [(SIZE_MIN, true), (SIZE_MAX, false)] {
            let Some(cue) = cues.iter().find(|cue| {
                cue.iter()
                    .enumerate()
                    .all(|(k, c)| words.get(i + k).map(String::as_str) == Some(*c))
            }) else {
                continue;
            };
            if let Some((bytes, used)) = size_bytes(words, i + cue.len()) {
                if is_min {
                    r.min_bytes = Some(bytes);
                } else {
                    r.max_bytes = Some(bytes);
                }
                mark(consumed, i, i + cue.len() + used);
            }
        }
    }
}

/// "10 MB", "10MB", "2.5 gb" at `at`. Returns bytes and how many words it used.
fn size_bytes(words: &[String], at: usize) -> Option<(u64, usize)> {
    let first = words.get(at)?;
    let split = first
        .find(|c: char| c.is_ascii_alphabetic())
        .unwrap_or(first.len());
    let (num, unit_inline) = first.split_at(split);
    let n: f64 = num.parse().ok()?;
    let (unit, used) = if unit_inline.is_empty() {
        (words.get(at + 1).map(String::as_str).unwrap_or(""), 2)
    } else {
        (unit_inline, 1)
    };
    let mult: f64 = match unit {
        "kb" | "k" => 1024.0,
        "mb" | "m" => 1024.0 * 1024.0,
        "gb" | "g" => 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };
    Some(((n * mult) as u64, used))
}

/// "30 days", "2 weeks", "6 months" at `at`. Returns days and words used.
fn duration_days(words: &[String], at: usize) -> Option<(i64, usize)> {
    let n: i64 = words.get(at)?.parse().ok()?;
    let per = match singular(words.get(at + 1)?).as_str() {
        "day" => 1,
        "week" => 7,
        "month" => 30,
        "year" => 365,
        _ => return None,
    };
    Some((n * per, 2))
}

// ── Matching ───────────────────────────────────────────────────────────────────

/// "leases" → "lease", "boxes" → "box", "receipts" → "receipt". Deliberately timid:
/// matching is by prefix, so leaving a word slightly long costs nothing, while
/// cutting it too short ("movies" → "movy") would stop it matching at all.
pub fn singular(w: &str) -> String {
    let w = w.to_lowercase();
    for suffix in ["sses", "xes", "ches", "shes"] {
        if w.len() > suffix.len() + 1 && w.ends_with(suffix) {
            return w[..w.len() - 2].to_string();
        }
    }
    if w.len() > 3
        && w.ends_with('s')
        && !w.ends_with("ss")
        && !w.ends_with("us")
        && !w.ends_with("is")
    {
        return w[..w.len() - 1].to_string();
    }
    w
}

/// A filename without its extension.
pub fn stem(name: &str) -> &str {
    match name.rfind('.') {
        Some(i) if i > 0 => &name[..i],
        _ => name,
    }
}

/// Lowercase word tokens: split on anything not alphanumeric, between letters and
/// digits, and at a camelCase hump. `Invoice_Aug2026` → invoice, aug, 2026.
pub fn tokens(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut prev: Option<char> = None;
    for c in s.chars() {
        if !c.is_alphanumeric() {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            prev = None;
            continue;
        }
        if let Some(p) = prev {
            let boundary = (p.is_alphabetic() && c.is_numeric())
                || (p.is_numeric() && c.is_alphabetic())
                || (p.is_lowercase() && c.is_uppercase());
            if boundary && !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
        }
        cur.extend(c.to_lowercase());
        prev = Some(c);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Whether `phrase` appears as a run inside `hay`, token for token.
///
/// A word of three letters or more matches as a prefix, so "invoice" finds
/// "invoices" and "tax" finds "taxes". Matching is per token and anchored at the
/// token's start, so "tax" never finds "syntax". A number must match exactly, so
/// "16" never finds "160", and so must a one- or two-letter word.
pub fn contains_phrase(hay: &[String], phrase: &[String]) -> bool {
    if phrase.is_empty() || phrase.len() > hay.len() {
        return false;
    }
    hay.windows(phrase.len()).any(|win| {
        win.iter().zip(phrase).all(|(h, p)| {
            if p.chars().all(|c| c.is_ascii_digit()) || p.chars().count() < 3 {
                h == p
            } else {
                h.starts_with(p.as_str())
            }
        })
    })
}

/// macOS, CleanShot and the Simulator all name screenshots recognisably.
pub fn looks_like_screenshot(name: &str) -> bool {
    let n = name.to_lowercase();
    n.starts_with("screenshot")
        || n.starts_with("screen shot")
        || n.starts_with("cleanshot")
        || n.starts_with("simulator screen shot")
        || n.starts_with("simulator screenshot")
}

/// The rule a subfolder follows when nobody has written one for it: files whose
/// names carry every word of the folder's name. `Invoices 2026` takes
/// `Invoice_Aug2026.pdf` but not `tax_form_2026.pdf`. Shown in the UI as the
/// placeholder under an empty subfolder, so it is never an invisible behaviour.
pub fn implicit_subfolder_reading(label: &str) -> Reading {
    let words: Vec<Vec<String>> = label
        .split_whitespace()
        .map(singular)
        .map(|w| tokens(&w))
        .filter(|t| !t.is_empty())
        .collect();
    Reading {
        intent: Some(Intent::Route),
        // One phrase per word, and all of them required — see `matches_all_words`.
        keywords: words,
        ..Reading::default()
    }
}

/// The implicit rule needs every word, where a written rule needs any one.
pub fn matches_all_words(reading: &Reading, entry: &ManifestEntry) -> bool {
    if reading.keywords.is_empty() {
        return false;
    }
    let name_tokens = tokens(stem(&entry.name));
    reading
        .keywords
        .iter()
        .all(|k| contains_phrase(&name_tokens, k))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn entry(name: &str) -> ManifestEntry {
        entry_sized(name, 1000)
    }

    fn entry_sized(name: &str, size: u64) -> ManifestEntry {
        let ext = name
            .rsplit_once('.')
            .map(|(_, e)| e.to_lowercase())
            .unwrap_or_default();
        ManifestEntry {
            id: "f_000".into(),
            name: name.into(),
            ext,
            size_bytes: size,
            created_at: Utc.with_ymd_and_hms(2026, 8, 14, 10, 0, 0).unwrap(),
            modified_at: Utc.with_ymd_and_hms(2026, 8, 14, 10, 0, 0).unwrap(),
            mime: "application/octet-stream".into(),
            origin: None,
            excerpt: None,
        }
    }

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 22, 12, 0, 0).unwrap()
    }

    fn kw(r: &Reading) -> Vec<String> {
        r.keywords.iter().map(|k| k.join(" ")).collect()
    }

    #[test]
    fn tokenises_names_the_way_people_write_them() {
        assert_eq!(tokens("Invoice_Aug2026"), vec!["invoice", "aug", "2026"]);
        assert_eq!(tokens("tax_form_16"), vec!["tax", "form", "16"]);
        assert_eq!(tokens("AnnualReport"), vec!["annual", "report"]);
        assert_eq!(
            tokens("Screenshot 2026-09-02 at 10.14.33"),
            vec!["screenshot", "2026", "09", "02", "at", "10", "14", "33"]
        );
    }

    #[test]
    fn singular_is_timid() {
        assert_eq!(singular("leases"), "lease");
        assert_eq!(singular("invoices"), "invoice");
        assert_eq!(singular("receipts"), "receipt");
        assert_eq!(singular("boxes"), "box");
        assert_eq!(singular("classes"), "class");
        assert_eq!(singular("movies"), "movie");
        assert_eq!(singular("status"), "status");
        assert_eq!(singular("analysis"), "analysis");
    }

    #[test]
    fn phrases_match_by_prefix_but_numbers_and_short_words_exactly() {
        let hay = tokens("Lease_Agreement_signed");
        assert!(contains_phrase(&hay, &["lease".into()]));
        assert!(contains_phrase(&hay, &["agreement".into(), "sign".into()]));

        let form = tokens("tax_form_160");
        assert!(!contains_phrase(&form, &["form".into(), "16".into()]));
        assert!(!contains_phrase(&tokens("syntax"), &["tax".into()]));
        assert!(contains_phrase(&tokens("tax_form_16"), &["tax".into()]));
    }

    // ── The design's own example rules ────────────────────────────────────────

    #[test]
    fn reads_a_word_list_and_ignores_the_destination_clause() {
        let r = read("Anything with the word lease or agreement goes to Documents/Contracts");
        assert_eq!(r.intent, Some(Intent::Route));
        assert_eq!(kw(&r), vec!["lease", "agreement"]);
        // "Documents" named the destination; it must not have become a file type.
        assert!(r.exts.is_empty());
        assert!(r.matches(&entry("Lease_Agreement_signed.pdf"), now()));
        assert!(r.matches(&entry("agreement-v2.docx"), now()));
        assert!(!r.matches(&entry("receipt.pdf"), now()));
    }

    #[test]
    fn reads_a_read_like_rule() {
        let r = read("Files that read like contracts land in Documents/Contracts");
        assert_eq!(kw(&r), vec!["contract"]);
        assert!(r.matches(&entry("Contract_signed.pdf"), now()));
    }

    #[test]
    fn reads_a_year() {
        let r = read("Anything dated 2026 lands in Documents/Invoices 2026");
        assert_eq!(r.year, Some(2026));
        assert!(!r.has_name_or_type(), "a year alone is a weak signal");
    }

    #[test]
    fn reads_keep_together() {
        let r = read("Create Documents/Tax 2026 and keep Form 16 and receipts together");
        assert_eq!(kw(&r), vec!["form 16", "receipt"]);
        assert!(r.matches(&entry("tax_form_16.pdf"), now()));
        assert!(r.matches(&entry("receipt_flight_BLR_DEL.pdf"), now()));
    }

    #[test]
    fn reads_screenshots_and_age() {
        let r = read("Screenshots older than 30 days go to Pictures/Screenshots/Archive");
        assert!(r.screenshot);
        assert_eq!(r.older_than_days, Some(30));
        // Modified 14 Aug, "now" is 22 Sep: 39 days.
        assert!(r.matches(&entry("Screenshot 2026-08-14 at 10.00.00.png"), now()));
        assert!(!r.matches(&entry("holiday.png"), now()));
    }

    #[test]
    fn reads_a_size_gate() {
        let r = read("Only move files over 10 MB");
        assert_eq!(r.intent, Some(Intent::Only));
        assert_eq!(r.min_bytes, Some(10 * 1024 * 1024));
        assert!(r.matches(&entry_sized("a.mov", 11 * 1024 * 1024), now()));
        assert!(!r.matches(&entry_sized("a.mov", 9 * 1024 * 1024), now()));
    }

    #[test]
    fn reads_a_skip_with_an_inline_unit() {
        let r = read("Skip anything under 2KB");
        assert_eq!(r.intent, Some(Intent::Skip));
        assert_eq!(r.max_bytes, Some(2 * 1024));
    }

    #[test]
    fn reads_types_and_sizes_together() {
        let r = read("Recordings over 500 MB go to Movies/Recordings/Large");
        assert!(r.exts.contains(&"mov".to_string()));
        assert_eq!(r.min_bytes, Some(500 * 1024 * 1024));
        assert!(r.has_name_or_type());
    }

    #[test]
    fn reads_quoted_phrases() {
        let r = read(r#"Anything called "Q3 forecast" goes here"#);
        assert_eq!(kw(&r), vec!["q 3 forecast"]);
        assert!(r.matches(&entry("Q3_forecast.xlsx"), now()));
    }

    #[test]
    fn recognises_a_policy_it_already_follows() {
        let r = read("Never overwrite a file already in here");
        assert_eq!(r.intent, Some(Intent::Policy));
        assert!(r.understood());
        assert!(
            !r.matches(&entry("a.pdf"), now()),
            "a policy routes nothing"
        );
    }

    #[test]
    fn says_so_when_it_needs_a_model() {
        for text in [
            "File by type, newest first",
            "Keep files grouped by year",
            "Leave duplicates where they are",
        ] {
            let r = read(text);
            assert!(!r.understood(), "`{text}` should need a model, got {r:?}");
            assert!(!r.matches(&entry("anything.pdf"), now()));
            assert!(r.describe().starts_with("Needs a model"));
        }
    }

    #[test]
    fn everyday_words_are_not_mistaken_for_extensions() {
        let r = read("Keep key documents and page numbers here");
        assert!(r.exts.is_empty(), "got {:?}", r.exts);
    }

    #[test]
    fn a_this_month_rule_is_about_recency() {
        let r = read("Only files created this month");
        assert_eq!(r.intent, Some(Intent::Only));
        assert_eq!(r.newer_than_days, Some(31));
    }

    #[test]
    fn describes_what_it_understood() {
        let r = read("Anything with the word lease or agreement goes to Documents/Contracts");
        assert_eq!(r.describe(), "Takes names with “lease” or “agreement”");
        let r = read("Only move files over 10 MB");
        assert_eq!(r.describe(), "Takes only files over 10 MB");
        let r = read("Anything dated 2026 lands in Documents/Invoices 2026");
        assert_eq!(r.describe(), "Takes files from 2026");
    }

    #[test]
    fn an_implicit_subfolder_rule_needs_every_word() {
        let r = implicit_subfolder_reading("Invoices 2026");
        assert!(matches_all_words(&r, &entry("Invoice_Aug2026.pdf")));
        assert!(!matches_all_words(&r, &entry("tax_form_2026.pdf")));
        assert!(!matches_all_words(&r, &entry("Invoice_Aug2025.pdf")));

        let s = implicit_subfolder_reading("Screenshots");
        assert!(matches_all_words(
            &s,
            &entry("Screenshot 2026-09-02 at 10.14.33.png")
        ));
    }

    #[test]
    fn screenshot_names_from_every_tool() {
        for n in [
            "Screenshot 2026-09-02 at 10.14.33.png",
            "Screen Shot 2021-01-01 at 1.00.00 PM.png",
            "CleanShot 2026-09-02 at 10.14.33@2x.png",
            "Simulator Screen Shot - iPhone 15 - 2026-09-02.png",
        ] {
            assert!(looks_like_screenshot(n), "{n}");
        }
        assert!(!looks_like_screenshot("screens.png"));
    }
}
