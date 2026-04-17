import { type ReactNode } from "react";
import { ALL_LOCALE_KEYS } from "../i18n/keys";
import type { Language, LocaleStrings } from "../types/bridge";

/**
 * Build a complete `LocaleStrings` bundle whose entries are the key names
 * themselves, so tests can assert on "the translator returned *something*
 * that corresponds to this key" without caring about EN/ZH wording.
 */
export function buildBundle(
  overrides: Partial<Record<(typeof ALL_LOCALE_KEYS)[number], string>> = {},
  language: Language = "english",
): LocaleStrings {
  const entries: Record<string, string> = {};
  for (const key of ALL_LOCALE_KEYS) {
    entries[key] = overrides[key] ?? key;
  }
  return { language, entries };
}

export function wrapChildren(children: ReactNode): ReactNode {
  return children;
}
