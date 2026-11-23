import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { basename, join } from "node:path";

import { parse } from "vue/compiler-sfc";

import { WORDED_CODES } from "../app/domain/codes";
import { KINDS, SIDES, STATES } from "../shared/types";
import { EN, KO, PACKS, keysOf, word } from "./words";

// The bundle as the contract holds it (bmo-ui's `docs/console-bundles.md`): the words answer in every locale with
// the same keys under the bundle's own namespace, every key a screen writes or builds from a value stands in every
// pack, every code of the API has its words, the notices the service declares have theirs, and no Korean stands
// in the sources themselves. The manifest is the service's, read through bmo-ui's own reading by the service's tests.

const ROOT = join(import.meta.dirname, "..");

/** Every source file the bundle's build reads. */
function sources(folder: string, extension: string): string[] {
  return readdirSync(join(ROOT, folder), { recursive: true })
    .map(String)
    .filter((file) => file.endsWith(extension))
    .map((file) => join(ROOT, folder, file));
}

/** The section of `docs/api.md` under a heading, to its next heading of the same level. */
function apiSection(heading: string): string {
  const api = readFileSync(join(ROOT, "..", "docs", "api.md"), "utf8");
  const from = api.indexOf(`## ${heading}`);

  return api.slice(from, api.indexOf("\n## ", from + 1));
}

test("every word pack holds the same keys, all under the bundle's name", () => {
  const en = keysOf(EN);

  expect(Object.keys(PACKS).sort()).toEqual(["en", "ja", "ko", "zh-CN"]);
  expect(en.length).toBeGreaterThan(60);

  for (const [locale, pack] of Object.entries(PACKS)) expect(keysOf(pack).sort(), locale).toEqual(en.sort());
  expect(en.every((key) => key.startsWith("voc."))).toBe(true);
});

test("every pack's word fills the same placeholders as the English one", () => {
  const placeholders = (said: unknown): string[] => [...String(said).matchAll(/\{([a-zA-Z_]+)\}/g)].map((match) => match[1]!).sort();

  for (const [locale, pack] of Object.entries(PACKS)) {
    for (const key of keysOf(EN)) expect(placeholders(word(pack, key)), `${locale} ${key}`).toEqual(placeholders(word(EN, key)));
  }
});

test("the title key the manifest names stands in every pack", () => {
  expect(word(EN, "voc.title")).toBe("Customer voice");

  for (const [locale, pack] of Object.entries(PACKS)) expect(word(pack, "voc.title"), locale).toBeString();
});

test("the words the screens build from a value stand in every pack", () => {
  const built = [
    ...KINDS.map((kind) => `voc.kind.${kind}`),
    ...STATES.map((state) => `voc.state.${state}`),
    ...SIDES.map((side) => `voc.side.${side}`),
    ...WORDED_CODES.map((code) => `voc.code.${code}`)
  ];

  for (const key of built) {
    for (const pack of Object.values(PACKS)) expect(word(pack, key), key).toBeString();
  }
});

test("every error code of the API a screen can meet has its words", () => {
  const section = apiSection("Codes");
  const codes = [...section.matchAll(/^\| `([a-z-]+)` \|/gm)].map((match) => match[1]!);
  // The common codes the section names before its table.
  const common = [...section.slice(0, section.indexOf("\n|")).matchAll(/`([a-z-]+)`/g)].map((match) => match[1]!);

  expect(codes.length).toBeGreaterThan(4);

  for (const code of new Set([...codes, ...common])) expect(WORDED_CODES as readonly string[], code).toContain(code);
});

test("every notice kind the API names rings in words of every pack", () => {
  const console = apiSection("Console");
  const kinds = [...console.slice(console.indexOf("notice kinds")).matchAll(/`(report\.[a-z]+)`/g)].map((match) => match[1]!);

  expect(new Set(kinds).size).toBe(3);

  for (const kind of kinds) {
    for (const pack of Object.values(PACKS)) expect(word(pack, `voc.notice.${kind}.title`), kind).toBeString();
  }
});

test("every key a source writes has a Korean word", () => {
  const written = new Set<string>();

  for (const file of [...sources("app", ".vue"), ...sources("app", ".ts"), ...sources("shared", ".ts")]) {
    for (const match of readFileSync(file, "utf8").matchAll(/["'`](voc\.[a-zA-Z0-9_.-]+)["'`]/g)) written.add(match[1]!);
  }

  expect(written.size).toBeGreaterThan(40);

  for (const key of written) {
    const value = word(KO, key);

    expect(value, `${key} has no Korean word`).toBeString();
    expect((value as string).trim()).not.toBe("");
  }
});

test("every value is chosen in the layer's one select, never the library's bare", () => {
  // The layer's select applies the one rule for when a list searches (bmo-ui's `docs/components.md`, "Choosing a
  // value"); the library's own selects would leave it to each screen.
  const BARE = new Set(["USelect", "USelectMenu", "UInputMenu"]);
  const tags = (node: { tag?: string; children?: unknown }): string[] => [
    ...(node.tag === undefined ? [] : [node.tag]),
    ...(Array.isArray(node.children) ? node.children.flatMap((child) => tags(child as { tag?: string; children?: unknown })) : [])
  ];

  for (const file of sources("app", ".vue")) {
    const template = parse(readFileSync(file, "utf8"), { filename: file }).descriptor.template?.ast;
    const stood = template === undefined || template === null ? [] : tags(template);

    expect(stood.filter((tag) => BARE.has(tag)), `${basename(file)} stands a bare library select`).toEqual([]);
  }
});

test("no Korean stands in the sources", () => {
  // The Hangul blocks of Unicode, put together from their code points so this file does not hold one either.
  const BLOCKS: [number, number][] = [
    [0x1100, 0x11ff],
    [0x3130, 0x318f],
    [0xa960, 0xa97f],
    [0xac00, 0xd7af],
    [0xd7b0, 0xd7ff],
    [0xffa0, 0xffdc]
  ];
  const HANGUL = new RegExp(`[${BLOCKS.map(([first, last]) => `\\u${first.toString(16).padStart(4, "0")}-\\u${last.toString(16).padStart(4, "0")}`).join("")}]`);

  for (const file of [...sources("app", ".vue"), ...sources("app", ".ts"), ...sources("shared", ".ts"), ...sources("console", ".ts"), ...sources("tests", ".ts")]) {
    expect(readFileSync(file, "utf8"), `${basename(file)} carries Korean`).not.toMatch(HANGUL);
  }
});
