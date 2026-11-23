import { readFileSync } from "node:fs";
import { join } from "node:path";

// The bundle's two word packs, read as the build writes them into one object, and the two ways the tests look
// into them: a key read down its dotted path, and every key of a pack listed.

const ROOT = join(import.meta.dirname, "..");

export type Pack = Record<string, unknown>;

function pack(locale: string): Pack {
  return JSON.parse(readFileSync(join(ROOT, "i18n", `${locale}.json`), "utf8")) as Pack;
}

export const EN = pack("en");
export const KO = pack("ko");

/** The value a key of the pack holds, read down its dotted path, or nothing where no such key stands. */
export function word(within: Pack, key: string): unknown {
  return key.split(".").reduce<unknown>((held, part) => (held !== null && typeof held === "object" ? (held as Pack)[part] : undefined), within);
}

function every(within: Pack, road: string[], into: string[]): string[] {
  for (const [name, value] of Object.entries(within)) {
    const at = [...road, name];

    if (typeof value === "string") into.push(at.join("."));
    else if (value !== null && typeof value === "object") every(value as Pack, at, into);
  }

  return into;
}

/** Every key of a pack, as a dotted path. */
export const keysOf = (within: Pack): string[] => every(within, [], []);
