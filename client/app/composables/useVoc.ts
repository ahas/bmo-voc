import { onBeforeUnmount, ref } from "vue";
import { useI18n } from "vue-i18n";
import { relayAddress, relayCall } from "bmo-ui/app/domain/relay";
import { useIsland } from "bmo-ui/app/domain/island";

import { Refusal, Unreachable, attachmentPath, createVocApi } from "../domain/api";
import { WORDED_CODES } from "../domain/codes";
import { cameFromPath } from "../domain/reports";
import { useVocStream } from "../domain/stream";

import { createAppComposable } from "./shared";

import type { RelayOptions } from "bmo-ui/app/domain/relay";
import type { Call } from "../domain/api";
import type { VocStreamEvent } from "../domain/stream";
import type { Context, Me } from "../../shared/types";

// What the screens hold of the voc service, read through the console's relay to this unit alone: the reader and the
// side they stand on, read once and again on a refresh; where the reader came to the screen from; and the words for
// what went wrong, put on the console's message pill.

export const SERVICE = "voc";
export const SCREEN_PATH = "/voc";

const relay: Call = <T>(path: string, options?: Parameters<Call>[1]) => relayCall<T>(SERVICE, path, options as RelayOptions);

const WORDED = new Set<string>(WORDED_CODES);

const _useVoc = () => {
  const { t } = useI18n();
  const island = useIsland(t);
  const api = createVocApi(relay);

  const me = ref<Me | null>(null);
  // Why the reader's last read failed, in words; null while it went through.
  const meFailure = ref<string | null>(null);
  // The console path the reader came to the screen from, kept while they move about inside it.
  const cameFrom = ref("");

  /** The reader, read once; `again` reads them afresh. */
  async function load(again = false): Promise<void> {
    if (me.value !== null && !again) return;

    try {
      me.value = await api.me();
      meFailure.value = null;
    } catch (error) {
      meFailure.value = readWords(error);
    }
  }

  /**
   * The reader read afresh as a screen of the voc opens. The console signs a person out and another in without a page
   * read, so the reader kept from an earlier visit may be somebody else: nothing of a side is drawn until the reader
   * is read again.
   */
  async function arrive(): Promise<void> {
    me.value = null;
    meFailure.value = null;
    await load(true);
  }

  /** Notes where the reader came from, as the router recorded the address before the one opened. */
  function arrived(): void {
    const back = cameFromPath((window.history.state as { back?: unknown } | null)?.back, SCREEN_PATH);

    if (back !== null) cameFrom.value = back;
  }

  /** What the reader's console says of itself as a report is written, each empty where it says nothing. */
  function context(): Context {
    const version = (useAppConfig().bmoUi as { consoleVersion?: unknown } | undefined)?.consoleVersion;

    return { path: cameFrom.value, consoleVersion: typeof version === "string" ? version : "", browser: navigator.userAgent };
  }

  /** The pack's words for a refusal's code, or null where it has none. */
  function codeWords(error: unknown): string | null {
    return error instanceof Refusal && WORDED.has(error.code) ? t(`voc.code.${error.code}`) : null;
  }

  /** Why a read failed, in words a region says in its own place. */
  function readWords(error: unknown): string {
    if (error instanceof Unreachable) return t("voc.message.unreachable");

    return codeWords(error) ?? t("voc.message.read_failed");
  }

  /** Says an act went wrong on the pill: the pack's words for its code, else the act's own. */
  function failed(error: unknown, fallback: string): void {
    const words = error instanceof Unreachable ? t("voc.message.unreachable") : (codeWords(error) ?? t(fallback));

    island.say({ tone: "error", title: words });
  }

  /** Says an act went through. */
  function told(words: string): void {
    island.say({ tone: "success", title: t(words) });
  }

  /** The address an attachment is opened or saved at, through the relay. */
  function attachmentAddress(reportId: string, attachmentId: string): string {
    return relayAddress(SERVICE, attachmentPath(reportId, attachmentId));
  }

  return { api, me, meFailure, cameFrom, load, arrive, arrived, context, codeWords, readWords, failed, told, attachmentAddress };
};

export const useVoc = createAppComposable(_useVoc);

// How long the screen waits for the rest of a burst of events before it reads again.
const GATHER_MS = 400;

/** Reads the screen again once the events heard say what it shows may have changed. */
export function useVocEvents(again: () => void, heard: (event: VocStreamEvent) => boolean = () => true): void {
  let gathering: ReturnType<typeof setTimeout> | null = null;

  function gather(): void {
    if (gathering !== null) clearTimeout(gathering);

    gathering = setTimeout(() => {
      gathering = null;
      again();
    }, GATHER_MS);
  }

  useVocStream(SERVICE, (event) => (heard(event) ? gather() : undefined), gather);

  onBeforeUnmount(() => {
    if (gathering !== null) clearTimeout(gathering);
  });
}
