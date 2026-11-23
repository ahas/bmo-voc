<template>
  <!-- A report's state where a list stands it: its mark alone, in its colour, named by the shared tooltip and to a
       screen reader. -->
  <HintTip :text="word" as="span" class="inline-flex shrink-0">
    <!-- The icon component lays down no role or name of its own, so the mark is named on the box around it. -->
    <span class="inline-flex" role="img" :aria-label="word" data-state-mark>
      <UIcon :name="STATE_LOOK[props.state].icon" class="size-5" :class="TONE_TEXT[STATE_LOOK[props.state].tone]" aria-hidden="true" />
    </span>
  </HintTip>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import HintTip from "bmo-ui/app/components/HintTip.vue";

import { STATE_LOOK, TONE_TEXT } from "../../domain/look";

import type { State } from "../../../shared/types";

const props = defineProps<{ state: State }>();

const { t } = useI18n();
const word = computed(() => t(`voc.state.${props.state}`));
</script>
