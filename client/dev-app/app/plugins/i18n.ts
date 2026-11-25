// The dev host speaks to its screens with the bundle's own words and the few of the layer's `common.*` words the
// screens draw, which a console answers from its own pack: vue-i18n stood up by hand, the bundle's pack read from
// its i18n folder. The common words here stand only in this host, for a browser check to read real words;
// the Japanese and Chinese packs stand beside them, reading the common words in English.

import { createI18n } from "vue-i18n";

import en from "../../../i18n/en.json";
import ja from "../../../i18n/ja.json";
import ko from "../../../i18n/ko.json";
import zhCN from "../../../i18n/zh-CN.json";

const COMMON = {
  ko: {
    label: { seconds: "{n}초", minutes: "{m}분", hours: "{h}시간", days: "{d}일", hours_minutes: "{h}시간 {m}분", days_hours: "{d}일 {h}시간", minutes_seconds: "{m}분 {s}초", properties: "속성" },
    action: { refresh: "새로고침", retry: "다시 시도", cancel: "취소", save: "저장", discard: "버리기", close: "닫기", more: "더보기", zoom_in: "확대", zoom_out: "축소", fit_screen: "화면에 맞추기", previous_picture: "이전 사진", next_picture: "다음 사진" },
    aria: { search_named: "{name} 검색", column_width: "{name} 너비", close_named: "{name} 닫기", image_zoom: "{title}, 휠이나 두 손가락으로 확대", more_notices: "알림 {n}건 더 있음" },
    title: { discard_changes: "저장하지 않은 변경을 버릴까요?" },
    status: { loading: "불러오는 중…", no_match: "찾은 항목 없음", no_choices: "항목 없음", picture_loading: "사진 불러오는 중…" },
    placeholder: { search: "검색" },
    error: { choices_find_failed: "검색하지 못했습니다", picture_failed: "사진을 불러오지 못했습니다." }
  },
  en: {
    label: { seconds: "{n}s", minutes: "{m}m", hours: "{h}h", days: "{d}d", hours_minutes: "{h}h {m}m", days_hours: "{d}d {h}h", minutes_seconds: "{m}m {s}s", properties: "Properties" },
    action: { refresh: "Refresh", retry: "Try again", cancel: "Cancel", save: "Save", discard: "Discard", close: "Close", more: "More", zoom_in: "Zoom in", zoom_out: "Zoom out", fit_screen: "Fit to screen", previous_picture: "Previous picture", next_picture: "Next picture" },
    aria: { search_named: "Search {name}", column_width: "{name} width", close_named: "Close {name}", image_zoom: "{title}, zoom with the wheel or two fingers", more_notices: "{n} more notices" },
    title: { discard_changes: "Discard unsaved changes?" },
    status: { loading: "Loading…", no_match: "Nothing found", no_choices: "No choices", picture_loading: "Loading picture…" },
    placeholder: { search: "Search" },
    error: { choices_find_failed: "Search failed", picture_failed: "The picture could not be loaded." }
  }
};

export default defineNuxtPlugin({
  name: "voc-dev-i18n",
  enforce: "pre",
  setup(nuxtApp) {
    nuxtApp.vueApp.use(
      createI18n({ legacy: false, globalInjection: true, locale: "ko", fallbackLocale: "en", messages: { en: { ...en, common: COMMON.en }, ko: { ...ko, common: COMMON.ko }, ja: { ...ja, common: COMMON.en }, "zh-CN": { ...zhCN, common: COMMON.en } } })
    );
  }
});
