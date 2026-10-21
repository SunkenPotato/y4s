#!/usr/bin/env node
/**
 * Corner style guard. Enforces the hard rules in DESIGN.md that a linter can check.
 * Scans src/**\/*.{css,ts,tsx} and exits 1 with file:line for every hit.
 *
 *  - no linear/radial/conic gradients
 *  - no monospace, and font-family only var(--font-sans), inherit or the next/font variable
 *  - no box-shadow with a blur radius over 4px
 *  - no hex/rgb/hsl color literals outside src/styles/tokens.css
 *  - no text-transform: uppercase
 */
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(fileURLToPath(new URL(".", import.meta.url)), "..");
const srcDir = join(root, "src");
const TOKENS = join("src", "styles", "tokens.css");
const FONT_VARIABLE = "var(--font-schibsted)";
const ALLOWED_FONT_FAMILIES = new Set(["var(--font-sans)", "inherit", FONT_VARIABLE]);

function walk(dir) {
  const out = [];
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    if (statSync(full).isDirectory()) out.push(...walk(full));
    else if (/\.(css|tsx?|mts)$/.test(name)) out.push(full);
  }
  return out;
}

const hits = [];
const report = (file, line, rule, text) =>
  hits.push(`${relative(root, file).split(sep).join("/")}:${line}  ${rule}  ${text.trim().slice(0, 120)}`);

/** Largest blur radius in a box-shadow value, in px. */
function maxBlur(value) {
  let max = 0;
  for (const shadow of value.split(/,(?![^(]*\))/)) {
    const lengths = shadow
      .replace(/var\([^)]*\)/g, " ")
      .replace(/\b(inset|none|initial|inherit|unset)\b/g, " ")
      .match(/-?\d*\.?\d+(px|rem|em)?\b/g);
    if (!lengths || lengths.length < 3) continue;
    const blur = lengths[2];
    const n = parseFloat(blur);
    const px = blur.endsWith("rem") || blur.endsWith("em") ? n * 16 : n;
    max = Math.max(max, px);
  }
  return max;
}

for (const file of walk(srcDir)) {
  const rel = relative(root, file);
  const isTokens = rel === TOKENS;
  const text = readFileSync(file, "utf8");
  const lines = text.split("\n");

  lines.forEach((line, i) => {
    const n = i + 1;
    if (/(repeating-)?(linear|radial|conic)-gradient/i.test(line)) report(file, n, "gradient", line);
    if (/\b(ui-)?monospace\b/i.test(line)) report(file, n, "monospace", line);
    if (/text-transform\s*:\s*uppercase|textTransform\s*:\s*["'`]uppercase/i.test(line))
      report(file, n, "uppercase", line);

    for (const m of line.matchAll(/font-family\s*:\s*([^;}]+)/gi)) {
      const v = m[1].trim().replace(/\s*!important$/, "");
      if (!ALLOWED_FONT_FAMILIES.has(v)) report(file, n, "font-family", line);
    }
    for (const m of line.matchAll(/fontFamily\s*:\s*["'`]([^"'`]+)["'`]/g)) {
      if (!ALLOWED_FONT_FAMILIES.has(m[1].trim())) report(file, n, "font-family", line);
    }

    if (!isTokens) {
      if (/(^|[^&\w])#(?:[0-9a-fA-F]{3,4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})\b/.test(line))
        report(file, n, "color-literal", line);
      if (/\b(rgba?|hsla?)\s*\(/i.test(line)) report(file, n, "color-literal", line);
    }
  });

  // box-shadow values can span lines; read until the end of the declaration.
  for (const m of text.matchAll(/(box-shadow\s*:|boxShadow\s*:)\s*([^;}]+)/gi)) {
    if (maxBlur(m[2]) > 4) {
      const n = text.slice(0, m.index).split("\n").length;
      report(file, n, "box-shadow-blur", m[0]);
    }
  }
}

if (hits.length) {
  console.error(`style-guard: ${hits.length} problem${hits.length === 1 ? "" : "s"}`);
  for (const h of hits) console.error("  " + h);
  process.exit(1);
}
console.log("style-guard: 0 problems");
