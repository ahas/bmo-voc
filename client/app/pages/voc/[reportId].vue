<template>
  <!-- One report's thread (`docs/design.md`, 「The console」), for either side: its title, kind and state, who sent it
       and from which company where the maker reads it, when, its body, its attachments, where it was sent from for the
       maker, its state's history, the replies of both sides in order, and the box that answers. The maker moves its
       state from the screen's row. Opening it marks it read for the reader's side. -->
  <VocFrame id="voc-report" :back="{ path: SCREEN_PATH }" :refreshing="refreshing" @refresh="refresh">
    <template #default>
      <div v-if="writesState && report !== null">
        <ChoiceSelect
          :model-value="report.state"
          :items="stateItems"
          value-key="value"
          size="sm"
          class="w-32"
          :loading="moving"
          :aria-label="$t('voc.aria.state_control')"
          @update:model-value="(value: unknown) => moveTo(String(value))"
        />
      </div>
    </template>

    <template #body>
      <ReadFailure v-if="failure !== null" :reason="failure" :reading="refreshing" @retry="refresh" />

      <div class="min-h-0 flex-1 overflow-y-auto">
        <ContentSkeleton v-if="report === null && failure === null" :label="$t('voc.status.report_loading')" list-class="mx-auto flex w-full max-w-3xl flex-col gap-6 p-4 sm:p-6">
          <div class="flex flex-col gap-3">
            <SkeletonLine size="lg" width="w-72 max-w-full" />
            <SkeletonLine width="w-48" />
          </div>
          <SkeletonText :lines="4" />
        </ContentSkeleton>

        <article v-else-if="report !== null" class="mx-auto flex w-full max-w-3xl min-w-0 flex-col gap-6 p-4 sm:p-6">
          <header class="flex min-w-0 flex-col gap-3">
            <h1 class="text-xl font-semibold break-words text-highlighted">{{ report.title }}</h1>

            <div class="flex min-w-0 flex-wrap items-center gap-2 text-sm text-muted">
              <KindBadge :kind="report.kind" />
              <StateChip :state="report.state" />
              <span v-if="maker" class="min-w-0 break-words">{{ report.sender.name }} · {{ report.company?.name ?? $t("voc.label.no_company") }}</span>
              <span class="tabular-nums">{{ formatDateClock(report.sentAt / 1000) }}</span>
            </div>
          </header>

          <MarkdownBody :body="report.body" />

          <section v-if="report.attachments.length > 0" class="flex min-w-0 flex-col gap-3" aria-labelledby="voc-files">
            <h2 id="voc-files" class="text-lg font-semibold text-highlighted">{{ $t("voc.label.files") }}</h2>
            <AttachmentList :report-id="report.id" :attachments="report.attachments" />
          </section>

          <section v-if="maker" class="flex min-w-0 flex-col gap-3" aria-labelledby="voc-context">
            <h2 id="voc-context" class="text-lg font-semibold text-highlighted">{{ $t("voc.label.context") }}</h2>
            <dl class="pair-list text-sm">
              <template v-for="row in contextRows" :key="row.label">
                <dt class="text-muted">{{ row.label }}</dt>
                <dd class="break-all text-highlighted">{{ row.value === "" ? "—" : row.value }}</dd>
              </template>
            </dl>
          </section>

          <section id="history" class="flex min-w-0 scroll-mt-4 flex-col gap-3" aria-labelledby="voc-history">
            <h2 id="voc-history" class="text-lg font-semibold text-highlighted">{{ $t("voc.label.history") }}</h2>
            <ol class="flex flex-col gap-2 text-sm">
              <li v-for="(entry, at) in report.states" :key="at" class="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
                <UIcon :name="STATE_LOOK[entry.state].icon" class="size-5 shrink-0" :class="TONE_TEXT[STATE_LOOK[entry.state].tone]" />
                <span class="font-medium text-highlighted">{{ $t(`voc.state.${entry.state}`) }}</span>
                <span class="text-muted">{{ entry.by.name }} · {{ $t(`voc.side.${entry.by.side}`) }}</span>
                <span class="tabular-nums text-muted">{{ formatDateClock(entry.at / 1000) }}</span>
              </li>
            </ol>
          </section>

          <section class="flex min-w-0 flex-col gap-3" aria-labelledby="voc-replies">
            <h2 id="voc-replies" class="flex items-center gap-2 text-lg font-semibold text-highlighted">
              {{ $t("voc.label.replies") }}
              <UBadge v-if="report.replies.length > 0" v-bind="COUNT_BADGE" color="neutral" :label="report.replies.length.toLocaleString(locale)" />
            </h2>

            <p v-if="report.replies.length === 0" class="text-sm text-muted">{{ $t("voc.message.no_replies") }}</p>

            <ol v-else class="flex flex-col gap-3">
              <li v-for="reply in report.replies" :key="reply.id">
                <article class="flex min-w-0 flex-col gap-2 rounded-lg p-3" :class="reply.side === 'maker' ? 'bg-primary/10' : 'bg-elevated'" :aria-label="$t(reply.side === 'maker' ? 'voc.aria.answer_by' : 'voc.aria.follow_by', { name: reply.by.name })" data-reply>
                  <header class="flex min-w-0 flex-wrap items-center gap-2 text-sm">
                    <span class="font-medium text-highlighted">{{ reply.by.name }}</span>
                    <UBadge size="sm" variant="outline" :color="reply.side === 'maker' ? 'primary' : 'neutral'" :label="$t(reply.side === 'maker' ? 'voc.label.answer' : 'voc.label.follow')" class="badge-ink" />
                    <span class="tabular-nums text-muted">{{ formatDateClock(reply.at / 1000) }}</span>
                  </header>
                  <MarkdownBody :body="reply.body" />
                </article>
              </li>
            </ol>
          </section>

          <ReplyBox v-if="answering !== null" :report-id="report.id" :side="answering" @sent="replied" />
        </article>
      </div>
    </template>
  </VocFrame>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import ChoiceSelect from "bmo-ui/app/components/ChoiceSelect.vue";
import ContentSkeleton from "bmo-ui/app/components/ContentSkeleton.vue";
import SkeletonLine from "bmo-ui/app/components/SkeletonLine.vue";
import SkeletonText from "bmo-ui/app/components/SkeletonText.vue";
import { COUNT_BADGE } from "bmo-ui/app/domain/numbers";
import { formatDateClock } from "bmo-ui/app/domain/time";

import AttachmentList from "../../components/voc/AttachmentList.vue";
import KindBadge from "../../components/voc/KindBadge.vue";
import MarkdownBody from "../../components/voc/MarkdownBody.vue";
import ReadFailure from "../../components/voc/ReadFailure.vue";
import ReplyBox from "../../components/voc/ReplyBox.vue";
import StateChip from "../../components/voc/StateChip.vue";
import VocFrame from "../../components/voc/VocFrame.vue";
import { SCREEN_PATH, useVoc, useVocEvents } from "../../composables/useVoc";
import { STATE_LOOK, TONE_TEXT } from "../../domain/look";
import { oneOf } from "../../domain/reports";
import { STATES } from "../../../shared/types";

import type { Report } from "../../../shared/types";

definePageMeta({ key: "voc-report" });

const { t } = useI18n();
const voc = useVoc();
const route = useRoute();
const locale = useAppConfig().bmoUi?.locale;

const reportId = computed(() => String(route.params.reportId ?? ""));

const report = ref<Report | null>(null);
const failure = ref<string | null>(null);
const refreshing = ref(false);
const moving = ref(false);

const side = computed(() => voc.me.value?.side ?? null);
// The maker, and a reader of everything, read who sent it and from where.
const maker = computed(() => side.value === "maker" || side.value === "reader");
const writesState = computed(() => side.value === "maker");
// The side that writes on the thread; a reader of everything writes nothing.
const answering = computed(() => (side.value === "maker" || side.value === "sender" ? side.value : null));

const stateItems = computed(() => STATES.map((state) => ({ label: t(`voc.state.${state}`), value: state })));

const contextRows = computed(() => {
  const context = report.value?.context;

  return [
    { label: t("voc.label.path"), value: context?.path ?? "" },
    { label: t("voc.label.console_version"), value: context?.consoleVersion ?? "" },
    { label: t("voc.label.browser"), value: context?.browser ?? "" }
  ];
});

/** Reads the report, and marks it read for the reader's side once it stands. */
async function read(): Promise<void> {
  const id = reportId.value;

  // The address is leaving the thread; nothing is named to read.
  if (id === "") return;

  try {
    const answer = await voc.api.report(id);

    if (id !== reportId.value) return;

    report.value = answer;
    failure.value = null;
    void voc.api.seen(id).catch(() => undefined);
  } catch (error) {
    if (id === reportId.value) failure.value = voc.readWords(error);
  }
}

async function refresh(): Promise<void> {
  refreshing.value = true;

  await Promise.all([voc.load(true), read()]);

  refreshing.value = false;
}

async function moveTo(value: string): Promise<void> {
  const state = oneOf(value, STATES);

  if (state === undefined || report.value === null || state === report.value.state) return;

  moving.value = true;

  try {
    report.value = await voc.api.setState(report.value.id, state);
    voc.told("voc.message.state_changed");
  } catch (error) {
    voc.failed(error, "voc.message.state_failed");
  } finally {
    moving.value = false;
  }
}

function replied(): void {
  void read();
}

// The thread is read again where an event names it; the stream carries only the reader's own.
useVocEvents(
  () => void read(),
  (event) => event.data.reportId === reportId.value
);

// A thread opened in place of another under the same screen reads the one it now names.
watch(reportId, () => {
  report.value = null;
  failure.value = null;
  void read();
});

onMounted(() => {
  voc.arrived();
  void voc.arrive();
  void read();
});
</script>
