/**
 * Copy the Geist variable font out of the `geist` package into `src/styles/fonts/`.
 *
 * The package only declares Next.js font-loader entry points in its `exports` field,
 * so Vite cannot resolve the .woff2 directly. Copying keeps `geist` as the pinned,
 * updatable source of truth while keeping font binaries out of git.
 */

import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const from = join(root, "node_modules/geist/dist/fonts/geist-sans/Geist-Variable.woff2");
const to = join(root, "src/styles/fonts/Geist-Variable.woff2");

mkdirSync(dirname(to), { recursive: true });
copyFileSync(from, to);
console.log("geist → src/styles/fonts/Geist-Variable.woff2");
