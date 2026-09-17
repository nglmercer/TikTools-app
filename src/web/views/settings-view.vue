<script lang="tsx">
import { IconGlobe, IconMoon, IconSettings, IconSun } from '../components/icons.vue';
import { Card } from '../components/ui/Card.vue';
import { Select } from '../components/ui/Select.vue';
import { Page } from '../components/ui/Page.vue';
import { defineVueComponent } from '../vue/component.ts';
import { t, type Locale } from '../i18n.ts';
import type { Theme } from '../preferences.ts';

type SettingsViewProps = {
  locale: Locale;
  theme: Theme;
  onLocaleChange: (l: Locale) => void;
  onThemeChange: (t: Theme) => void;
};

function ThemePreview({ theme, onThemeChange, darkLabel, lightLabel }: { theme: Theme; onThemeChange: (t: Theme) => void; darkLabel: string; lightLabel: string }) {
  const modes = [
    { value: 'dark' as Theme, label: darkLabel },
    { value: 'light' as Theme, label: lightLabel },
  ];
  return (
    <div class="theme-preview" role="group">
      {modes.map((mode) => (
        <button
          key={mode.value}
          type="button"
          class={`theme-preview__swatch is-${mode.value}${theme === mode.value ? ' is-active' : ''}`}
          onClick={() => onThemeChange(mode.value)}
          aria-pressed={theme === mode.value}
          aria-label={mode.label}
        >
          <span class="theme-preview__bar" aria-hidden="true" />
          <span class="theme-preview__line" aria-hidden="true" />
          <span class="theme-preview__line is-short" aria-hidden="true" />
          <span class="theme-preview__caption">{mode.label}</span>
        </button>
      ))}
    </div>
  );
}

function renderSettingsView({ locale, theme, onLocaleChange, onThemeChange }: SettingsViewProps) {
  return (
    <Page width="medium">
      <div class="ui-cols-2">
        <Card title={t(locale, 'appearance')} icon={<IconSun />}>
          <Select
            id="settings-theme-select"
            name="theme"
            value={theme}
            label={t(locale, 'theme')}
            onValueChange={(v) => onThemeChange(v as Theme)}
            leadingIcon={theme === 'dark' ? <IconMoon size={14} /> : <IconSun size={14} />}
            options={[
              { value: 'dark', label: t(locale, 'dark') },
              { value: 'light', label: t(locale, 'light') },
            ]}
          />
          <ThemePreview
            theme={theme}
            onThemeChange={onThemeChange}
            darkLabel={t(locale, 'dark')}
            lightLabel={t(locale, 'light')}
          />
        </Card>

        <Card title={t(locale, 'application')} icon={<IconSettings />}>
          <Select
            id="settings-language"
            name="language"
            value={locale}
            label={t(locale, 'language')}
            onValueChange={(v) => onLocaleChange(v as Locale)}
            leadingIcon={<IconGlobe size={14} />}
            options={[
              { value: 'en', label: t(locale, 'english') },
              { value: 'es', label: t(locale, 'spanish') },
            ]}
          />
        </Card>
      </div>
    </Page>
  );
}

export const SettingsView = defineVueComponent<SettingsViewProps>(
  ['locale', 'theme', 'onLocaleChange', 'onThemeChange'],
  (props) => () => renderSettingsView(props),
);

export default SettingsView;
</script>
