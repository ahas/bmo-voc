<template>
  <!-- A report's state on its thread: a chip with its mark and word, leading to the state's history on the page. -->
  <UButton
    size="sm"
    variant="soft"
    :color="STATE_LOOK[props.state].tone"
    :icon="STATE_LOOK[props.state].icon"
    :label="$t(`voc.state.${props.state}`)"
    class="badge-ink shrink-0"
    href="#history"
    data-state-chip
    @click.prevent="toHistory"
  />
</template>

<script setup lang="ts">
import { STATE_LOOK } from "../../domain/look";

import type { State } from "../../../shared/types";

const props = defineProps<{ state: State }>();

function toHistory(): void {
  document.getElementById("history")?.scrollIntoView({ behavior: "smooth", block: "start" });
}
</script>
