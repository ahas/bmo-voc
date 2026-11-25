import { computed, ref, shallowReactive, watch } from "vue";

import { useVoc } from "./useVoc";
import { createAppComposable } from "./shared";

import type { Ref } from "vue";
import type { ReportFilter, ReportSummary } from "../../shared/types";

// The reports a list shows, newest activity first, read a page at a time (`docs/api.md`: `after` the rows read so far,
// `limit` the page). What was last read under each filter is kept while the app stands, so a list opened again — the
// way back from a report — stands at once as it was left and is read again under it.

/** One page, the service's own default. */
export const PAGE = 50;
/** The most one read asks for, the service's own ceiling. */
const MOST = 200;

interface Kept {
  items: ReportSummary[];
  more: boolean;
}

const useKept = createAppComposable(() => shallowReactive(new Map<string, Kept>()));

export interface ReportList {
  items: Ref<ReportSummary[]>;
  more: Ref<boolean>;
  /** Whether the list has been read under its filter, so a skeleton stands only before. */
  read: Ref<boolean>;
  reading: Ref<boolean>;
  /** Why the last read failed, in words; null while it went through. */
  failure: Ref<string | null>;
  /** Reads the rows shown again: as many as stand, from the first. */
  again: () => Promise<void>;
  /** Reads the next page onto the end; it throws where the page failed, which the list's foot says with its retry. */
  next: () => Promise<void>;
}

export function useReportList(filter: () => ReportFilter): ReportList {
  const voc = useVoc();
  const kept = useKept();
  // Kept by the reader as well as the filter, so a list one reader read is never shown to another.
  const keyOf = computed(() => JSON.stringify([voc.me.value?.user ?? null, voc.me.value?.side ?? null, filter()]));

  const reading = ref(false);
  const failure = ref<string | null>(null);
  // What the list last showed, which stands while a filter read for the first time is read, so the rows already on the
  // screen stay there rather than giving way to a skeleton.
  let shown: Kept | null = null;
  const standing = computed(() => {
    const held = kept.get(keyOf.value);

    if (held !== undefined) shown = held;

    return held ?? shown;
  });
  // Each read is numbered, so an answer to a filter left behind lands nowhere.
  let asked = 0;

  async function again(): Promise<void> {
    const key = keyOf.value;
    const number = ++asked;
    const count = Math.min(Math.max(PAGE, standing.value?.items.length ?? 0), MOST);

    reading.value = true;

    try {
      const page = await voc.api.reports(filter(), 0, count);

      if (number !== asked) return;

      kept.set(key, { items: page.items, more: page.more });
      failure.value = null;
    } catch (error) {
      if (number === asked) failure.value = voc.readWords(error);
    } finally {
      if (number === asked) reading.value = false;
    }
  }

  async function next(): Promise<void> {
    const key = keyOf.value;
    const held = kept.get(key);

    if (held === undefined || !held.more) return;

    const page = await voc.api.reports(filter(), held.items.length, PAGE);
    const known = new Set(held.items.map((item) => item.id));

    kept.set(key, { items: [...held.items, ...page.items.filter((item) => !known.has(item.id))], more: page.more });
  }

  watch(keyOf, () => {
    failure.value = null;
    void again();
  });

  return {
    items: computed(() => standing.value?.items ?? []),
    more: computed(() => standing.value?.more ?? false),
    read: computed(() => standing.value !== null),
    reading,
    failure,
    again,
    next
  } as ReportList;
}
