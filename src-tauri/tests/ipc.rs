//! The IPC contract, end to end, on Tauri's mock runtime.
//!
//! Each call below is the JSON the window sends — the same bodies `src/lib/ipc.ts`
//! builds and `e2e/flow.spec.ts` asserts — handed to the real registered commands
//! (`menlo_lib::with_commands`), which then touch real files in a sandboxed home.
//!
//! This is the seam neither side's tests can see alone: the frontend's e2e proves what
//! the window sends, the unit tests prove what the logic does, and only this proves
//! that one arrives as the other. It also checks the negative — that the wrong casing
//! is refused — so the mapping is demonstrated rather than assumed.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, INVOKE_KEY};
use tauri::webview::InvokeRequest;

struct Sandbox(PathBuf);

impl Sandbox {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!("menlo-ipc-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&p).unwrap();
        Sandbox(p.canonicalize().unwrap())
    }
    fn join(&self, rel: &str) -> PathBuf {
        self.0.join(rel)
    }
    fn write(&self, rel: &str, bytes: &[u8]) {
        let p = self.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    }
    fn s(&self, rel: &str) -> String {
        self.join(rel).display().to_string()
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn invoke(
    webview: &tauri::WebviewWindow<tauri::test::MockRuntime>,
    cmd: &str,
    body: Value,
) -> Result<Value, Value> {
    get_ipc_response(
        webview,
        InvokeRequest {
            cmd: cmd.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: "tauri://localhost".parse().unwrap(),
            body: InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        },
    )
    .map(|b| b.deserialize::<Value>().unwrap())
}

fn keys(v: &Value, field: &str) -> Vec<String> {
    v[field]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["key"].as_str().unwrap().to_string())
        .collect()
}

fn exists(p: &Path) -> bool {
    p.exists()
}

/// One test, in order, because HOME and the app directory are process-global and a
/// run is a sequence anyway.
#[test]
fn the_window_and_the_commands_agree() {
    let home = Sandbox::new();
    // A home with the usual folders, a subfolder or two, and a pile in Downloads.
    for d in [
        "Downloads",
        "Desktop",
        "Documents/Contracts",
        "Pictures/Screenshots",
    ] {
        std::fs::create_dir_all(home.join(d)).unwrap();
    }
    home.write("Downloads/Lease_Agreement_signed.pdf", b"lease");
    home.write("Downloads/Screenshot 2026-09-02 at 10.14.33.png", b"pixels");
    home.write("Downloads/Invoice.pdf", b"invoice bytes");
    home.write("Documents/Invoice.pdf", b"invoice bytes"); // already filed, byte for byte
    home.write("Desktop/notes.xyz", b"?");

    std::env::set_var("HOME", &home.0);
    std::env::set_var("MENLO_APP_DIR", home.join("app"));
    std::env::set_var("MENLO_TRASH_DIR", home.join(".Trash"));

    let app = menlo_lib::with_commands(mock_builder())
        .build(mock_context(noop_assets()))
        .expect("mock app");
    let w = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    // ── First launch seeds from the folders that exist ──────────────────────
    let view = invoke(&w, "get_state", json!({})).unwrap();
    let dests = keys(&view, "destinations");
    assert!(dests.contains(&"documents".into()), "{dests:?}");
    assert!(dests.contains(&"documents-contracts".into()));
    assert!(dests.contains(&"images-screenshots".into()));
    assert_eq!(view["folder_sets"][0]["label"], "Everyday");
    assert_eq!(view["sources"].as_array().unwrap().len(), 2);

    // ── camelCase in, as Tauri maps it; snake_case refused ──────────────────
    let rule = json!({ "destinationKey": "documents-contracts",
                       "text": "Anything with the word lease or agreement goes here" });
    let view = invoke(&w, "add_folder_rule", rule).unwrap();
    assert_eq!(view["rules"][0]["understood"], true);
    assert_eq!(
        view["rules"][0]["reading"],
        "Takes names with “lease” or “agreement”"
    );

    let wrong = json!({ "destination_key": "documents", "text": "PDFs go here" });
    assert!(
        invoke(&w, "add_folder_rule", wrong).is_err(),
        "snake_case arguments must not be accepted — the window sends camelCase"
    );

    // ── Planning only accepts folders the user has added ────────────────────
    let stranger = json!({ "sources": ["/etc"], "destinations": ["documents"], "setLabel": null });
    let err = invoke(&w, "plan_run", stranger).unwrap_err();
    assert_eq!(err["kind"], "safety");

    let plan = invoke(
        &w,
        "plan_run",
        json!({
            "sources": [home.s("Downloads"), home.s("Desktop")],
            "destinations": ["documents", "images"],
            "setLabel": "Everyday",
        }),
    )
    .unwrap();

    let entry = |name: &str| -> Value {
        plan["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["name"] == name)
            .cloned()
            .unwrap()
    };
    assert_eq!(
        entry("Lease_Agreement_signed.pdf")["destination_key"],
        "documents-contracts"
    );
    let shot = entry("Screenshot 2026-09-02 at 10.14.33.png");
    assert_eq!(shot["destination_key"], "images-screenshots");
    assert_eq!(shot["rename_to"], "Screenshots 2026-09-02 10.14.33.png");
    assert_eq!(entry("Invoice.pdf")["duplicate"]["kind"], "identical");
    assert_eq!(entry("notes.xyz")["action"], "needs_review");

    // ── Apply what the user decided, as the window sends it ─────────────────
    let mut decided = plan.clone();
    for e in decided["entries"].as_array_mut().unwrap() {
        if e["name"] == "Invoice.pdf" {
            e["on_duplicate"] = json!("trash");
        }
    }
    let outcome = invoke(&w, "apply_plan", json!({ "plan": decided })).unwrap();
    assert_eq!(outcome["moved"], 2);
    assert_eq!(outcome["trashed"], 1);
    assert_eq!(outcome["stopped"], false);

    assert!(exists(
        &home.join("Documents/Contracts/Lease_Agreement_signed.pdf")
    ));
    assert!(exists(&home.join(
        "Pictures/Screenshots/Screenshots 2026-09-02 10.14.33.png"
    )));
    assert!(exists(&home.join(".Trash/Invoice.pdf")));
    assert!(
        exists(&home.join("Documents/Invoice.pdf")),
        "the filed copy is untouched"
    );
    assert!(
        exists(&home.join("Desktop/notes.xyz")),
        "unmatched files stay put"
    );

    // ── Runs reads it back, and restores one row of it ──────────────────────
    let batch = plan["batch_id"].as_str().unwrap();
    let run = invoke(&w, "get_run", json!({ "batchId": batch })).unwrap();
    let labels: Vec<&str> = run["groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["label"].as_str().unwrap())
        .collect();
    assert!(labels.contains(&"Documents/Contracts"), "{labels:?}");
    assert!(labels.contains(&"Pictures/Screenshots"));
    assert!(labels.contains(&"Trash"));
    assert_eq!(run["summary"]["set_label"], "Everyday");

    // `key: null` restores what the run sent to the Trash, and only that.
    let run = invoke(
        &w,
        "revert_run_group",
        json!({ "batchId": batch, "key": null }),
    )
    .unwrap();
    assert!(exists(&home.join("Downloads/Invoice.pdf")));
    assert!(
        exists(&home.join("Documents/Contracts/Lease_Agreement_signed.pdf")),
        "other rows untouched"
    );
    let trash_row = run["groups"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["trash"] == true)
        .cloned()
        .unwrap();
    assert_eq!(trash_row["restored"], 1);

    // And the rest of it, with the rename undone.
    invoke(&w, "revert_run", json!({ "batchId": batch })).unwrap();
    assert!(exists(
        &home.join("Downloads/Screenshot 2026-09-02 at 10.14.33.png")
    ));
    assert!(exists(&home.join("Downloads/Lease_Agreement_signed.pdf")));

    // ── Settings round-trip whole ───────────────────────────────────────────
    let mut settings = view["settings"].clone();
    settings["profile_name"] = json!("Ada");
    settings["duplicates"] = json!("trash");
    let view = invoke(&w, "save_settings", json!({ "settings": settings })).unwrap();
    assert_eq!(view["settings"]["profile_name"], "Ada");
    assert_eq!(view["settings"]["duplicates"], "trash");
    assert_eq!(
        view["settings"]["check_open_files"], true,
        "untouched fields survive"
    );
}
