<template>
  <!-- A list read and holding nothing: its mark and one line saying so. -->
  <div class="flex flex-col items-center gap-3 px-4 py-16 text-center">
    <UIcon name="i-material-symbols-campaign-outline" class="size-10 text-dimmed" />
    <p class="text-sm text-muted">{{ props.words }}</p>
  </div>
</template>

<script setup lang="ts">
const props = defineProps<{ words: string }>();
</script>
