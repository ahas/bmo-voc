import { afterAll, beforeAll, expect, test } from "bun:test";
import { ofetch } from "ofetch";

import { Refusal, Unreachable, attachmentPath, createVocApi } from "../app/domain/api";

import type { Call } from "../app/domain/api";

// The screens' calls over a stand-in of the voc service on a loopback port: each call at the path, method, query and
// body the service's API writes, a report sent as its multipart parts, a refusal carrying its code, and a service
// not there to ask — a 503 or no answer — told apart from a refusal.

interface Asked {
  method: string;
  path: string;
  query: Record<string, string>;
  type: string;
  body: unknown;
}

const asked: Asked[] = [];
let server: ReturnType<typeof Bun.serve>;
let call: Call;

/** What a request carried: its multipart parts by name, its JSON, or nothing. */
async function bodyOf(request: Request): Promise<unknown> {
  const type = request.headers.get("content-type") ?? "";

  if (type.startsWith("multipart/form-data")) {
    const form = await request.formData();
    const files = form.getAll("files") as File[];

    return {
      report: JSON.parse(String(form.get("report"))) as unknown,
      files: await Promise.all(files.map(async (file) => ({ name: file.name, type: file.type, text: await file.text() })))
    };
  }

  return request.method === "GET" ? null : await request.json().catch(() => null);
}

beforeAll(() => {
  server = Bun.serve({
    port: 0,
    hostname: "127.0.0.1",
    async fetch(request) {
      const url = new URL(request.url);

      asked.push({ method: request.method, path: url.pathname, query: Object.fromEntries(url.searchParams), type: request.headers.get("content-type") ?? "", body: await bodyOf(request) });

      if (url.pathname.startsWith("/absent")) return new Response("", { status: 503 });
      if (url.pathname === "/reports/r-9") return Response.json({ code: "not-found", error: "No such report." }, { status: 404 });
      if (url.pathname.endsWith("/seen")) return new Response(null, { status: 204 });
      if (url.pathname === "/companies") return Response.json({ items: [{ id: "c1", name: "First" }], more: false });
      if (url.pathname === "/reports" && request.method === "GET") return Response.json({ items: [], more: false });

      return Response.json({ id: "r-1" }, { status: request.method === "POST" ? 201 : 200 });
    }
  });

  const base = `http://127.0.0.1:${server.port}`;

  call = (path, options) => ofetch(`${base}${path}`, { method: options?.method ?? "GET", body: options?.body as BodyInit | Record<string, unknown> | undefined, query: options?.query });
});

afterAll(() => {
  void server.stop(true);
});

test("each call goes to its route with its method, query and body", async () => {
  const api = createVocApi(call);

  asked.length = 0;
  await api.reports({ kind: "bug", state: "under-review" }, 50, 50);
  await api.reports({}, 0, 50);
  await api.reply("r 1", "Thanks");
  await api.setState("r-1", "done");
  await api.seen("r-1");
  expect(await api.companies()).toEqual([{ id: "c1", name: "First" }]);

  expect(asked.map(({ method, path, query, body }) => ({ method, path, query, body }))).toEqual([
    { method: "GET", path: "/reports", query: { kind: "bug", state: "under-review", after: "50", limit: "50" }, body: null },
    { method: "GET", path: "/reports", query: { after: "0", limit: "50" }, body: null },
    { method: "POST", path: "/reports/r%201/replies", query: {}, body: { body: "Thanks" } },
    { method: "PUT", path: "/reports/r-1/state", query: {}, body: { state: "done" } },
    { method: "PUT", path: "/reports/r-1/seen", query: {}, body: null },
    { method: "GET", path: "/companies", query: {}, body: null }
  ]);
});

test("a report is sent as its JSON part and one part for each file", async () => {
  const api = createVocApi(call);
  const context = { path: "/calendar", consoleVersion: "1.2.3", browser: "Test Browser" };

  asked.length = 0;
  await api.send({ kind: "suggestion", title: "Dark mode", body: "Please", context }, [new File(["png bytes"], "shot.png", { type: "image/png" }), new File(["notes"], "notes.txt", { type: "text/plain" })]);

  expect(asked).toHaveLength(1);
  expect(asked[0]!.method).toBe("POST");
  expect(asked[0]!.path).toBe("/reports");
  expect(asked[0]!.type).toStartWith("multipart/form-data");
  expect(asked[0]!.body).toEqual({
    report: { kind: "suggestion", title: "Dark mode", body: "Please", context },
    files: [
      { name: "shot.png", type: "image/png", text: "png bytes" },
      { name: "notes.txt", type: expect.stringMatching(/^text\/plain/), text: "notes" }
    ]
  });
});

test("a refusal carries its status and code, and a service not there is unreachable", async () => {
  const api = createVocApi(call);
  const absent = createVocApi((path, options) => call(`/absent${path}`, options));
  const nowhere = createVocApi((path) => ofetch(`http://127.0.0.1:1${path}`));

  const refused = await api.report("r-9").catch((error: unknown) => error);

  expect(refused).toBeInstanceOf(Refusal);
  expect((refused as Refusal).status).toBe(404);
  expect((refused as Refusal).code).toBe("not-found");
  expect(await absent.me().catch((error: unknown) => error)).toBeInstanceOf(Unreachable);
  expect(await nowhere.me().catch((error: unknown) => error)).toBeInstanceOf(Unreachable);
});

test("an attachment's path names the report and the attachment, each escaped", () => {
  expect(attachmentPath("r 1", "a/2")).toBe("/reports/r%201/attachments/a%2F2");
});
