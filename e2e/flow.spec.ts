/**
 * The run, end to end, through the real window against a stubbed backend.
 *
 * The Rust side has its own tests on real files. What only this can check is the
 * seam: that the window sends Rust exactly what the user chose — the right folders,
 * the right argument names, the duplicate decisions they made — and nothing it did
 * not choose. Two properties matter above the rest, as they did for the first review
 * screen: nothing is applied until the user asks, and what is applied is what was on
 * screen.
 */

import { callsTo, expect, test } from "./fixtures";
import type { Plan } from "../src/lib/types";

const HOME = "/Users/ada";

async function toShape(app: import("@playwright/test").Page) {
  await app.getByRole("button", { name: "Let's tidy up" }).click();
  await app.getByRole("button", { name: "Continue" }).click();
}

test("home greets by the saved name and counts the chosen set", async ({ app }) => {
  await expect(app.getByRole("heading", { level: 1 })).toContainText("Ada");
  // Everyday = Downloads (4) + Desktop (1), picked on arrival.
  await expect(app.getByText("5 files are waiting in 2 folders")).toBeVisible();
  // Once in production; React's StrictMode runs effects twice in development.
  expect((await callsTo(app, "get_state")).length).toBeGreaterThanOrEqual(1);
});

test("planning sends exactly the chosen folders, and applies nothing", async ({ app }) => {
  await toShape(app);
  await app.getByRole("button", { name: "Move", exact: true }).click();

  const [args] = await callsTo(app, "plan_run");
  expect(args).toEqual({
    sources: [`${HOME}/Downloads`, `${HOME}/Desktop`],
    destinations: ["documents", "images"],
    setLabel: "Everyday",
  });
  // The duplicate question comes before anything moves.
  await expect(app.getByRole("dialog", { name: /already filed/ })).toBeVisible();
  expect(await callsTo(app, "apply_plan")).toHaveLength(0);
});

test("an identical copy is asked about, defaults to keep, and a name clash never is", async ({ app }) => {
  await toShape(app);
  await app.getByRole("button", { name: "Move", exact: true }).click();

  const sheet = app.getByRole("dialog", { name: /already filed/ });
  await expect(sheet.getByText("1 file is already filed")).toBeVisible();
  await expect(sheet.getByText("Already in Documents as Invoice.pdf")).toBeVisible();
  // Keep is the default — the one choice that can never lose anything.
  await expect(sheet.getByRole("button", { name: "Keep", exact: true })).toHaveAttribute("aria-pressed", "true");
  // report.pdf only shares a name: explained, never offered for deletion.
  await expect(sheet.getByText(/1 other file shares a name/)).toBeVisible();
  await expect(sheet.getByText("report.pdf", { exact: true })).toHaveCount(0);
});

test("what is applied is what was on screen, duplicate choice included", async ({ app }) => {
  await toShape(app);
  await app.getByRole("button", { name: "Move", exact: true }).click();

  await app.getByRole("button", { name: "Move to Trash" }).click();
  await app.getByRole("button", { name: "Continue" }).click();

  // The preview shows the rename and the duplicate headed for the Trash.
  const preview = app.getByRole("dialog", { name: /have a place/ });
  await expect(preview.getByText(/1 gets a clearer name/)).toBeVisible();
  await expect(preview.getByText(/1 duplicate goes to the Trash/)).toBeVisible();
  await preview.getByRole("button", { name: /Pictures\/Screenshots/ }).click();
  await expect(preview.getByText("Screenshots 2026-09-02 10.14.33.png")).toBeVisible();

  await preview.getByRole("button", { name: /^Move \d+$/ }).click();

  const [args] = await callsTo(app, "apply_plan");
  const sent = args.plan as Plan;
  const byId = Object.fromEntries(sent.entries.map((e) => [e.id, e]));
  expect(byId.f_002.on_duplicate).toBe("trash");
  expect(byId.f_003.on_duplicate).toBeNull();
  expect(byId.f_001.rename_to).toBe("Screenshots 2026-09-02 10.14.33.png");
  expect(byId.f_004.included).toBe(false);
});

test("progress drives the Move stage and the receipt reads the journal", async ({ app }) => {
  await toShape(app);
  await app.getByRole("button", { name: "Move", exact: true }).click();
  await app.getByRole("button", { name: "Continue" }).click();
  await app.getByRole("dialog", { name: /have a place/ }).getByRole("button", { name: /^Move \d+$/ }).click();

  await expect(app.getByRole("heading", { name: "3 files filed" })).toBeVisible();
  await expect(app.getByText(/1 duplicate in the Trash/)).toBeVisible();
  await expect(app.getByRole("list").getByText("Documents/Contracts")).toBeVisible();
  expect(await callsTo(app, "get_run")).toContainEqual({ batchId: "run-1" });
});

test("a rule written in Shape is saved against that folder's key", async ({ app }) => {
  await toShape(app);
  // Off-screen stages are inert, so this can only be Shape's card, not Choose's row.
  await app.getByRole("button", { name: /^Documents.*subfolder/ }).click();
  await app.getByRole("button", { name: "+ Add rule" }).first().click();
  await app.getByPlaceholder(/Add a rule/).fill("Keep tax forms and receipts together");
  await app.getByRole("button", { name: "Add rule", exact: true }).click();

  expect(await callsTo(app, "add_folder_rule")).toContainEqual({
    destinationKey: "documents",
    text: "Keep tax forms and receipts together",
  });
});

test("Add folder opens the picker and adds any folder as a source", async ({ app }) => {
  await app.getByRole("button", { name: "Let's tidy up" }).click();
  await app.getByRole("button", { name: /Add folder/ }).first().click();

  expect(await callsTo(app, "add_source")).toEqual([{ path: `${HOME}/Projects/old-exports` }]);
});

test("stages that have slid away cannot be reached by keyboard or screen reader", async ({ app }) => {
  await toShape(app);
  // Choose's rows are still mounted, but not reachable.
  await expect(app.getByRole("button", { name: /Downloads/ })).toHaveCount(0);
  await expect(app.getByRole("heading", { name: /Pick the folders/ })).toBeVisible();
});

test("picking a profile picture saves it and shows it in the corner", async ({ app }) => {
  await app.getByRole("button", { name: "Menu" }).click();
  await app.getByRole("menuitem", { name: /Settings/ }).click();
  await app.getByRole("button", { name: "Spinelli" }).click();

  expect(await callsTo(app, "set_avatar")).toEqual([{ id: "spinelli" }]);
  await expect(app.getByRole("button", { name: "Spinelli" })).toHaveAttribute("aria-pressed", "true");

  // The corner shows that same picture. Compared by source rather than by filename:
  // the build inlines a small SVG as a data URI, so there is no name to match on.
  const picked = await app.getByRole("button", { name: "Spinelli" }).locator("img").getAttribute("src");
  await expect(app.getByRole("button", { name: "Menu" }).locator("img")).toHaveAttribute("src", picked!);
});

test("a changed setting is saved, with the rest left as they were", async ({ app }) => {
  await app.getByRole("button", { name: "Menu" }).click();
  await app.getByRole("menuitem", { name: /Settings/ }).click();
  await app.getByRole("switch", { name: "Include hidden files" }).click();

  const [args] = await callsTo(app, "save_settings");
  expect(args.settings).toMatchObject({ include_hidden: true, rename_unclear: true, duplicates: "ask" });
});
