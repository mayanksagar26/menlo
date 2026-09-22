//! Clearer names for files whose names say nothing.
//!
//! `Screenshot 2026-09-02 at 10.14.33.png`, `IMG_4821.heic`, `download (3).pdf` —
//! names a tool chose, not a person. Once Menlo knows which folder a file belongs in,
//! it can do better: the folder's name and the moment the file was made, which is how
//! people actually look for these things later.
//!
//! This is the no-model half. Naming a screenshot after what is *in* it — "Boarding
//! pass BLR–DEL" — needs a vision model and arrives with the Gemma harness; it will
//! fill the same `rename_to` field, so nothing downstream changes.
//!
//! Only names that match a known tool pattern are touched, and only when the new name
//! would say more than the old one. A screenshot filed into top-level Pictures keeps
//! its name — calling it "Pictures 2026-09-02" would lose the one thing its name did
//! say. Filed into Documents/Tax 2026, it becomes "Tax 2026 2026-09-02 10.14.33",
//! which says what it is for. A person's own name for a file, however terse, is never
//! touched. Every proposal is shown in the move preview before anything happens, and a
//! revert restores the original name.

use crate::config::Config;
use crate::plan::Plan;
use crate::scan::{ManifestEntry, Scan};
use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, Utc};

/// Camera and recorder prefixes: `IMG_4821`, `DSC01234`, `PXL_20260902_101433`.
const CAMERA_PREFIXES: &[&str] = &[
    "img_",
    "img-",
    "dsc_",
    "dsc0",
    "dscn",
    "dscf",
    "pxl_",
    "vid_",
    "mov_",
    "gopr",
    "dji_",
    "p100",
    "mvimg_",
    "screen recording",
];

/// Whole names that carry no information at all.
const GENERIC_STEMS: &[&str] = &[
    "image",
    "download",
    "untitled",
    "unknown",
    "file",
    "document",
    "scan",
    "photo",
    "picture",
    "video",
    "export",
    "output",
    "new document",
];

/// Why a name reads as tool-chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Unclear {
    /// A screenshot or camera name. It does say what the file *is*, just not what it
    /// is for — so it is worth replacing only with the name of a specific folder.
    Descriptive,
    /// `download (3)`, `Untitled`, a hash. Says nothing; anything is an improvement.
    Empty,
}

fn unclear(name: &str) -> Option<Unclear> {
    let lower = name.to_lowercase();
    if crate::prose::looks_like_screenshot(&lower)
        || CAMERA_PREFIXES.iter().any(|p| lower.starts_with(p))
    {
        return Some(Unclear::Descriptive);
    }
    let stem = strip_copy_suffix(crate::prose::stem(&lower));
    (GENERIC_STEMS.contains(&stem) || looks_like_hash(stem)).then_some(Unclear::Empty)
}

/// Whether this name was chosen by a tool rather than a person.
pub fn is_unclear(name: &str) -> bool {
    unclear(name).is_some()
}

/// `download (3)` → `download`, `image copy 2` → `image`.
fn strip_copy_suffix(stem: &str) -> &str {
    let s = stem.trim();
    let s = match s.rfind(" (") {
        Some(i)
            if s.ends_with(')') && s[i + 2..s.len() - 1].chars().all(|c| c.is_ascii_digit()) =>
        {
            &s[..i]
        }
        _ => s,
    };
    let s = s.trim_end_matches(|c: char| c.is_ascii_digit()).trim_end();
    s.strip_suffix(" copy").unwrap_or(s).trim()
}

/// A UUID or a long run of hex — what a web app names a file when it cannot be
/// bothered to.
fn looks_like_hash(stem: &str) -> bool {
    let hex: String = stem.chars().filter(|c| *c != '-' && *c != '_').collect();
    hex.len() >= 16 && hex.chars().all(|c| c.is_ascii_hexdigit())
}

/// Propose a name for every entry that would gain from one. Leaves alone any entry
/// that already has a `rename_to` — the user, or a model, has named it already.
pub fn propose(plan: &mut Plan, scan: &Scan, config: &Config) {
    for e in plan.entries.iter_mut() {
        if e.rename_to.is_some() {
            continue;
        }
        let Some(dest) = e
            .destination_key
            .as_deref()
            .and_then(|k| config.destination(k))
        else {
            continue;
        };
        let Some(item) = scan.get(&e.id) else {
            continue;
        };
        e.rename_to = suggest(&item.entry, &dest.label, dest.is_subfolder());
    }
}

/// A clearer name for `entry` filed into a folder called `folder_label`, or `None` if
/// the new name would not say more than the old one. Never contains `/`, `..` or a
/// leading dot, so it always passes `plan::validate_rename`.
pub fn suggest(entry: &ManifestEntry, folder_label: &str, into_subfolder: bool) -> Option<String> {
    match unclear(&entry.name)? {
        Unclear::Descriptive if !into_subfolder => return None,
        _ => {}
    }
    let when = captured_at(&entry.name).unwrap_or_else(|| local(entry.modified_at));
    let label = clean_label(folder_label);
    let stamp = when.format("%Y-%m-%d %H.%M.%S");
    let name = if entry.ext.is_empty() {
        format!("{label} {stamp}")
    } else {
        format!("{label} {stamp}.{}", entry.ext)
    };
    crate::plan::validate_rename(&name).ok().map(|_| name)
}

fn local(t: DateTime<Utc>) -> NaiveDateTime {
    t.with_timezone(&Local).naive_local()
}

/// A folder name made safe to lead a filename: no path separators, no leading dots,
/// no doubled dots, and never empty.
fn clean_label(label: &str) -> String {
    let mut s: String = label
        .chars()
        .map(|c| if c == '/' || c == ':' { '-' } else { c })
        .collect();
    while s.contains("..") {
        s = s.replace("..", ".");
    }
    let s = s.trim().trim_start_matches('.').trim();
    if s.is_empty() {
        "File".to_string()
    } else {
        s.to_string()
    }
}

/// The capture time written into a screenshot's or camera file's own name. More
/// accurate than the modified date, which a copy or a sync can change.
///
/// Understands `2026-09-02 at 10.14.33`, `2026-09-02 at 1.00.00 PM`, and the compact
/// `20260902_101433` Pixel and Android cameras use.
pub fn captured_at(name: &str) -> Option<NaiveDateTime> {
    let toks = crate::prose::tokens(crate::prose::stem(name));

    // `…2026 09 02 [at] 10 14 33 [pm]`
    for i in 0..toks.len() {
        let (Some(y), Some(m), Some(d)) = (num(&toks, i), num(&toks, i + 1), num(&toks, i + 2))
        else {
            continue;
        };
        if toks[i].len() != 4 {
            continue;
        }
        let Some(date) = NaiveDate::from_ymd_opt(y as i32, m, d) else {
            continue;
        };
        let mut j = i + 3;
        if toks.get(j).map(String::as_str) == Some("at") {
            j += 1;
        }
        let time = match (num(&toks, j), num(&toks, j + 1), num(&toks, j + 2)) {
            (Some(h), Some(min), Some(s)) => {
                let h = match toks.get(j + 3).map(String::as_str) {
                    Some("pm") if h < 12 => h + 12,
                    Some("am") if h == 12 => 0,
                    _ => h,
                };
                NaiveTime::from_hms_opt(h, min, s)
            }
            _ => None,
        };
        return Some(date.and_time(time.unwrap_or(NaiveTime::MIN)));
    }

    // `20260902 101433`
    for i in 0..toks.len() {
        let t = &toks[i];
        if t.len() == 8 && t.chars().all(|c| c.is_ascii_digit()) {
            let date = NaiveDate::parse_from_str(t, "%Y%m%d").ok()?;
            let time = toks
                .get(i + 1)
                .filter(|n| n.len() >= 6 && n.chars().all(|c| c.is_ascii_digit()))
                .and_then(|n| NaiveTime::parse_from_str(&n[..6], "%H%M%S").ok());
            return Some(date.and_time(time.unwrap_or(NaiveTime::MIN)));
        }
    }
    None
}

fn num(toks: &[String], i: usize) -> Option<u32> {
    toks.get(i).and_then(|t| t.parse().ok())
}

/// Stable for tests: the same instant in UTC, whatever machine runs them.
#[cfg(test)]
fn utc(y: i32, m: u32, d: u32, h: u32, min: u32, s: u32) -> DateTime<Utc> {
    use chrono::TimeZone;
    Utc.with_ymd_and_hms(y, m, d, h, min, s).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str) -> ManifestEntry {
        let ext = name
            .rsplit_once('.')
            .map(|(_, e)| e.to_lowercase())
            .unwrap_or_default();
        ManifestEntry {
            id: "f_000".into(),
            name: name.into(),
            ext,
            size_bytes: 1,
            created_at: utc(2026, 9, 2, 10, 0, 0),
            modified_at: utc(2026, 9, 2, 10, 0, 0),
            mime: "application/octet-stream".into(),
            origin: None,
            excerpt: None,
        }
    }

    #[test]
    fn spots_names_a_tool_chose() {
        for n in [
            "Screenshot 2026-09-02 at 10.14.33.png",
            "CleanShot 2026-09-02 at 10.14.33@2x.png",
            "IMG_4821.heic",
            "DSC01234.JPG",
            "PXL_20260902_101433123.jpg",
            "download.pdf",
            "download (3).pdf",
            "image copy 2.png",
            "Untitled.png",
            "3f2a9c1e-77b4-4c1a-9d0e-5b6f7a8c9d0e.pdf",
        ] {
            assert!(is_unclear(n), "{n} should read as unclear");
        }
    }

    #[test]
    fn leaves_a_persons_names_alone() {
        for n in [
            "Invoice_Aug2026.pdf",
            "tax_form_16.pdf",
            "moodboard_v3.png",
            "Q3_forecast.xlsx",
            "img.png", // Terse, but chosen — there is no camera number after it.
            "photos of mum.zip",
        ] {
            assert!(!is_unclear(n), "{n} should be left alone");
        }
    }

    #[test]
    fn reads_the_capture_time_out_of_the_name() {
        let t = captured_at("Screenshot 2026-09-02 at 10.14.33.png").unwrap();
        assert_eq!(t.to_string(), "2026-09-02 10:14:33");

        let pm = captured_at("Screen Shot 2021-01-05 at 1.07.09 PM.png").unwrap();
        assert_eq!(pm.to_string(), "2021-01-05 13:07:09");

        let pixel = captured_at("PXL_20260902_101433123.jpg").unwrap();
        assert_eq!(pixel.to_string(), "2026-09-02 10:14:33");

        assert!(captured_at("IMG_4821.heic").is_none());
    }

    #[test]
    fn names_a_screenshot_after_its_folder_and_capture_time() {
        let name = suggest(
            &entry("Screenshot 2026-09-02 at 10.14.33.png"),
            "Tax 2026",
            true,
        );
        assert_eq!(name.as_deref(), Some("Tax 2026 2026-09-02 10.14.33.png"));
    }

    #[test]
    fn a_screenshot_in_a_top_level_folder_keeps_its_name() {
        // "Pictures 2026-09-02 …" would say less than "Screenshot 2026-09-02 …".
        assert!(suggest(
            &entry("Screenshot 2026-09-02 at 10.14.33.png"),
            "Pictures",
            false
        )
        .is_none());
        assert!(suggest(&entry("IMG_4821.heic"), "Pictures", false).is_none());
    }

    #[test]
    fn an_empty_name_is_improved_anywhere_using_the_modified_date() {
        let name = suggest(&entry("download (3).pdf"), "Documents", false).unwrap();
        assert!(name.starts_with("Documents 2026-09-0"), "{name}");
        assert!(name.ends_with(".pdf"));
    }

    #[test]
    fn a_clear_name_gets_no_suggestion() {
        assert!(suggest(&entry("Invoice_Aug2026.pdf"), "Invoices", true).is_none());
    }

    #[test]
    fn a_suggestion_always_passes_rename_validation() {
        for label in ["../../etc", ".hidden", "a/b", "", "v1..2", "Tax: 2026"] {
            let name = suggest(&entry("download.pdf"), label, false).unwrap();
            assert!(
                crate::plan::validate_rename(&name).is_ok(),
                "{label} → {name}"
            );
            assert!(!name.contains('/'));
        }
    }
}
