<script setup lang="ts">
// The dev host's own frame: the app root the layer asks its host for, a line saying what this is, and the
// bundle's pages rendering inside it. UApp is the overlay provider the question module opens through and the
// tooltip setting the layer holds; MessageIsland is the one place news is said, as the console says it; the two page
// layers show a cut line whole and name a button the library draws with its name as markup, as the console stands them.
import { ko } from "@nuxt/ui/locale";
import CutLineTip from "bmo-ui/app/components/CutLineTip.vue";
import MarkupButtonTip from "bmo-ui/app/components/MarkupButtonTip.vue";
import MessageIsland from "bmo-ui/app/components/MessageIsland.vue";
import { TOOLTIP_PROVIDER } from "bmo-ui/app/domain/tooltip";
</script>

<template>
  <UApp :locale="ko" :tooltip="TOOLTIP_PROVIDER" :toaster="null">
    <div class="flex h-dvh min-h-0 flex-col">
      <div class="flex h-10 shrink-0 items-center gap-2 border-b border-default bg-elevated px-3 text-sm text-muted">
        <NuxtLink to="/voc" class="font-medium text-highlighted">bmo-voc dev host</NuxtLink>
        <span class="min-w-0 truncate">the bundle under a host of its own; the console relays at <code>/proxy/</code></span>
      </div>

      <div class="flex min-h-0 flex-1">
        <NuxtPage />
      </div>
    </div>

    <CutLineTip />
    <MarkupButtonTip />
    <MessageIsland />
  </UApp>
</template>
