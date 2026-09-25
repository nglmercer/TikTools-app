<script lang="tsx">
import { IconGlobe, IconMoon, IconSettings, IconSun } from '../components/icons.vue';
import { Card } from '../components/ui/Card.vue';
import { Select } from '../components/ui/Select.vue';
import { Page } from '../components/ui/Page.vue';
import { defineVueComponent } from '../vue/component.ts';
import { t, type Locale } from '../i18n.ts';
import type { Theme } from '../preferences.ts';
import { GlobalsCard } from './settings/GlobalsCard.vue';

type SettingsViewProps = {
  locale: Locale;
  theme: Theme;
  onLocaleChange: (l: Locale) => void;
  onThemeChange: (t: Theme) => void;
  globals: Record<string, string>;
  globalsLoading: boolean;
  globalsError: string | null;
  onLoadGlobals: () => Promise<void>;
  onSaveGlobals: (next: Record<string, string>) => Promise<void>;
};

/**
 * Settings: one explicit theme selector (the top-nav button stays as the
 * global shortcut). No duplicated Dark/Light preview buttons.
 */
function renderSettingsView({ locale, theme, onLocaleChange, onThemeChange, globals, globalsLoading, globalsError, onLoadGlobals, onSaveGlobals }: SettingsViewProps) {
  return (
    <Page width="medium">
      <div class="ui-cols-2">
        <Card title={t(locale, 'appearance')} icon={<IconSun />}>
          <Select
            id="settings-theme-select"
            name="theme"
            value={theme}
            label={t(locale, 'theme')}
            hint={t(locale, 'switchTheme')}
            onValueChange={(v) => onThemeChange(v as Theme)}
            leadingIcon={theme === 'dark' ? <IconMoon size={14} /> : <IconSun size={14} />}
            options={[
              { value: 'dark', label: t(locale, 'dark') },
              { value: 'light', label: t(locale, 'light') },
            ]}
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
      <GlobalsCard
        locale={locale}
        globals={globals}
        loading={globalsLoading}
        error={globalsError}
        onLoad={onLoadGlobals}
        onSave={onSaveGlobals}
      />
    </Page>
  );
}

export const SettingsView = defineVueComponent<SettingsViewProps>(
  ['locale', 'theme', 'onLocaleChange', 'onThemeChange', 'globals', 'globalsLoading', 'globalsError', 'onLoadGlobals', 'onSaveGlobals'],
  (props) => () => renderSettingsView(props),
);

export default SettingsView;
</script>
