<template>
  <!-- One report in a list, the whole row its link (`docs/design.md`, 「The console」): the unread mark and the state's
       mark first, then its title, which takes the room left over, and its state's word, its kind, its sender and
       company where the maker reads it, its reply count and when it was sent. Where the row has the width these stand
       on one line in columns; a phone puts all but the title on a line under it. -->
  <NuxtLink
    :to="`/voc/${encodeURIComponent(props.report.id)}`"
    class="flex min-h-14 min-w-0 items-center gap-3 border-b border-default py-2 ps-3 pe-4 transition-colors hover:bg-elevated active:bg-accented focus-inset"
    data-report-row
  >
    <UnreadMark :unread="props.report.unread" />
    <StateMark :state="props.report.state" aria-hidden="true" />

    <span class="flex min-w-0 flex-1 flex-col gap-1 md:flex-row md:items-center md:gap-3">
      <KindBadge :kind="props.report.kind" class="max-md:hidden" />
      <span class="min-w-0 flex-1 truncate text-highlighted" :class="props.report.unread ? 'font-semibold' : ''">{{ props.report.title }}</span>

      <span class="flex min-w-0 items-center gap-2 text-sm text-muted md:gap-3">
        <span class="shrink-0 md:w-16" :class="TONE_TEXT[STATE_LOOK[props.report.state].tone]" data-report-state>{{ $t(`voc.state.${props.report.state}`) }}</span>
        <span class="md:hidden">{{ $t(`voc.kind.${props.report.kind}`) }}</span>
        <template v-if="props.maker">
          <span class="truncate md:w-28">{{ props.report.sender.name }}</span>
          <span class="truncate md:w-32">{{ props.report.company?.name ?? $t("voc.label.no_company") }}</span>
        </template>
        <span class="flex shrink-0 items-center gap-1 tabular-nums md:w-12 md:justify-end">
          <UIcon name="i-material-symbols-chat-outline" class="size-4" />
          <span class="sr-only">{{ $t("voc.label.replies") }}</span>
          {{ props.report.replies.toLocaleString(locale) }}
        </span>
        <span class="shrink-0 tabular-nums md:w-36 md:text-end">{{ formatDateClock(props.report.sentAt / 1000) }}</span>
      </span>
    </span>
  </NuxtLink>
</template>

<script setup lang="ts">
import { formatDateClock } from "bmo-ui/app/domain/time";

import { STATE_LOOK, TONE_TEXT } from "../../domain/look";

import KindBadge from "./KindBadge.vue";
import StateMark from "./StateMark.vue";
import UnreadMark from "./UnreadMark.vue";

import type { ReportSummary } from "../../../shared/types";

const props = defineProps<{ report: ReportSummary; maker?: boolean }>();

const locale = useAppConfig().bmoUi?.locale;
</script>
