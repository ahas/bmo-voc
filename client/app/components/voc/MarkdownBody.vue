<template>
  <!-- A body a person wrote in markdown, laid into the layer's `markdown` container, its own words' measures in the
       bundle's stylesheet. -->
  <!-- eslint-disable-next-line vue/no-v-html -- the markup is the sanitised rendering of the markdown -->
  <div class="markdown voc-markdown min-w-0 break-words" v-html="html" />
</template>

<script setup lang="ts">
import { computed } from "vue";

import { markdownHtml } from "../../domain/markdown";

const props = defineProps<{ body: string }>();

const html = computed(() => markdownHtml(props.body));
</script>
