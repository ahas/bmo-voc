import { afterAll, afterEach, beforeAll, describe, expect, test } from "bun:test";
import { existsSync } from "node:fs";

import { chromium } from "playwright-core";

import { stoodUp } from "./host";
import { KO, word } from "../words";
import { CONTEXT, MAKER, MAKER_REPLY, SECOND_COMPANY, SENDER, TITLES } from "./stand-in";

import type { Browser, BrowserContext, Locator, Page } from "playwright-core";
import type { Host } from "./host";

// The voc screens as a person reaches them (`docs/design.md`, 「The console」), over the dev host relaying to a
// stand-in of the service, at a desktop's width and a phone's. A sender reads their own reports, the one the maker
// answered marked unread until it is opened, sends a report with a file and lands on its thread, and answers the
// maker's answer with a follow-up; the maker reads every company's reports, filters them by kind, state and company, opens one to read
// where it was sent from and its files, moves its state and answers. At both widths nothing reaches past the screen's
// edge. What is checked is what is drawn and reachable by role and name. Skipped where the browser build is not on
// the machine.

const ready = existsSync(chromium.executablePath());

const HOST_MS = 900_000;
const TEST_MS = 120_000;
// How long the screen is given to follow a press.
const FOLLOWS_MS = 30_000;

const WIDTHS = [
  { name: "wide", viewport: { width: 1280, height: 800 } },
  { name: "phone", viewport: { width: 390, height: 844 } }
] as const;

const say = (key: string, values: Record<string, string> = {}): string =>
  Object.entries(values).reduce((text, [name, value]) => text.replace(`{${name}}`, value), word(KO, key) as string);

let host: Host | null = null;
let browser: Browser;

beforeAll(async () => {
  if (!ready) return;

  host = await stoodUp();
  browser = await chromium.launch();

  // The first pages a dev host serves may reload themselves under the reader while it finds the packages the
  // screens import; each screen is asked for once before the tests read it.
  const page = await browser.newPage();

  for (const path of ["/voc", "/voc/r-1"]) {
    await page.goto(`${host.address}${path}`);
    await page.getByRole("heading", { name: TITLES.answered }).or(page.getByRole("link", { name: new RegExp(TITLES.answered) })).first().waitFor({ timeout: HOST_MS });
  }

  await page.close();
}, HOST_MS);

afterAll(async () => {
  await browser?.close();
  await host?.down();
}, HOST_MS);

// The browser context the running test opened, closed after it however it ended.
let standing: BrowserContext | null = null;

afterEach(async () => {
  await standing?.close();
  standing = null;
});

/** A page of its own at a width, so nothing one test read stands in another's. */
async function opened(viewport: { width: number; height: number }, path: string): Promise<Page> {
  standing = await browser.newContext({ viewport, locale: "ko-KR", timezoneId: "Asia/Seoul" });

  const page = await standing.newPage();

  page.setDefaultTimeout(FOLLOWS_MS);
  await page.goto(`${host!.address}${path}`);

  return page;
}

/** Whether anything on the page reaches past its right edge, so the page itself would scroll sideways. */
async function sideScroll(page: Page): Promise<boolean> {
  return page.evaluate(() => [document.documentElement, document.body].some((element) => element.scrollWidth > element.clientWidth));
}

/** A report's link in a list, by its title. */
const reportLink = (page: Page, title: string): Locator => page.getByRole("link", { name: new RegExp(title) });

/** The list's rows' titles, in the order they stand. */
async function listedTitles(page: Page): Promise<string[]> {
  return page.evaluate((titles) => {
    const text = document.querySelector("main, body")!.textContent ?? "";

    return titles.filter((title) => text.includes(title)).sort((left, right) => text.indexOf(left) - text.indexOf(right));
  }, Object.values(TITLES));
}

/** Chooses a value of one of the inbox's filters, from the row on a wide screen and from the filter button on a phone. */
async function filterBy(page: Page, label: string, choice: string): Promise<void> {
  const select = page.getByLabel(label, { exact: true }).filter({ visible: true }).first();

  if (!(await select.isVisible())) await page.getByRole("button", { name: say("voc.action.filter") }).click();

  await select.click();
  await page.getByRole("option", { name: choice, exact: true }).click();
}

for (const width of WIDTHS) {
  describe.skipIf(!ready)(`a sender on a ${width.name} (${width.viewport.width}px)`, () => {
    test("reads their own reports, newest activity first, the one the maker answered marked unread", async () => {
      host!.service.reset();
      const page = await opened(width.viewport, "/voc");

      await reportLink(page, TITLES.answered).waitFor();

      expect(await listedTitles(page)).toEqual([TITLES.answered, TITLES.done]);
      expect(await reportLink(page, TITLES.answered).getByRole("img", { name: say("voc.label.unread") }).count()).toBe(1);
      expect(await reportLink(page, TITLES.done).getByRole("img", { name: say("voc.label.unread") }).count()).toBe(0);
      expect(await sideScroll(page)).toBe(false);

    }, TEST_MS);

    test("sends a report with a file and lands on its thread", async () => {
      host!.service.reset();
      const page = await opened(width.viewport, "/voc");

      await reportLink(page, TITLES.answered).waitFor();

      if (width.name === "wide") {
        // The person came to the screen from another of the console's, and went into a report and back before writing.
        await page.evaluate((from) => window.history.replaceState({ ...(window.history.state as object), back: from }, ""), CONTEXT.path);
        await page.reload();
        await reportLink(page, TITLES.answered).click();
        await page.getByRole("heading", { name: TITLES.answered }).waitFor();
        await page.goBack();
        await reportLink(page, TITLES.answered).waitFor();
      }

      await page.getByRole("button", { name: say("voc.action.new_report") }).click();

      const dialog = page.getByRole("dialog");

      await dialog.waitFor();
      expect(await sideScroll(page)).toBe(false);

      await dialog.getByRole("radio", { name: say("voc.kind.suggestion") }).click();
      await dialog.getByRole("textbox", { name: new RegExp(say("voc.label.title")) }).fill("Search is slow");
      await dialog.getByRole("textbox", { name: new RegExp(say("voc.label.body")) }).fill("It takes **ten** seconds.\n\n![trace](https://example.invalid/trace.png)");
      await dialog.locator("[data-file-picker]").setInputFiles({ name: "trace.txt", mimeType: "text/plain", buffer: Buffer.from("trace lines") });
      await dialog.getByText("trace.txt").waitFor();
      await dialog.getByRole("button", { name: say("voc.action.send"), exact: true }).click();

      await page.getByRole("heading", { name: "Search is slow" }).waitFor();
      expect(new URL(page.url()).pathname).toBe("/voc/r-4");
      expect(await page.getByRole("link", { name: say("voc.aria.download", { name: "trace.txt" }) }).isVisible()).toBe(true);
      expect(await page.locator("strong", { hasText: "ten" }).isVisible()).toBe(true);
      // A picture written into the body is fetched from nowhere: a report's pictures are its attachments.
      expect(await page.locator('img[src^="https:"]').count()).toBe(0);
      expect(await sideScroll(page)).toBe(false);

      const sent = host!.service.asked.findLast((call) => call.method === "POST" && call.path === "/reports")!.body as { report: { kind: string; title: string; context: typeof CONTEXT }; files: string[] };

      expect(sent.report.kind).toBe("suggestion");
      expect(sent.report.title).toBe("Search is slow");
      expect(sent.files).toEqual(["trace.txt"]);
      expect(sent.report.context.consoleVersion).toBe("dev-host");
      expect(sent.report.context.browser).toContain("Mozilla");

      if (width.name === "wide") expect(sent.report.context.path).toBe(CONTEXT.path);

    }, TEST_MS);

    test("reads the maker's answer on their thread and follows it up, which leaves the report read", async () => {
      host!.service.reset();
      const page = await opened(width.viewport, "/voc/r-1");
      const makerReply = page.getByRole("article", { name: say("voc.aria.answer_by", { name: MAKER.name }) });

      await makerReply.waitFor();
      expect(await makerReply.textContent()).toContain(MAKER_REPLY);
      expect(await makerReply.getByText(say("voc.label.answer"), { exact: true }).isVisible()).toBe(true);
      // Where it was sent from is the maker's to read alone.
      expect(await page.getByRole("heading", { name: say("voc.label.context") }).count()).toBe(0);
      expect(await page.getByLabel(say("voc.aria.state_control"), { exact: true }).count()).toBe(0);

      await page.getByRole("textbox", { name: new RegExp(say("voc.label.follow")) }).fill("It still fails on Fridays.");
      await page.getByRole("button", { name: say("voc.action.follow"), exact: true }).click();
      await page.getByRole("article", { name: say("voc.aria.follow_by", { name: SENDER.name }) }).getByText("It still fails on Fridays.").waitFor();
      expect(await sideScroll(page)).toBe(false);

      await page.getByRole("link", { name: say("voc.action.back") }).click();
      await reportLink(page, TITLES.answered).waitFor();
      expect(await reportLink(page, TITLES.answered).getByRole("img", { name: say("voc.label.unread") }).count()).toBe(0);

    }, TEST_MS);
  });

  describe.skipIf(!ready)(`the maker on a ${width.name} (${width.viewport.width}px)`, () => {
    test("reads every company's reports and filters them by kind, state and company", async () => {
      host!.service.reset();
      host!.service.side = "maker";
      const page = await opened(width.viewport, "/voc");

      await reportLink(page, TITLES.other).waitFor();
      expect((await listedTitles(page)).sort()).toEqual(Object.values(TITLES).sort());
      expect(await page.getByText(SECOND_COMPANY.name).first().isVisible()).toBe(true);
      expect(await sideScroll(page)).toBe(false);

      await filterBy(page, say("voc.label.kind"), say("voc.kind.suggestion"));
      await reportLink(page, TITLES.answered).waitFor({ state: "detached" });
      expect((await listedTitles(page)).sort()).toEqual([TITLES.done, TITLES.other].sort());

      await filterBy(page, say("voc.label.state"), say("voc.state.under-review"));
      await reportLink(page, TITLES.done).waitFor({ state: "detached" });
      expect(await listedTitles(page)).toEqual([TITLES.other]);

      await filterBy(page, say("voc.label.kind"), say("voc.label.all_kinds"));
      await filterBy(page, say("voc.label.state"), say("voc.label.all_states"));
      await reportLink(page, TITLES.answered).waitFor();
      await filterBy(page, say("voc.label.company"), "First Company");
      await reportLink(page, TITLES.other).waitFor({ state: "detached" });
      expect((await listedTitles(page)).sort()).toEqual([TITLES.answered, TITLES.done].sort());
      expect(new URL(page.url()).searchParams.get("company")).toBe("c-1");
      expect(await sideScroll(page)).toBe(false);

    }, TEST_MS);

    test("opens a report, reads where it was sent from and its files, moves its state and answers", async () => {
      host!.service.reset();
      host!.service.side = "maker";
      const page = await opened(width.viewport, "/voc");

      await reportLink(page, TITLES.answered).first().click();
      await page.getByRole("heading", { name: TITLES.answered }).waitFor();

      for (const value of [CONTEXT.path, CONTEXT.consoleVersion, CONTEXT.browser]) expect(await page.getByText(value, { exact: true }).isVisible()).toBe(true);
      expect(await page.getByText(`${SENDER.name} · First Company`).isVisible()).toBe(true);
      expect(await sideScroll(page)).toBe(false);

      await page.getByRole("button", { name: say("voc.aria.open_picture", { name: "shot.png" }) }).click();
      await page.getByRole("dialog", { name: "shot.png" }).waitFor();
      await page.keyboard.press("Escape");
      await page.getByRole("dialog", { name: "shot.png" }).waitFor({ state: "detached" });

      const download = page.getByRole("link", { name: say("voc.aria.download", { name: "notes.txt" }) });
      const answer = await page.request.get(new URL((await download.getAttribute("href"))!, page.url()).href);

      expect(answer.headers()["content-disposition"]).toStartWith("attachment");
      expect(await answer.text()).toBe("what I saw");

      await page.getByLabel(say("voc.aria.state_control"), { exact: true }).click();
      await page.getByRole("option", { name: say("voc.state.done"), exact: true }).click();
      await page.locator("[data-state-chip]", { hasText: say("voc.state.done") }).waitFor();
      expect(await page.locator("#history").getByText(say("voc.state.done"), { exact: true }).isVisible()).toBe(true);
      expect(host!.service.asked.findLast((call) => call.method === "PUT" && call.path === "/reports/r-1/state")?.body).toEqual({ state: "done" });

      await page.getByRole("textbox", { name: new RegExp(say("voc.label.answer")) }).fill("Fixed in the next release.");
      await page.getByRole("button", { name: say("voc.action.answer"), exact: true }).click();
      await page.getByRole("article", { name: say("voc.aria.answer_by", { name: MAKER.name }) }).getByText("Fixed in the next release.").waitFor();
      expect(await sideScroll(page)).toBe(false);

    }, TEST_MS);
  });
}

describe.skipIf(!ready)("a long inbox and a failed read", () => {
  test("the inbox reads a page at a time, the next at its end", async () => {
    host!.service.reset();
    host!.service.side = "maker";
    host!.service.seed(55);

    const page = await opened(WIDTHS[1].viewport, "/voc");

    await page.waitForFunction(() => document.querySelectorAll("[data-report-row]").length === 50);
    await page.getByRole("button", { name: say("voc.action.load_more") }).click();
    await page.waitForFunction(() => document.querySelectorAll("[data-report-row]").length === 58);
    expect(await page.getByRole("button", { name: say("voc.action.load_more") }).count()).toBe(0);

  }, TEST_MS);

  test("a list that could not be read says why in one band, and its refresh reads it again", async () => {
    host!.service.reset();
    host!.service.failing = true;

    const page = await opened(WIDTHS[0].viewport, "/voc");
    const band = page.getByRole("alert").filter({ hasText: say("voc.code.failed") });

    await band.waitFor();
    host!.service.failing = false;
    await band.getByRole("button", { name: say("voc.action.refresh") }).click();
    await reportLink(page, TITLES.answered).waitFor();
    await band.waitFor({ state: "detached" });

  }, TEST_MS);
});
