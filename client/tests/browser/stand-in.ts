import type { Attachment, Company, Context, Kind, Limits, Person, Reply, Report, ReportSummary, Side, State, StateEntry } from "../../shared/types";

// A stand-in of the voc service for the browser tests: every route the screens call, answered from memory as
// `docs/api.md` states them, so a test drives the screens without the service and its peers. The side the reader
// stands on is the test's to set (`side`), whatever role the dev relay names, and the sender is always the same
// account; a sender reads their own reports alone, every other one answered 404. It records every call it takes,
// so a test waits on a call rather than on the clock. `reset` puts the seeded reports back.

export const SENDER: Person = { user: "u-1", name: "Sender One" };
export const OTHER_SENDER: Person = { user: "u-2", name: "Sender Two" };
export const MAKER: Person = { user: "u-9", name: "Admin Kim" };
export const FIRST_COMPANY: Company = { id: "c-1", name: "First Company" };
export const SECOND_COMPANY: Company = { id: "c-2", name: "Second Company" };

/** The seeded reports' titles, by what each one is for. */
export const TITLES = {
  answered: "Save button does nothing",
  other: "Add a dark theme",
  done: "Export the month to a sheet"
};

export const MAKER_REPLY = "We are looking into it.";
export const CONTEXT: Context = { path: "/calendar/2030-05", consoleVersion: "1.4.0", browser: "Test Browser 1.0" };
// What one report may carry, as the service ships its settings.
export const LIMITS: Limits = { maxFiles: 5, maxBytes: 10 * 1024 * 1024 };

// A picture of one pixel, which a browser draws.
const PIXEL = Uint8Array.from(atob("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg=="), (char) => char.charCodeAt(0));
const PICTURE_TYPES = ["image/png", "image/jpeg", "image/gif", "image/webp"];
const START = Date.UTC(2030, 4, 1, 1, 0);

interface Stored {
  report: Report;
  bytes: Map<string, Uint8Array>;
  /** The moment each side last opened it. */
  seen: { maker: number; sender: number };
}

export interface Asked {
  method: string;
  path: string;
  /** The JSON body, or for a report sent its `report` part and its files' names. */
  body: unknown;
}

export interface StandIn {
  address: string;
  /** The side the reader stands on. */
  side: Side;
  asked: Asked[];
  /** Whether the reports' list answers a failure, as a service that broke would. */
  failing: boolean;
  /** Puts the seeded reports back, the side a sender's and nothing failing. */
  reset: () => void;
  /** Adds as many more reports, from another company's sender. */
  seed: (count: number) => void;
  stop: () => void;
}

function json(value: unknown, status = 200): Response {
  return Response.json(value, { status });
}

function refused(status: number, code: string): Response {
  return json({ code, error: `Refused: ${code}.` }, status);
}

export function standIn(): StandIn {
  const reports = new Map<string, Stored>();
  let clock = START;
  let made = 0;

  const tick = (): number => (clock += 60_000);

  function store(kind: Kind, title: string, sender: Person, company: Company | null, files: { name: string; type: string; bytes: Uint8Array }[], context: Context): Stored {
    const id = `r-${++made}`;
    const at = tick();
    const bytes = new Map<string, Uint8Array>();
    const attachments: Attachment[] = files.map((file, index) => {
      const attachmentId = `a-${made}-${index + 1}`;

      bytes.set(attachmentId, file.bytes);

      return { id: attachmentId, name: file.name, type: file.type || "application/octet-stream", size: file.bytes.length };
    });
    const arrival: StateEntry = { state: "received", by: { ...sender, side: "sender" }, at };
    const stored: Stored = {
      report: { id, kind, title, state: "received", sender, company, sentAt: at, activityAt: at, unread: false, body: `Steps:\n\n1. Open the **month**\n2. Press save`, context, attachments, states: [arrival], replies: [] },
      bytes,
      seen: { maker: 0, sender: at }
    };

    reports.set(id, stored);

    return stored;
  }

  function reply(stored: Stored, side: "maker" | "sender", body: string): Reply {
    const at = tick();
    const written: Reply = { id: `p-${stored.report.replies.length + 1}-${stored.report.id}`, side, by: side === "maker" ? MAKER : stored.report.sender, body, at };

    stored.report.replies.push(written);
    stored.report.activityAt = at;

    return written;
  }

  function reset(): void {
    service.side = "sender";
    service.failing = false;
    reports.clear();
    clock = START;
    made = 0;

    const answered = store("bug", TITLES.answered, SENDER, FIRST_COMPANY, [
      { name: "shot.png", type: "image/png", bytes: PIXEL },
      { name: "notes.txt", type: "text/plain", bytes: new TextEncoder().encode("what I saw") }
    ], CONTEXT);

    store("suggestion", TITLES.other, OTHER_SENDER, SECOND_COMPANY, [], CONTEXT);
    reports.get("r-2")!.report.state = "under-review";

    const done = store("suggestion", TITLES.done, SENDER, FIRST_COMPANY, [], CONTEXT);

    done.report.state = "done";
    done.seen.maker = tick();
    reply(answered, "maker", MAKER_REPLY);
  }

  function seed(count: number): void {
    for (let at = 1; at <= count; at++) store("bug", `Seeded report ${at}`, OTHER_SENDER, SECOND_COMPANY, [], CONTEXT);
  }

  const service: StandIn = { address: "", side: "sender", asked: [], failing: false, reset, seed, stop: () => undefined };

  /** Whether the reader's side has not read what the other side did since it last opened the report. */
  function unread(stored: Stored, side: Side): boolean {
    const { report, seen } = stored;

    if (side === "maker") return report.sentAt > seen.maker || report.replies.some((one) => one.side === "sender" && one.at > seen.maker);
    if (side === "sender") return report.replies.some((one) => one.side === "maker" && one.at > seen.sender) || report.states.slice(1).some((entry) => entry.at > seen.sender);

    return false;
  }

  function summary(stored: Stored): ReportSummary {
    const { body: _body, context: _context, attachments: _attachments, states: _states, replies, ...rest } = stored.report;

    return { ...rest, replies: replies.length, unread: unread(stored, service.side) };
  }

  function whole(stored: Stored): Report {
    return { ...structuredClone(stored.report), unread: unread(stored, service.side) };
  }

  /** The report a reader may read, none where it does not stand or is another sender's. */
  function readable(id: string): Stored | undefined {
    const stored = reports.get(decodeURIComponent(id));

    return stored === undefined || (service.side === "sender" && stored.report.sender.user !== SENDER.user) ? undefined : stored;
  }

  async function answer(request: Request, url: URL, entry: Asked): Promise<Response> {
    const path = url.pathname;
    const method = request.method;
    const one = /^\/reports\/([^/]+)(\/.*)?$/.exec(path);

    if (path === "/me" && method === "GET") return json({ user: service.side === "maker" ? MAKER.user : SENDER.user, side: service.side, limits: LIMITS });

    if (path === "/events") {
      // An open stream that says nothing, as the service's does while nothing happens.
      return new Response(new ReadableStream({ start: (controller) => controller.enqueue(new TextEncoder().encode(": open\n\n")) }), { headers: { "content-type": "text/event-stream" } });
    }

    if (path === "/reports" && method === "GET") {
      if (service.failing) return refused(500, "failed");

      const maker = service.side !== "sender";
      const after = Number(url.searchParams.get("after") ?? 0);
      const limit = Math.min(Number(url.searchParams.get("limit") ?? 50), 200);
      const kind = maker ? url.searchParams.get("kind") : null;
      const state = maker ? url.searchParams.get("state") : null;
      const company = maker ? url.searchParams.get("company") : null;
      const rows = [...reports.values()]
        .filter((stored) => maker || stored.report.sender.user === SENDER.user)
        .filter((stored) => (kind === null || stored.report.kind === kind) && (state === null || stored.report.state === state) && (company === null || stored.report.company?.id === company))
        .sort((left, right) => right.report.activityAt - left.report.activityAt);

      return json({ items: rows.slice(after, after + limit).map(summary), more: rows.length > after + limit });
    }

    if (path === "/reports" && method === "POST") {
      if (service.side === "maker") return refused(403, "maker-side");

      const form = await request.formData();
      const written = JSON.parse(String(form.get("report"))) as { kind: Kind; title: string; body: string; context: Context };
      const files = await Promise.all((form.getAll("files") as File[]).map(async (file) => ({ name: file.name, type: file.type, bytes: new Uint8Array(await file.arrayBuffer()) })));

      entry.body = { report: written, files: files.map((file) => file.name) };

      if (written.title.trim() === "" || written.body === "") return refused(422, "bad-report");

      const stored = store(written.kind, written.title.trim(), SENDER, FIRST_COMPANY, files, written.context);

      stored.report.body = written.body;

      return json(whole(stored), 201);
    }

    if (path === "/companies" && method === "GET") {
      if (service.side === "sender") return refused(403, "sender-side");

      const companies = new Map<string, Company>();

      for (const stored of reports.values()) if (stored.report.company !== null) companies.set(stored.report.company.id, stored.report.company);

      return json({ items: [...companies.values()].sort((left, right) => left.name.localeCompare(right.name)), more: false });
    }

    if (path === "/unread" && method === "GET") return json({ count: [...reports.values()].filter((stored) => readable(stored.report.id) !== undefined && unread(stored, service.side)).length });

    const stored = one === null ? undefined : readable(one[1]!);

    if (one === null || stored === undefined) return refused(404, "not-found");

    const rest = one[2] ?? "";
    const attachment = /^\/attachments\/([^/]+)$/.exec(rest);

    if (rest === "" && method === "GET") return json(whole(stored));

    if (attachment !== null && method === "GET") {
      const file = stored.report.attachments.find((held) => held.id === decodeURIComponent(attachment[1]!));

      if (file === undefined) return refused(404, "not-found");

      const inline = PICTURE_TYPES.includes(file.type);

      return new Response(stored.bytes.get(file.id)!, {
        headers: { "content-type": file.type, "content-disposition": `${inline ? "inline" : "attachment"}; filename="${file.name}"`, "x-content-type-options": "nosniff" }
      });
    }

    if (rest === "/replies" && method === "POST") {
      if (service.side === "reader") return refused(403, "forbidden");

      const { body } = (await request.json()) as { body: string };

      return json(reply(stored, service.side === "maker" ? "maker" : "sender", body), 201);
    }

    if (rest === "/state" && method === "PUT") {
      if (service.side !== "maker") return refused(403, "sender-side");

      const { state } = (await request.json()) as { state: State };

      if (state !== stored.report.state) {
        const at = tick();

        stored.report.state = state;
        stored.report.states.push({ state, by: { ...MAKER, side: "maker" }, at });
        stored.report.activityAt = at;
      }

      return json(whole(stored));
    }

    if (rest === "/seen" && method === "PUT") {
      if (service.side === "maker" || service.side === "sender") stored.seen[service.side] = tick();

      return new Response(null, { status: 204 });
    }

    return refused(404, "not-found");
  }

  reset();

  const server = Bun.serve({
    hostname: "127.0.0.1",
    port: 0,
    idleTimeout: 0,
    async fetch(request) {
      const url = new URL(request.url);
      const type = request.headers.get("content-type") ?? "";
      const body = type.includes("application/json") ? ((await request.clone().json().catch(() => null)) as unknown) : null;

      const entry: Asked = { method: request.method, path: url.pathname, body };

      service.asked.push(entry);

      return answer(request, url, entry);
    }
  });

  service.address = `http://127.0.0.1:${server.port}`;
  service.stop = () => void server.stop(true);

  return service;
}
