<script lang="tsx">
import { computed, onMounted, ref, watch } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import { Card } from '../../components/ui/Card.vue';
import { TextField } from '../../components/ui/fields/TextField.vue';
import { Tooltip } from '../../components/ui/Tooltip.vue';
import { Icon, IconClose } from '../../components/icons/index.ts';
import { isGlobalKey } from '../../features/globals.ts';
import { t, type Locale } from '../../i18n.ts';

type GlobalsRow = { key: string; value: string };

type GlobalsCardProps = {
  locale: Locale;
  globals: Record<string, string>;
  loading: boolean;
  error: string | null;
  onLoad: () => Promise<void>;
  onSave: (next: Record<string, string>) => Promise<void>;
};

const toRows = (globals: Record<string, string>): GlobalsRow[] => (
  Object.entries(globals)
    .sort(([left], [right]) => (left < right ? -1 : left > right ? 1 : 0))
    .map(([key, value]) => ({ key, value }))
);

/**
 * Runtime globals editor: key/value rows with host-mirrored key
 * validation, saved as one draft (deletions + writes in key order).
 * Fully-empty rows are ignored; half-empty or duplicate keys block save.
 */
export const GlobalsCard = defineVueComponent<GlobalsCardProps>(
  ['locale', 'globals', 'loading', 'error', 'onLoad', 'onSave'],
  (props) => {
    const rows = ref<GlobalsRow[]>(toRows(props.globals));
    const dirty = ref(false);
    const saving = ref(false);

    watch(() => props.globals, (next) => {
      if (!dirty.value) rows.value = toRows(next);
    });
    onMounted(() => {
      void props.onLoad();
    });

    const counts = computed(() => {
      const seen = new Map<string, number>();
      for (const row of rows.value) {
        if (row.key === '') continue;
        seen.set(row.key, (seen.get(row.key) ?? 0) + 1);
      }
      return seen;
    });

    const rowError = (row: GlobalsRow, locale: Locale): string | null => {
      if (row.key === '') return row.value === '' ? null : t(locale, 'runtimeGlobalsEmptyKey');
      if (!isGlobalKey(row.key)) return t(locale, 'runtimeGlobalsInvalidKey');
      if ((counts.value.get(row.key) ?? 0) > 1) return t(locale, 'runtimeGlobalsDuplicateKey');
      return null;
    };

    const blocked = computed(() => rows.value.some((row) => rowError(row, props.locale) !== null));

    const setRow = (index: number, patch: Partial<GlobalsRow>): void => {
      rows.value = rows.value.map((row, current) => (current === index ? { ...row, ...patch } : row));
      dirty.value = true;
    };

    const removeRow = (index: number): void => {
      rows.value = rows.value.filter((_, current) => current !== index);
      dirty.value = true;
    };

    const addRow = (): void => {
      rows.value = [...rows.value, { key: '', value: '' }];
    };

    const save = async (): Promise<void> => {
      if (blocked.value || saving.value) return;
      const next: Record<string, string> = {};
      for (const row of rows.value) {
        if (row.key === '') continue;
        next[row.key] = row.value;
      }
      saving.value = true;
      try {
        await props.onSave(next);
        dirty.value = false;
      } catch {
        // The failure surfaces through `error`; the draft stays editable.
      } finally {
        saving.value = false;
      }
    };

    return () => {
      const locale = props.locale;
      return (
        <Card
          title={t(locale, 'runtimeGlobals')}
          hint={t(locale, 'runtimeGlobalsHint')}
          icon={<Icon name="code" size={16} />}
        >
          {rows.value.length === 0 && (
            <p class="plg-empty__desc">{t(locale, 'runtimeGlobalsEmpty')}</p>
          )}
          {rows.value.map((row, index) => (
            <div class="plg-kv-row" key={`global-${index}`}>
              <Tooltip text={t(locale, 'runtimeGlobalsKey')} position="right">
                <TextField
                  value={row.key}
                  onValueChange={(key) => setRow(index, { key })}
                  ariaLabel={t(locale, 'runtimeGlobalsKey')}
                  placeholder="commandPort"
                  locale={locale}
                  className="field--mono"
                  error={rowError(row, locale) ?? undefined}
                />
              </Tooltip>
              <span class="plg-kv-row__value">
                <TextField
                  value={row.value}
                  onValueChange={(value) => setRow(index, { value })}
                  ariaLabel={t(locale, 'runtimeGlobalsValue')}
                  placeholder="8080"
                  locale={locale}
                  className="field--mono"
                />
              </span>
              <Tooltip text={t(locale, 'cancel')} position="left">
                <button
                  type="button"
                  class="plg-btn plg-btn--icon plg-btn--danger"
                  aria-label={t(locale, 'cancel')}
                  onClick={() => removeRow(index)}
                >
                  <IconClose size={10} />
                </button>
              </Tooltip>
            </div>
          ))}
          {props.error && <div class="plg-alert">{props.error}</div>}
          <div class="plg-row">
            <Tooltip text={t(locale, 'runtimeGlobalsAdd')} position="bottom">
              <button type="button" class="plg-btn plg-btn--sm" onClick={addRow}>
                + {t(locale, 'add')}
              </button>
            </Tooltip>
            <button
              type="button"
              class="plg-btn plg-btn--sm plg-btn--primary"
              disabled={blocked.value || saving.value || props.loading || !dirty.value}
              onClick={() => void save()}
            >
              {saving.value ? t(locale, 'runtimeGlobalsSaving') : t(locale, 'runtimeGlobalsSave')}
            </button>
          </div>
        </Card>
      );
    };
  },
);

export default GlobalsCard;
</script>
