<template>
  <!-- A report the other side acted on since the reader's side last opened it: a dot in the accent, named. The place
       stands empty where it is read, so the rows' marks line up. -->
  <span class="flex size-5 shrink-0 items-center justify-center">
    <HintTip v-if="props.unread" :text="$t('voc.label.unread')" as="span" class="inline-flex">
      <span class="size-2.5 rounded-full bg-primary" role="img" :aria-label="$t('voc.label.unread')" data-unread />
    </HintTip>
  </span>
</template>

<script setup lang="ts">
import HintTip from "bmo-ui/app/components/HintTip.vue";

const props = defineProps<{ unread: boolean }>();
</script>
