/**
 * The plan-review flow (§1.3): the screen the whole product rests on.
 *
 * These assert the two things that make the screen trustworthy — that it shows the user
 * exactly what will happen, and that what it sends to Rust on approval is exactly what
 * was on screen.
 */

import { expect, test } from "./fixtures";
import type { Plan } from "../src/lib/types";

test.beforeEach(async ({ app }) => {
  await app.getByRole("button", { name: "Scan folder" }).first().click();
  await expect(app.getByText("Invoice_Aug2026.pdf")).toBeVisible();
});

test("nothing is applied until the user asks", async ({ app }) => {
  const calls = await app.evaluate(() => window.__calls.map((c) => c.cmd));
  expect(calls).toContain("scan_and_plan");
  expect(calls).not.toContain("apply_plan");
});

test("shows every file with its reason, confidence and destination", async ({ app }) => {
  const row = app.locator("li", { hasText: "Invoice_Aug2026.pdf" });
  await expect(row).toContainText("Learned from an earlier approval");
  await expect(row).toContainText("1.00");
  await expect(row.getByRole("combobox")).toHaveValue("finance");
  await expect(row.getByText("learned", { exact: true })).toBeVisible();
});

test("counts reconcile with the rows", async ({ app }) => {
  await expect(app.getByText("4", { exact: true }).first()).toBeVisible();
  await expect(app.getByRole("button", { name: /^All 4$/ })).toBeVisible();
  await expect(app.getByRole("button", { name: /^Filing 3$/ })).toBeVisible();
  await expect(app.getByRole("button", { name: /^Review 1$/ })).toBeVisible();
  await expect(app.getByRole("button", { name: /File 3 files/ })).toBeVisible();
});

test("a file needing review is excluded and cannot be included blind", async ({ app }) => {
  const row = app.locator("li", { hasText: "Q3_forecast.xlsx" });
  const checkbox = row.getByRole("checkbox");
  await expect(checkbox).not.toBeChecked();
  // Nothing without a destination can be included; the toggle would be a lie.
  await expect(checkbox).toBeDisabled();
  await expect(row).toContainText("No rule matched");
});

test("excluding a file updates the counts and the destination tile", async ({ app }) => {
  const tile = app.getByRole("button", { name: /Images/ });
  await expect(tile).toContainText("1 file");

  await app.locator("li", { hasText: "IMG_4821.heic" }).getByRole("checkbox").uncheck();

  await expect(app.getByRole("button", { name: /File 2 files/ })).toBeVisible();
  await expect(tile).toContainText("nothing yet");
});

test("choosing a destination for a reviewed file makes it actionable", async ({ app }) => {
  const row = app.locator("li", { hasText: "Q3_forecast.xlsx" });
  await row.getByRole("combobox").selectOption("documents");

  await expect(row.getByRole("checkbox")).toBeChecked();
  await expect(row).toContainText("You chose this.");
  await expect(app.getByRole("button", { name: /File 4 files/ })).toBeVisible();
});

test("filters narrow the list without changing the plan", async ({ app }) => {
  await app.getByRole("button", { name: /^Review 1$/ }).click();
  await expect(app.locator("li")).toHaveCount(1);
  await expect(app.getByText("Q3_forecast.xlsx")).toBeVisible();

  await app.getByRole("button", { name: /^All 4$/ }).click();
  await expect(app.locator("li")).toHaveCount(4);
});

test("exclude all applies only to the visible filter", async ({ app }) => {
  await app.getByRole("button", { name: /^Filing 3$/ }).click();
  await app.getByRole("button", { name: "Exclude all" }).click();
  await expect(app.getByRole("button", { name: /File 0 files/ })).toBeDisabled();
});

test("apply sends exactly what is on screen, and the overrides with it", async ({ app }) => {
  // One exclusion and one manual override — the two edits that must survive.
  await app.locator("li", { hasText: "annual_report.pdf" }).getByRole("checkbox").uncheck();
  await app
    .locator("li", { hasText: "Q3_forecast.xlsx" })
    .getByRole("combobox")
    .selectOption("finance");

  await app.getByRole("button", { name: /File 3 files/ }).click();
  await expect(app.getByText(/Filed/)).toBeVisible({ timeout: 10_000 });

  const sent = await app.evaluate(
    () => window.__calls.find((c) => c.cmd === "apply_plan")?.args.plan as Plan,
  );

  const byId = Object.fromEntries(sent.entries.map((e) => [e.id, e]));
  expect(byId.f_001.included).toBe(false);
  expect(byId.f_003.destination_key).toBe("finance");
  // The override flag is what stops Menlo learning a rule from a corrected guess.
  expect(byId.f_003.overridden).toBe(true);
  expect(byId.f_000.overridden).toBe(false);
  expect(sent.entries.filter((e) => e.included && e.action === "move")).toHaveLength(3);
});

test("the outcome offers an immediate undo", async ({ app }) => {
  await app.getByRole("button", { name: /File 3 files/ }).click();
  await expect(app.getByRole("button", { name: "Undo this batch" })).toBeVisible({
    timeout: 10_000,
  });
});
