<template>
  <!-- The frame every voc view stands in (`docs/design.md`, 「The console」): a title bar holding the screen's name and
       mark alone — the console's where it offers one, the unit's own otherwise — then the screen's own row flush under
       it: the way back at its left end where the view was gone into, what the view names there, and at its right end
       the refresh key and the view's actions. -->
  <UDashboardPanel :id="props.id" :ui="{ body: 'p-0 sm:p-0 gap-0 sm:gap-0 overflow-y-hidden [&>*]:max-w-none' }">
    <template v-if="!consoleBar" #header>
      <!-- Drawn only where no console stands around the screen, so there is no sidebar for it to open. -->
      <UDashboardNavbar :title="$t('voc.title')" icon="i-material-symbols-campaign-outline" :toggle="false" />
    </template>

    <template #body>
      <ToolBar ref="bar" :label="$t('voc.aria.toolbar')">
        <div v-if="props.back !== null">
          <IconButton :label="$t('voc.action.back')" icon="i-material-symbols-arrow-back" :to="props.back" />
        </div>

        <slot :short="short" />

        <!-- The first cell of the right end carries the margin that pushes it and those after it there. -->
        <div class="ms-auto">
          <IconButton :label="$t('voc.action.refresh')" icon="i-material-symbols-refresh" :loading="props.refreshing" @click="emit('refresh')" />
        </div>

        <slot name="actions" :short="short" />
      </ToolBar>

      <slot name="body" />
    </template>
  </UDashboardPanel>
</template>

<script setup lang="ts">
import { inject, useTemplateRef } from "vue";
import IconButton from "bmo-ui/app/components/IconButton.vue";
import ToolBar from "bmo-ui/app/components/ToolBar.vue";
import { useBarGathers } from "bmo-ui/app/domain/actionbar";

import type { RouteLocationRaw } from "vue-router";

interface Props {
  /** The panel's own id, which the console's frame keys the screen by. */
  id: string;
  /** Where Back on the row leads, where the view was gone into from another. */
  back?: RouteLocationRaw | null;
  refreshing?: boolean;
}

const props = withDefaults(defineProps<Props>(), { back: null, refreshing: false });

const emit = defineEmits<{ refresh: [] }>();

// Whether the row is short of room, read off the row itself as every bar reads it, so a view folds what it names
// there to its compact shape before the row would have to scroll.
const bar = useTemplateRef<{ $el: HTMLElement }>("bar");
const short = useBarGathers(() => bar.value?.$el);

// Whether a console draws its title bar over the screen, under the key bmo-ui's bundle contract names.
const consoleBar = inject<object | null>("bmo-console-title-bar", null) !== null;
</script>
