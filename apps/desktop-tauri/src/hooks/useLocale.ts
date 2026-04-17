// Re-export so consumers can import from `hooks/useLocale` following the
// existing hook naming convention. Implementation lives in the provider
// module so the context/hook share a single source of truth.
export { useLocale } from "../i18n/LocaleProvider";
