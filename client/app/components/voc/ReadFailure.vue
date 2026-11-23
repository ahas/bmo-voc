<template>
  <!-- A read that failed, said in one band in the region's place (bmo-web's 「A failed read is said in one band」): the
       reason, and at its trailing end the icon button that reads it again, spinning while it does. -->
  <div role="alert" class="flex min-h-12 shrink-0 items-center gap-3 border-b border-default bg-error/10 ps-4 pe-2 py-1 text-sm" data-read-failure>
    <UIcon name="i-material-symbols-error-outline" class="size-5 shrink-0 text-error" />
    <span class="min-w-0 flex-1 text-highlighted">{{ props.reason }}</span>
    <IconButton :label="$t('voc.action.refresh')" icon="i-material-symbols-refresh" :loading="props.reading" class="shrink-0" @click="emit('retry')" />
  </div>
</template>

<script setup lang="ts">
import IconButton from "bmo-ui/app/components/IconButton.vue";

const props = defineProps<{ reason: string; reading: boolean }>();

const emit = defineEmits<{ retry: [] }>();
</script>
