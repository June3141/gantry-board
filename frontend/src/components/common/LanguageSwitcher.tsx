import { useTranslation } from 'react-i18next';

const LANGUAGES = [
  { code: 'en', labelKey: 'language.en' },
  { code: 'ja', labelKey: 'language.ja' },
] as const;

export function LanguageSwitcher() {
  const { t, i18n } = useTranslation();

  return (
    <select
      value={i18n.resolvedLanguage ?? i18n.language}
      onChange={(e) => i18n.changeLanguage(e.target.value)}
      aria-label={t('language.switchLanguage')}
      className="rounded border border-input bg-background px-2 py-1 text-xs text-foreground"
    >
      {LANGUAGES.map((lang) => (
        <option key={lang.code} value={lang.code}>
          {t(lang.labelKey)}
        </option>
      ))}
    </select>
  );
}
