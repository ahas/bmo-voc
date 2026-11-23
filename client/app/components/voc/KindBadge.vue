<template>
  <!-- A report's kind, its mark and its word. -->
  <UBadge size="md" color="neutral" variant="subtle" :icon="KIND_ICONS[props.kind]" :label="$t(`voc.kind.${props.kind}`)" class="shrink-0" />
</template>

<script setup lang="ts">
import { KIND_ICONS } from "../../domain/look";

import type { Kind } from "../../../shared/types";

const props = defineProps<{ kind: Kind }>();
</script>
