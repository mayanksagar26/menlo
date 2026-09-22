/**
 * Every screen, rendered at the design size against the stubbed backend.
 *
 * Two jobs. It is a smoke test — each screen renders, and the page logs no errors
 * getting there. And it leaves a screenshot of each in `test-results/screens/` for a
 * human to look at, which is the only way to review a glass UI.
 */

import { expect, test } from "./fixtures";

const SHOTS = "test-results/screens";

test.use({ viewport: { width: 1040, height: 720 } });

// One long test by design — eleven captures, each after a settle — which runs about
// 18s warm. Against a cold Vite compile it can pass the default 30s, so it gets room.
test.setTimeout(90_000);

test("every screen renders without an error", async ({ app }) => {
  const errors: string[] = [];
  app.on("pageerror", (e) => errors.push(e.message));
  app.on("console", (m) => m.type() === "error" && errors.push(m.text()));
  const shot = (name: string) => app.screenshot({ path: `${SHOTS}/${name}.png` });
  // Let entrances and the stage track finish before each capture.
  const settle = () => app.waitForTimeout(1000);

  await expect(app.getByRole("heading", { level: 1 })).toBeVisible();
  await settle();
  await shot("01-home");

  await app.getByRole("button", { name: "Menu" }).click();
  await settle();
  await shot("02-profile-menu");
  await app.keyboard.press("Escape");
  await app.mouse.click(10, 300);

  await app.getByRole("button", { name: "Let's tidy up" }).click();
  await settle();
  await shot("03-choose");

  await app.getByRole("button", { name: "Continue" }).click();
  await settle();
  await shot("04-shape");

  await app.getByRole("button", { name: /^Documents.*subfolder/ }).click();
  await settle();
  await shot("05-shape-drilldown");
  await app.getByRole("button", { name: "Back to all folders" }).click();
  await settle();

  await app.getByRole("button", { name: "Move", exact: true }).click();
  await expect(app.getByRole("dialog", { name: /already filed/ })).toBeVisible();
  await settle();
  await shot("06-duplicates");

  await app.getByRole("button", { name: "Move to Trash" }).click();
  await app.getByRole("button", { name: "Continue" }).click();
  const preview = app.getByRole("dialog", { name: /have a place/ });
  await preview.getByRole("button", { name: /Pictures\/Screenshots/ }).click();
  await settle();
  await shot("07-preview");

  await preview.getByRole("button", { name: /^Move \d+$/ }).click();
  await expect(app.getByRole("heading", { name: "3 files filed" })).toBeVisible();
  await settle();
  await shot("08-receipt");

  await app.getByRole("button", { name: "Menu" }).click();
  await app.getByRole("menuitem", { name: /Runs/ }).click();
  await expect(app.getByRole("button", { name: "Restore this run" })).toBeVisible();
  await settle();
  await shot("09-runs");
  await app.getByRole("button", { name: "Close" }).click();

  await app.getByRole("button", { name: "Menu" }).click();
  await app.getByRole("menuitem", { name: /Rules memory/ }).click();
  await app.getByRole("button", { name: /Contracts/ }).click();
  await settle();
  await shot("10-rules");
  await app.getByRole("button", { name: "Close" }).click();

  await app.getByRole("button", { name: "Menu" }).click();
  await app.getByRole("menuitem", { name: /Settings/ }).click();
  await settle();
  await shot("11-settings");
  await app.getByRole("button", { name: "Close" }).click();

  await app.getByRole("button", { name: "Menu" }).click();
  await app.getByRole("menuitem", { name: /Folder sets/ }).click();
  await settle();
  await shot("12-folder-sets");

  expect(errors).toEqual([]);
});
