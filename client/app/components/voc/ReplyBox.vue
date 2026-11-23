<template>
  <!-- The box at a thread's foot that answers the other side (`docs/design.md`, 「Replies」): the maker's answer, or the
       sender's follow-up on their own report, in markdown, sent by its button alone, so Enter breaks the line on every
       device. -->
  <form class="flex min-w-0 flex-col gap-2" @submit.prevent="send">
    <UFormField :label="$t(words.label)" :hint="$t('voc.hint.body', { max: BODY_MAX.toLocaleString(locale) })">
      <GrowingTextarea v-model="body" class="w-full" :rows="3" :maxlength="BODY_MAX" />
    </UFormField>

    <div class="flex justify-end">
      <UButton type="submit" icon="i-material-symbols-send-outline" :label="$t(words.action)" :disabled="body.trim() === ''" :loading="sending" />
    </div>
  </form>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import GrowingTextarea from "bmo-ui/app/components/GrowingTextarea.vue";
import { useLeaveGuard, useOpenedWith } from "bmo-ui/app/domain/form-save";
import { useI18n } from "vue-i18n";

import { useVoc } from "../../composables/useVoc";
import { BODY_MAX } from "../../../shared/types";

import type { Reply } from "../../../shared/types";

const props = defineProps<{ reportId: string; side: "maker" | "sender" }>();

const emit = defineEmits<{ sent: [reply: Reply] }>();

const { t } = useI18n();
const voc = useVoc();
const locale = useAppConfig().bmoUi?.locale;

// The maker answers; the sender follows up on their own report.
const WORDS = {
  maker: { label: "voc.label.answer", action: "voc.action.answer", done: "voc.message.answered", failed: "voc.message.answer_failed" },
  sender: { label: "voc.label.follow", action: "voc.action.follow", done: "voc.message.followed", failed: "voc.message.follow_failed" }
} as const;

const words = computed(() => WORDS[props.side]);
const body = ref("");
const sending = ref(false);

// Leaving the thread with an answer written and not sent asks first.
const changes = useOpenedWith(() => body.value);

useLeaveGuard(changes.dirty, t);

async function send(): Promise<void> {
  if (body.value.trim() === "" || sending.value) return;

  sending.value = true;

  try {
    const reply = await voc.api.reply(props.reportId, body.value);

    body.value = "";
    await changes.take();
    voc.told(words.value.done);
    emit("sent", reply);
  } catch (error) {
    voc.failed(error, words.value.failed);
  } finally {
    sending.value = false;
  }
}
</script>
