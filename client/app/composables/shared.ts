import { effectScope } from "vue";
import { useNuxtApp } from "nuxt/app";

// A composable held once to the app rather than once to the component: the screens of one bundle are several
// pages over one store, and what they hold is theirs together. A scope of its own keeps the store's watchers
// alive as long as the app stands and gone when it goes.

export function createAppComposable<T>(composable: () => T): () => T {
  const instances = new WeakMap<object, T>();

  return () => {
    const nuxtApp = useNuxtApp();

    if (!instances.has(nuxtApp)) instances.set(nuxtApp, effectScope(true).run(composable)!);

    return instances.get(nuxtApp)!;
  };
}
