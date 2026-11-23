<template>
  <!-- A report's attachments (`docs/design.md`, 「A report」): its pictures as thumbnails, each opening whole in the
       layer's picture viewer, stepping to the others; every other file a link that saves it, with its size. -->
  <div class="flex min-w-0 flex-col gap-3">
    <ul v-if="pictures.length > 0" class="flex flex-wrap gap-2">
      <li v-for="(picture, at) in pictures" :key="picture.id">
        <HintTip :text="picture.name" as="span" class="inline-flex" press>
          <button
            type="button"
            class="block size-24 overflow-hidden rounded-lg border border-default transition-colors hover:border-accented hover:bg-elevated active:bg-accented sm:size-28"
            :aria-label="$t('voc.aria.open_picture', { name: picture.name })"
            @click="show(at)"
          >
            <LoadingImage :src="address(picture)" :alt="picture.name" fill image-class="size-full object-cover" />
          </button>
        </HintTip>
      </li>
    </ul>

    <ul v-if="files.length > 0" class="flex min-w-0 flex-col gap-1">
      <li v-for="file in files" :key="file.id" class="min-w-0">
        <a
          :href="address(file)"
          :download="file.name"
          class="flex min-h-10 min-w-0 items-center gap-2 rounded-md px-2 py-1 text-sm transition-colors hover:bg-elevated active:bg-accented"
          :aria-label="$t('voc.aria.download', { name: file.name })"
        >
          <UIcon :name="fileIcon(file.name)" class="size-5 shrink-0" />
          <span class="min-w-0 flex-1 truncate text-highlighted">{{ file.name }}</span>
          <span class="shrink-0 tabular-nums text-muted">{{ sizeText(file.size, locale) }}</span>
          <UIcon name="i-material-symbols-download" class="size-5 shrink-0 text-muted" />
        </a>
      </li>
    </ul>

    <ImageViewer
      v-if="shown !== null"
      v-model:open="viewing"
      :src="address(shown)"
      :title="shown.name"
      :can-previous="at > 0"
      :can-next="at < pictures.length - 1"
      @previous="at -= 1"
      @next="at += 1"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import HintTip from "bmo-ui/app/components/HintTip.vue";
import ImageViewer from "bmo-ui/app/components/ImageViewer.vue";
import LoadingImage from "bmo-ui/app/components/LoadingImage.vue";
import { fileIcon } from "bmo-ui/app/domain/file-icons";

import { useVoc } from "../../composables/useVoc";
import { isPicture, sizeText } from "../../domain/reports";

import type { Attachment } from "../../../shared/types";

const props = defineProps<{ reportId: string; attachments: readonly Attachment[] }>();

const voc = useVoc();
const locale = useAppConfig().bmoUi?.locale;

const pictures = computed(() => props.attachments.filter(isPicture));
const files = computed(() => props.attachments.filter((attachment) => !isPicture(attachment)));

const viewing = ref(false);
const at = ref(0);
const shown = computed(() => pictures.value[at.value] ?? null);

const address = (attachment: Attachment): string => voc.attachmentAddress(props.reportId, attachment.id);

function show(index: number): void {
  at.value = index;
  viewing.value = true;
}
</script>
