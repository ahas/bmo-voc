<template>
  <!-- The screen's first view (`docs/design.md`, 「The console」): for a sender their own reports with the create
       button that writes a new one, for the maker the inbox of every report, filtered by kind, state and company in
       the screen's row. Newest activity first, read a page at a time, the next page at the list's end. -->
  <VocFrame id="voc" :refreshing="refreshing" @refresh="refresh">
    <template #default="{ short }">
      <template v-if="maker">
        <template v-if="!short">
          <div v-for="choice in choices" :key="choice.key">
            <ChoiceSelect
              :model-value="choice.value"
              :items="choice.items"
              value-key="value"
              size="sm"
              class="w-36"
              :aria-label="choice.label"
              @update:model-value="(value: unknown) => choose(choice.key, String(value ?? ''))"
            />
          </div>
        </template>

        <div v-else>
          <UPopover :content="{ align: 'start' }">
            <IconButton :label="$t('voc.action.filter')" icon="i-material-symbols-filter-list" :color="filtering ? 'primary' : 'neutral'" :variant="filtering ? 'soft' : 'ghost'" />

            <template #content>
              <div class="flex w-64 max-w-[calc(100vw-2rem)] flex-col gap-3 p-3">
                <UFormField v-for="choice in choices" :key="choice.key" :label="choice.label">
                  <ChoiceSelect :model-value="choice.value" :items="choice.items" value-key="value" class="w-full" :aria-label="choice.label" @update:model-value="(value: unknown) => choose(choice.key, String(value ?? ''))" />
                </UFormField>
              </div>
            </template>
          </UPopover>
        </div>
      </template>
    </template>

    <template #actions>
      <CreateButton v-if="sender" :label="$t('voc.action.new_report')" @click="sending = true" />
    </template>

    <template #body>
      <ReadFailure v-if="failure !== null" :reason="failure" :reading="refreshing" @retry="refresh" />

      <!-- Which view stands is the reader's side, so nothing stands before the reader is read. -->
      <template v-if="voc.me.value !== null">
        <!-- The maker's inbox on a wide screen: one table filling the view, the next page at its foot. -->
        <div v-if="maker && !narrow" class="flex min-h-0 flex-1 flex-col">
          <DataTableSkeleton v-if="!list.read.value && failure === null" layout="fill" :headers="columns.map((column) => String(column.header ?? ''))" :rows="5" :label="$t('voc.status.list_loading')" />

          <EmptyList v-else-if="list.read.value && list.items.value.length === 0" :words="filtering ? $t('voc.message.inbox_no_match') : $t('voc.message.inbox_empty')" />

          <DataTable v-else-if="list.read.value" :data="list.items.value" :columns="columns" layout="fill" selectable :row-key="(row: ReportSummary) => row.id" @select="(row: ReportSummary) => open(row)">
            <template #state-cell="{ row }">
              <span class="flex min-h-12 items-center gap-1 ps-3 pe-2">
                <UnreadMark :unread="row.original.unread" />
                <StateMark :state="row.original.state" />
              </span>
            </template>

            <template #kind-cell="{ row }">
              <span class="flex min-h-12 items-center px-2"><KindBadge :kind="row.original.kind" /></span>
            </template>

            <template #title-cell="{ row }">
              <span class="flex min-h-12 max-w-[32rem] min-w-0 items-center px-2">
                <NuxtLink :to="placeOf(row.original)" class="truncate text-highlighted hover:underline" :class="row.original.unread ? 'font-semibold' : ''" @click.stop>{{ row.original.title }}</NuxtLink>
              </span>
            </template>

            <template #sender-cell="{ row }">
              <span class="flex min-h-12 max-w-48 items-center px-2 text-sm"><span class="truncate">{{ row.original.sender.name }}</span></span>
            </template>

            <template #company-cell="{ row }">
              <span class="flex min-h-12 max-w-48 items-center px-2 text-sm"><span class="truncate">{{ row.original.company?.name ?? $t("voc.label.no_company") }}</span></span>
            </template>

            <template #replies-cell="{ row }">
              <span class="flex min-h-12 items-center justify-end px-2 text-sm tabular-nums">{{ row.original.replies.toLocaleString(locale) }}</span>
            </template>

            <template #sent-cell="{ row }">
              <span class="flex min-h-12 items-center ps-2 pe-4 text-sm tabular-nums">{{ formatDateClock(row.original.sentAt / 1000) }}</span>
            </template>
          </DataTable>

          <div v-if="list.more.value" class="flex shrink-0 justify-center border-t border-default p-2">
            <UButton color="neutral" variant="ghost" icon="i-material-symbols-expand-more" :label="$t('voc.action.load_more')" :loading="list.loadingMore.value" @click="list.next" />
          </div>
        </div>

        <!-- A sender's reports, and the maker's on a phone: a list of rows, the next page at its end. -->
        <div v-else class="min-h-0 flex-1 overflow-y-auto pb-(--bmo-create-room)">
          <ContentSkeleton v-if="!list.read.value && failure === null" :rows="4" list-class="flex flex-col" :label="$t('voc.status.list_loading')">
            <div class="flex min-h-14 items-center gap-3 border-b border-default py-2 ps-3 pe-4">
              <SkeletonBar class="size-5 rounded-full" />
              <SkeletonBar class="size-5 rounded-full" />
              <div class="flex min-w-0 flex-1 flex-col">
                <SkeletonLine size="base" width="w-48" />
                <SkeletonLine width="w-32" />
              </div>
            </div>
          </ContentSkeleton>

          <EmptyList v-else-if="list.read.value && list.items.value.length === 0" :words="maker ? (filtering ? $t('voc.message.inbox_no_match') : $t('voc.message.inbox_empty')) : $t('voc.message.sender_empty')" />

          <template v-else-if="list.read.value">
            <ul class="flex flex-col" :aria-label="$t('voc.title')">
              <li v-for="report in list.items.value" :key="report.id">
                <ReportRow :report="report" :maker="maker" />
              </li>
            </ul>

            <div v-if="list.more.value" class="flex justify-center p-2">
              <UButton color="neutral" variant="ghost" icon="i-material-symbols-expand-more" :label="$t('voc.action.load_more')" :loading="list.loadingMore.value" @click="list.next" />
            </div>
          </template>
        </div>
      </template>

      <SendDialog v-if="sender" v-model:open="sending" @sent="sent" />
    </template>
  </VocFrame>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import ChoiceSelect from "bmo-ui/app/components/ChoiceSelect.vue";
import ContentSkeleton from "bmo-ui/app/components/ContentSkeleton.vue";
import CreateButton from "bmo-ui/app/components/CreateButton.vue";
import DataTable from "bmo-ui/app/components/DataTable.vue";
import DataTableSkeleton from "bmo-ui/app/components/DataTableSkeleton.vue";
import IconButton from "bmo-ui/app/components/IconButton.vue";
import SkeletonBar from "bmo-ui/app/components/SkeletonBar.vue";
import SkeletonLine from "bmo-ui/app/components/SkeletonLine.vue";
import { useNarrow } from "bmo-ui/app/domain/screen";
import { formatDateClock } from "bmo-ui/app/domain/time";

import EmptyList from "../../components/voc/EmptyList.vue";
import KindBadge from "../../components/voc/KindBadge.vue";
import ReadFailure from "../../components/voc/ReadFailure.vue";
import ReportRow from "../../components/voc/ReportRow.vue";
import SendDialog from "../../components/voc/SendDialog.vue";
import StateMark from "../../components/voc/StateMark.vue";
import UnreadMark from "../../components/voc/UnreadMark.vue";
import VocFrame from "../../components/voc/VocFrame.vue";
import { useReportList } from "../../composables/useReportList";
import { useVoc, useVocEvents } from "../../composables/useVoc";
import { oneOf } from "../../domain/reports";
import { KINDS, STATES } from "../../../shared/types";

import type { TableColumn } from "@nuxt/ui";
import type { Company, Report, ReportFilter, ReportSummary } from "../../../shared/types";

definePageMeta({ key: "voc" });

type FilterKey = "kind" | "state" | "company";

const { t } = useI18n();
const voc = useVoc();
const route = useRoute();
const router = useRouter();
const narrow = useNarrow();
const locale = useAppConfig().bmoUi?.locale;

const side = computed(() => voc.me.value?.side ?? null);
const sender = computed(() => side.value === "sender");
// The maker reads the inbox; a reader of everything reads it too, and writes nothing.
const maker = computed(() => side.value === "maker" || side.value === "reader");

const companies = ref<Company[]>([]);
const sending = ref(false);
const refreshing = ref(false);

// The filters stand in the address, so the inbox opened again is filtered as it was left.
const filter = computed<ReportFilter>(() => {
  if (!maker.value) return {};

  const company = typeof route.query.company === "string" && route.query.company !== "" ? route.query.company : undefined;

  return { kind: oneOf(route.query.kind, KINDS), state: oneOf(route.query.state, STATES), company };
});
const filtering = computed(() => Object.values(filter.value).some((value) => value !== undefined));

const list = useReportList(() => filter.value);

const failure = computed(() => voc.meFailure.value ?? list.failure.value);

const choices = computed(() => [
  {
    key: "kind" as FilterKey,
    label: t("voc.label.kind"),
    value: filter.value.kind ?? "",
    items: [{ label: t("voc.label.all_kinds"), value: "" }, ...KINDS.map((kind) => ({ label: t(`voc.kind.${kind}`), value: kind }))]
  },
  {
    key: "state" as FilterKey,
    label: t("voc.label.state"),
    value: filter.value.state ?? "",
    items: [{ label: t("voc.label.all_states"), value: "" }, ...STATES.map((state) => ({ label: t(`voc.state.${state}`), value: state }))]
  },
  {
    key: "company" as FilterKey,
    label: t("voc.label.company"),
    value: filter.value.company ?? "",
    items: [{ label: t("voc.label.all_companies"), value: "" }, ...companies.value.map((company) => ({ label: company.name, value: company.id }))]
  }
]);

const columns = computed<TableColumn<ReportSummary>[]>(() => [
  { id: "state", header: t("voc.label.state") },
  { id: "kind", header: t("voc.label.kind") },
  { id: "title", header: t("voc.label.title") },
  { id: "sender", header: t("voc.label.sender") },
  { id: "company", header: t("voc.label.company") },
  { id: "replies", header: t("voc.label.replies") },
  { id: "sent", header: t("voc.label.sent_at") }
]);

const placeOf = (report: Pick<ReportSummary, "id">): string => `/voc/${encodeURIComponent(report.id)}`;

function choose(key: FilterKey, value: string): void {
  const query = { ...route.query, [key]: value === "" ? undefined : value };

  void router.replace({ query });
}

function open(report: ReportSummary): void {
  void router.push(placeOf(report));
}

async function readCompanies(): Promise<void> {
  if (!maker.value) return;

  try {
    companies.value = await voc.api.companies();
  } catch {
    // The companies only fill a filter's choices; the inbox stands without them, and a refresh reads them again.
  }
}

async function refresh(): Promise<void> {
  refreshing.value = true;

  await voc.load(true);
  await Promise.all([list.again(), readCompanies()]);

  refreshing.value = false;
}

function sent(report: Report): void {
  voc.told("voc.message.sent");
  void router.push(placeOf(report));
}

useVocEvents(() => {
  void list.again();

  if (maker.value) void readCompanies();
});

onMounted(async () => {
  voc.arrived();
  await voc.arrive();

  if (voc.me.value === null) return;

  await Promise.all([list.again(), readCompanies()]);
});
</script>
