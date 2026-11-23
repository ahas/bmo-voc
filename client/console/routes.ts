import VocList from "../app/pages/voc/index.vue";
import VocReport from "../app/pages/voc/[reportId].vue";

import "./screens.css";

import type { ConsoleBundleModule } from "bmo-ui/shared/console-manifest";

// The console bundle's module (bmo-ui's `docs/console-bundles.md`): the voc screen under its screen path — a sender's
// own reports or the maker's inbox, and one report's thread — under the console's title bar.
const bundle: ConsoleBundleModule = {
  routes: [
    { path: "/voc", name: "voc", component: VocList, meta: { key: "voc", consoleTitleBar: true } },
    { path: "/voc/:reportId", name: "voc-report", component: VocReport, meta: { key: "voc-report", consoleTitleBar: true } }
  ]
};

export default bundle;
