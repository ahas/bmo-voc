import { expect, test } from "bun:test";

import { cameFromPath, isPicture, oneOf, sizeText } from "../app/domain/reports";
import { STATES } from "../shared/types";

// What the screens read off a report and an address beyond what the service answers.

test("the path a person came from is kept only where it stands outside the screen", () => {
  expect(cameFromPath("/calendar/2030-05", "/voc")).toBe("/calendar/2030-05");
  expect(cameFromPath("/vocabulary", "/voc")).toBe("/vocabulary");
  expect(cameFromPath("/voc", "/voc")).toBeNull();
  expect(cameFromPath("/voc/r-1", "/voc")).toBeNull();
  expect(cameFromPath("/voc?kind=bug", "/voc")).toBeNull();
  expect(cameFromPath(null, "/voc")).toBeNull();
  expect(cameFromPath("", "/voc")).toBeNull();
});

test("a picture is one of the types a browser shows, and no other", () => {
  expect(isPicture({ type: "image/PNG" })).toBe(true);
  expect(isPicture({ type: "image/webp" })).toBe(true);
  expect(isPicture({ type: "image/svg+xml" })).toBe(false);
  expect(isPicture({ type: "application/pdf" })).toBe(false);
});

test("a size stands in its largest whole unit", () => {
  expect(sizeText(512, "en-US")).toBe("512 B");
  expect(sizeText(1536, "en-US")).toBe("1.5 KB");
  expect(sizeText(10 * 1024 * 1024, "en-US")).toBe("10 MB");
  expect(sizeText(1_250_000_000, "en-US")).toBe("1.2 GB");
});

test("a query's word is taken only where it is one of the words", () => {
  expect(oneOf("done", STATES)).toBe("done");
  expect(oneOf("lost", STATES)).toBeUndefined();
  expect(oneOf(["done"], STATES)).toBeUndefined();
});
