/**
 * i18n.ts — i18next configuration for Nexus web client.
 *
 * Uses:
 * - i18next-http-backend: loads translations from /public/locales/<lang>/translation.json
 * - i18next-browser-languagedetector: detects user's preferred language from
 *   browser settings, localStorage, and URL query params (?lng=de)
 *
 * Supported languages (community-contributed; English is source of truth):
 *   en  — English (base)
 *   de  — German
 *   es  — Spanish
 *   fr  — French
 *   ja  — Japanese
 *   pt  — Portuguese
 *
 * Usage in components:
 *   import { useTranslation } from 'react-i18next';
 *   const { t } = useTranslation();
 *   t('chat.messagePlaceholder', { channel: 'general' })
 *
 * Adding a new language:
 *   1. Copy public/locales/en/translation.json to public/locales/<code>/translation.json
 *   2. Translate all values (keep keys identical)
 *   3. Add the language code to the `supportedLngs` array below
 */

import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import HttpBackend from 'i18next-http-backend';
import LanguageDetector from 'i18next-browser-languagedetector';

i18n
  // Load translations from /public/locales via HTTP (no bundle bloat)
  .use(HttpBackend)
  // Auto-detect user's language preference
  .use(LanguageDetector)
  // Pass the i18n instance to react-i18next
  .use(initReactI18next)
  .init({
    // Fallback to English when a translation key is missing
    fallbackLng: 'en',

    // Only these languages are officially supported (others fall back to English)
    supportedLngs: ['en', 'de', 'es', 'fr', 'ja', 'pt'],

    // Namespace — single namespace for the web client
    defaultNS: 'translation',
    ns: ['translation'],

    // Language detection order: URL query → localStorage → browser
    detection: {
      order: ['querystring', 'localStorage', 'navigator'],
      lookupQuerystring: 'lng',
      lookupLocalStorage: 'nexus:language',
      caches: ['localStorage'],
    },

    backend: {
      // Path to translation files in the public directory
      loadPath: '/locales/{{lng}}/{{ns}}.json',
    },

    interpolation: {
      // React already escapes values — no need to double-escape
      escapeValue: false,
    },

    // Return the key path as-is for missing translations (visible in dev)
    // In production this means English text shows for untranslated languages.
    parseMissingKeyHandler: (key: string) => key,

    // Enable React Suspense support for lazy-loading translations
    react: {
      useSuspense: true,
    },
  });

export default i18n;
