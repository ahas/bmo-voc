<template>
  <!-- A report written and sent (`docs/design.md`, 「The sender's screen」): its kind, its title, its body in markdown and
       its files, each bound shown beside its field; a picture pasted into the body is attached as a file. The console
       the sender came from, its version and the browser are taken as the form opens. One send at the foot. -->
  <SettingsDialog v-model:open="open" :title="$t('voc.action.new_report')" :dirty="changes">
    <UFormField :label="$t('voc.label.kind')">
      <SettingsSegment v-model="kind" :items="kindItems" :label="$t('voc.label.kind')" />
    </UFormField>

    <UFormField :label="$t('voc.label.title')" :hint="$t('voc.hint.title', { max: TITLE_MAX.toLocaleString(locale) })" required>
      <UInput v-model="title" class="w-full" :maxlength="TITLE_MAX" />
    </UFormField>

    <UFormField :label="$t('voc.label.body')" :hint="$t('voc.hint.body', { max: BODY_MAX.toLocaleString(locale) })" required>
      <GrowingTextarea v-model="body" class="w-full" :rows="6" :maxlength="BODY_MAX" @paste="pasted" />
    </UFormField>

    <UFormField :label="$t('voc.label.files')" :hint="$t('voc.hint.files', { count: MAX_FILES.toLocaleString(locale), size: sizeText(MAX_BYTES, locale) })" :error="filesError">
      <div class="flex min-w-0 flex-col gap-2">
        <ul v-if="files.length > 0" class="flex min-w-0 flex-col gap-1" :aria-label="$t('voc.label.files')">
          <li v-for="(file, at) in files" :key="`${file.name}-${at}`" class="flex min-h-10 min-w-0 items-center gap-2 rounded-md bg-elevated ps-2 text-sm">
            <UIcon :name="fileIcon(file.name)" class="size-5 shrink-0" />
            <span class="min-w-0 flex-1 truncate text-highlighted">{{ file.name }}</span>
            <span class="shrink-0 tabular-nums text-muted">{{ sizeText(file.size, locale) }}</span>
            <IconButton :label="$t('voc.action.remove_file', { name: file.name })" icon="i-material-symbols-close" @click="files.splice(at, 1)" />
          </li>
        </ul>

        <div>
          <UButton color="neutral" variant="outline" icon="i-material-symbols-attach-file" :label="$t('voc.action.attach')" @click="picker?.click()" />
          <input ref="picker" type="file" multiple hidden data-file-picker @change="picked" />
        </div>
      </div>
    </UFormField>

    <UAlert v-if="refusal !== null" color="error" variant="subtle" icon="i-material-symbols-error-outline" :description="refusal" role="alert" />

    <template #footer>
      <UButton icon="i-material-symbols-send-outline" :label="$t('voc.action.send')" :disabled="!sendable" :loading="sending" @click="send" />
    </template>
  </SettingsDialog>
</template>

<script setup lang="ts">
import { computed, ref, useTemplateRef, watch } from "vue";
import { useI18n } from "vue-i18n";
import GrowingTextarea from "bmo-ui/app/components/GrowingTextarea.vue";
import IconButton from "bmo-ui/app/components/IconButton.vue";
import SettingsDialog from "bmo-ui/app/components/SettingsDialog.vue";
import SettingsSegment from "bmo-ui/app/components/SettingsSegment.vue";
import { useOpenedWith } from "bmo-ui/app/domain/form-save";
import { fileIcon } from "bmo-ui/app/domain/file-icons";

import { useVoc } from "../../composables/useVoc";
import { Refusal } from "../../domain/api";
import { sizeText } from "../../domain/reports";
import { BODY_MAX, KINDS, MAX_BYTES, MAX_FILES, TITLE_MAX } from "../../../shared/types";

import type { Context, Kind, Report } from "../../../shared/types";

const open = defineModel<boolean>("open", { default: false });

const emit = defineEmits<{ sent: [report: Report] }>();

const { t } = useI18n();
const voc = useVoc();
const locale = useAppConfig().bmoUi?.locale;

// The codes the service refuses a file with, said beside the files rather than on the pill.
const FILE_CODES = new Set(["too-many-files", "file-too-large"]);

const kind = ref<Kind>("bug");
const title = ref("");
const body = ref("");
const files = ref<File[]>([]);
const context = ref<Context>({ path: "", consoleVersion: "", browser: "" });
const sending = ref(false);
const refused = ref<Refusal | null>(null);
const picker = useTemplateRef<HTMLInputElement>("picker");

const kindItems = computed(() => KINDS.map((value) => ({ label: t(`voc.kind.${value}`), value })));

const changes = useOpenedWith(() => ({ kind: kind.value, title: title.value, body: body.value, files: files.value }), { open });

/** What the files break of the bounds, said before the send; the service's own refusal where it gave one. */
const filesError = computed<string | undefined>(() => {
  if (files.value.length > MAX_FILES) return t("voc.message.too_many_files", { count: MAX_FILES.toLocaleString(locale) });

  const large = files.value.find((file) => file.size > MAX_BYTES);

  if (large !== undefined) return t("voc.message.file_too_large", { name: large.name, size: sizeText(MAX_BYTES, locale) });

  const empty = files.value.find((file) => file.size === 0);

  if (empty !== undefined) return t("voc.message.empty_file", { name: empty.name });
  if (refused.value !== null && FILE_CODES.has(refused.value.code)) return voc.codeWords(refused.value) ?? undefined;

  return undefined;
});

/** A refusal of the report that names no file, said over the foot. */
const refusal = computed(() => (refused.value !== null && !FILE_CODES.has(refused.value.code) ? voc.codeWords(refused.value) : null));

const sendable = computed(() => title.value.trim() !== "" && body.value.trim() !== "" && filesError.value === undefined);

watch(open, (opened) => {
  if (!opened) return;

  kind.value = "bug";
  title.value = "";
  body.value = "";
  files.value = [];
  refused.value = null;
  context.value = voc.context();
});

// A refusal is of what was sent; a change to what is written lets it go.
watch([kind, title, body, () => files.value.length], () => {
  refused.value = null;
});

function picked(event: Event): void {
  const input = event.target as HTMLInputElement;

  files.value.push(...Array.from(input.files ?? []));
  input.value = "";
}

/** A picture pasted into the body is attached rather than written; words pasted stay words. */
function pasted(event: ClipboardEvent): void {
  const pictures = Array.from(event.clipboardData?.files ?? []).filter((file) => file.type.startsWith("image/"));

  if (pictures.length === 0) return;

  event.preventDefault();
  files.value.push(...pictures);
}

async function send(): Promise<void> {
  sending.value = true;
  refused.value = null;

  try {
    const report = await voc.api.send({ kind: kind.value, title: title.value.trim(), body: body.value, context: context.value }, files.value);

    // Sent, the form is closed as it stands: nothing of it is left to throw away.
    open.value = false;
    emit("sent", report);
  } catch (error) {
    if (error instanceof Refusal && voc.codeWords(error) !== null && error.code !== "upstream-failed") refused.value = error;
    else voc.failed(error, "voc.message.send_failed");
  } finally {
    sending.value = false;
  }
}
</script>
