// A host of one's own: the smallest Nuxt app that loads this bundle as a console would, for a worker's
// browser check of the screens before the platform console is told about the unit at all (the job report
// names what it stands in for and what it cannot). It is not the console: the frame, the sign-in and the
// owner headers the console sets stand here as fixed development values.
export default defineNuxtConfig({
  extends: [".."],
  compatibilityDate: "2025-09-01",
  devtools: { enabled: false },
  // A console bundle mounts in the browser; this host renders nothing of it on the server.
  ssr: false,
  app: { baseURL: "/" },
  appConfig: {
    bmoUi: {
      // The host states the locale and the zone the layer reads figures and moments in (bmo-ui's
      // `docs/boundary.md`, "What the host owes"); this host shows the bundle's Korean pack.
      locale: "ko-KR",
      zone: "Asia/Seoul",
      // The dev relay: every bundle call goes to /proxy/<service>/... and stands for the console's own
      // relay of the unit, answered by `server/routes/proxy/[...].ts`.
      relayBase: "/proxy",
      // The console's version as a console states it, which the send form takes with a report.
      consoleVersion: "dev-host",
      translator: { service: "", path: "", batch: 4, asksAtOnce: 1 }
    }
  }
});
