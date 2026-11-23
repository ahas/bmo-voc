// The attendance screen's sources as a Nuxt project on bmo-ui, which `nuxi prepare`, the typecheck and the dev host
// read. A console loads none of it: it attaches the bundle `bun run bundle` builds from the same sources by the
// manifest the service's console face answers (bmo-ui's `docs/console-bundles.md`).
export default defineNuxtConfig({
  extends: ["bmo-ui"],
  compatibilityDate: "2025-09-01",
  devtools: { enabled: false }
});
