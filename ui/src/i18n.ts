import i18n from "i18next"
import { initReactI18next } from "react-i18next"
import en from "./locales/en.json"
import fa from "./locales/fa.json"

// Add a language: drop a JSON file in ./locales and register it here.
export const resources = { en: { translation: en }, fa: { translation: fa } } as const

i18n.use(initReactI18next).init({
  resources,
  lng: "en",
  fallbackLng: "en",
  interpolation: { escapeValue: false },
})

/** Full mirrored UI when the app language is RTL. */
export function applyDirection(lang: string) {
  document.documentElement.dir = lang === "fa" ? "rtl" : "ltr"
  document.documentElement.lang = lang
}

export default i18n
