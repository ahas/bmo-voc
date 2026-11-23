// The screens' calls to the voc service, each at the path the service's own API writes it (`docs/api.md`). The caller
// hands the call itself — through the console's relay in the browser (`composables/useVoc.ts`), to a stand-in in a
// test — so this module names no address and no fetcher, and a screen never reaches another unit.

import type { Company, ListPage, Me, NewReport, Reply, Report, ReportFilter, ReportSummary, State } from "../../shared/types";

export type Call = <T>(path: string, options?: { method?: "GET" | "POST" | "PUT"; body?: unknown; query?: Record<string, string> }) => Promise<T>;

/** A call the service answered with a refusal: its status, its code (`docs/api.md`, 「Codes」) and the body it sent. */
export class Refusal extends Error {
  constructor(
    readonly status: number,
    readonly code: string,
    message: string
  ) {
    super(message);
  }
}

/** A call that never reached the voc service: the network, or a 503 saying the service could not be reached. */
export class Unreachable extends Error {}

// What the fetcher threw, sorted into a refusal, which names a fault of the call itself, and a call that never
// landed. A 503 is the service not there to ask, never a fault of the call.
function sorted(error: unknown): Error {
  const failed = error as { response?: { status?: number }; status?: number; statusCode?: number; data?: Record<string, unknown> | null } | null;
  const status = failed?.response?.status ?? failed?.statusCode ?? failed?.status;

  if (typeof status === "number" && status > 0 && status !== 503) {
    const data = failed?.data !== null && typeof failed?.data === "object" ? failed.data : {};

    return new Refusal(status, typeof data.code === "string" ? data.code : "failed", typeof data.error === "string" ? data.error : String(status));
  }

  return new Unreachable(error instanceof Error ? error.message : String(error));
}

const segment = (value: string): string => encodeURIComponent(value);

/** The path of a report's attachment, which a screen hands the relay to make the address a picture or a link opens. */
export function attachmentPath(reportId: string, attachmentId: string): string {
  return `/reports/${segment(reportId)}/attachments/${segment(attachmentId)}`;
}

/** The filters as the service's query names them, none where a filter is left out. */
function filterQuery(filter: ReportFilter): Record<string, string> {
  return Object.fromEntries(Object.entries(filter).filter((entry): entry is [string, string] => typeof entry[1] === "string" && entry[1] !== ""));
}

export interface VocApi {
  me: () => Promise<Me>;
  /** One page of the reports, `after` the rows read so far. */
  reports: (filter: ReportFilter, after: number, limit: number) => Promise<ListPage<ReportSummary>>;
  send: (report: NewReport, files: readonly File[]) => Promise<Report>;
  report: (reportId: string) => Promise<Report>;
  reply: (reportId: string, body: string) => Promise<Reply>;
  setState: (reportId: string, state: State) => Promise<Report>;
  seen: (reportId: string) => Promise<void>;
  companies: () => Promise<Company[]>;
}

/** The API over a call, every failure sorted into a refusal or an unreachable service. */
export function createVocApi(call: Call): VocApi {
  const ask: Call = async <T>(path: string, options?: Parameters<Call>[1]) => {
    try {
      return await call<T>(path, options);
    } catch (error) {
      throw sorted(error);
    }
  };
  const report = (reportId: string): string => `/reports/${segment(reportId)}`;

  return {
    me: () => ask<Me>("/me"),
    reports: (filter, after, limit) => ask<ListPage<ReportSummary>>("/reports", { query: { ...filterQuery(filter), after: String(after), limit: String(limit) } }),
    send: (written, files) => {
      // One part `report`, the JSON of the report, and one part `files` for each attachment, its name and type as
      // the browser gave them.
      const form = new FormData();

      form.append("report", JSON.stringify(written));

      for (const file of files) form.append("files", file, file.name);

      return ask<Report>("/reports", { method: "POST", body: form });
    },
    report: (reportId) => ask<Report>(report(reportId)),
    reply: (reportId, body) => ask<Reply>(`${report(reportId)}/replies`, { method: "POST", body: { body } }),
    setState: (reportId, state) => ask<Report>(`${report(reportId)}/state`, { method: "PUT", body: { state } }),
    seen: async (reportId) => {
      await ask<unknown>(`${report(reportId)}/seen`, { method: "PUT" });
    },
    companies: async () => (await ask<ListPage<Company>>("/companies")).items
  };
}
