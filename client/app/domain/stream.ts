import { onBeforeUnmount, onMounted } from "vue";
import { relayAddress } from "bmo-ui/app/domain/relay";

// The voc service's event stream as a screen reads it (`docs/api.md`, 「Events the voc unit publishes」), through the
// console's relay. The browser resumes it after the last event it read; where the service no longer keeps events
// since then it refuses the stream, and the screen opens it afresh and reads what it shows again. An event reaches
// the maker always and a sender for their own reports alone, so what is heard is the reader's to read.

export const STREAM_KINDS = ["report.received", "report.answered", "report.followed", "report.changed"] as const;

export type StreamKind = (typeof STREAM_KINDS)[number];

export interface VocStreamEvent {
  kind: StreamKind;
  data: { reportId?: string; sender?: string; title?: string; kind?: string; state?: string };
}

// How long a stream the service refused waits before it is opened afresh.
const REOPEN_MS = 3_000;

/** Calls `onEvent` with each event while the screen holding it stands, and `onReset` each time it is opened afresh. */
export function useVocStream(service: string, onEvent: (event: VocStreamEvent) => void, onReset: () => void): void {
  let source: EventSource | null = null;
  let reopening: ReturnType<typeof setTimeout> | null = null;
  let standing = false;

  function read(message: MessageEvent<string>): void {
    try {
      const envelope = JSON.parse(message.data) as { kind?: string; data?: unknown };

      onEvent({ kind: (envelope.kind ?? message.type) as StreamKind, data: (envelope.data ?? envelope) as VocStreamEvent["data"] });
    } catch {
      // An event that does not read is passed over; the next one says where things stand.
    }
  }

  function open(): void {
    source = new EventSource(relayAddress(service, `/events?kinds=${STREAM_KINDS.join(",")}`));

    for (const kind of STREAM_KINDS) source.addEventListener(kind, read);

    source.onerror = () => {
      if (source?.readyState !== EventSource.CLOSED || !standing) return;

      source = null;
      reopening = setTimeout(() => {
        reopening = null;

        if (!standing) return;

        open();
        onReset();
      }, REOPEN_MS);
    };
  }

  onMounted(() => {
    standing = true;
    open();
  });

  onBeforeUnmount(() => {
    standing = false;
    source?.close();
    source = null;

    if (reopening !== null) clearTimeout(reopening);
  });
}
