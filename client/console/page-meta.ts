// A page's meta, which the bundle's routes carry themselves (`routes.ts`). In a Nuxt build the call is
// the compiler's hint and is taken out; in the bundle it stands and does nothing.
export function definePageMeta(_meta: Record<string, unknown>): void {}
