<script lang="tsx">
import { onBeforeUnmount, ref } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import { Icon } from '../../components/icons/Icon.vue';
import { Button } from '../../components/ui/Button.vue';
import { TextInput } from '../../components/ui/TextInput.vue';
import { useDialogs } from '../../composables/useDialogs.ts';
import { t, type Locale } from '../../i18n.ts';
import { DEFAULT_PROFILE_ID, type ProfilePack } from '../../features/rule-profiles.ts';

export type ProfileSelectProps = {
  locale: Locale;
  /** Resolved packs (default included) with live rule counts. */
  packs: ProfilePack[];
  activeId: string;
  error: string;
  onSwitch: (id: string) => Promise<void>;
  onCreate: (name: string) => Promise<void>;
  onDelete: (id: string) => Promise<void>;
  onExport: (id: string) => void;
  onOpenImport: () => void;
};

/**
 * Custom profile selector: the trigger shows the live profile and the menu
 * switches, creates, imports, exports, and deletes profiles in place, so
 * profile management lives in the selector instead of the templates modal.
 */
export const ProfileSelect = defineVueComponent<ProfileSelectProps>(
  ['locale', 'packs', 'activeId', 'error', 'onSwitch', 'onCreate', 'onDelete', 'onExport', 'onOpenImport'],
  (props) => {
    const dialogs = useDialogs();
    const open = ref(false);
    const rootRef = ref<HTMLDivElement | null>(null);
    const triggerRef = ref<HTMLButtonElement | null>(null);
    const draftName = ref('');
    const switchingId = ref<string | null>(null);
    const creating = ref(false);
    const deletingId = ref<string | null>(null);

    const close = (restoreFocus = false): void => {
      open.value = false;
      if (restoreFocus) triggerRef.value?.focus();
    };

    const onOutside = (event: Event): void => {
      const target = event.target as Node | null;
      if (rootRef.value && target && rootRef.value.contains(target)) return;
      close(false);
    };
    if (typeof document !== 'undefined') document.addEventListener('pointerdown', onOutside, true);
    onBeforeUnmount(() => {
      if (typeof document !== 'undefined') document.removeEventListener('pointerdown', onOutside, true);
    });

    const displayName = (pack: ProfilePack): string =>
      pack.id === DEFAULT_PROFILE_ID ? t(props.locale, 'behavior.copy.proDefault') : pack.name;

    const runSwitch = async (id: string): Promise<void> => {
      if (switchingId.value || id === props.activeId) return;
      switchingId.value = id;
      try {
        await props.onSwitch(id);
        close(false);
      } catch {
        // Surfaced through the error prop; the menu stays open.
      } finally {
        switchingId.value = null;
      }
    };

    const runCreate = async (): Promise<void> => {
      const name = draftName.value.trim();
      if (name === '' || creating.value) return;
      creating.value = true;
      try {
        await props.onCreate(name);
        draftName.value = '';
      } catch {
        // Surfaced through the error prop; the draft stays editable.
      } finally {
        creating.value = false;
      }
    };

    const runDelete = async (pack: ProfilePack): Promise<void> => {
      if (deletingId.value) return;
      const confirmed = await dialogs.confirm(
        t(props.locale, 'behavior.copy.proDeleteMessage', {
          name: pack.name,
          events: pack.eventIds.length,
          actions: pack.actionIds.length,
        }),
        {
          title: t(props.locale, 'behavior.copy.proDeleteTitle'),
          confirmLabel: t(props.locale, 'behavior.copy.remove'),
          cancelLabel: t(props.locale, 'cancel'),
          danger: true,
        },
      );
      if (!confirmed) return;
      deletingId.value = pack.id;
      try {
        await props.onDelete(pack.id);
      } catch {
        // Surfaced through the error prop; the menu stays open.
      } finally {
        deletingId.value = null;
      }
    };

    return () => {
      const locale = props.locale;
      const active = props.packs.find((pack) => pack.id === props.activeId) ?? null;
      return (
        <div ref={rootRef} class="rule-profile-select">
          <button
            ref={triggerRef}
            type="button"
            class="rule-profile-select__trigger"
            aria-haspopup="menu"
            aria-expanded={open.value}
            aria-label={t(locale, 'behavior.copy.proSelectAria')}
            onClick={() => { open.value = !open.value; }}
            onKeydown={(event: KeyboardEvent) => {
              if (event.key === 'Escape') {
                event.preventDefault();
                close(true);
              } else if ((event.key === 'ArrowDown' || event.key === 'Enter') && !open.value) {
                event.preventDefault();
                open.value = true;
              }
            }}
          >
            <span class="rule-profile-select__name">{active ? displayName(active) : ''}</span>
            {active && (
              <span class="rule-profile-select__counts">
                {t(locale, 'behavior.copy.proCounts', { events: active.eventIds.length, actions: active.actionIds.length })}
              </span>
            )}
            <span class="rule-profile-select__chevron" aria-hidden="true"><Icon name="arrow-down" size={12} /></span>
          </button>
          {open.value && (
            <div
              class="rule-profile-select__menu"
              role="menu"
              aria-label={t(locale, 'behavior.copy.proSelectAria')}
              onKeydown={(event: KeyboardEvent) => {
                if (event.key === 'Escape') {
                  event.preventDefault();
                  close(true);
                }
              }}
            >
              {props.error ? <p class="rule-profile-select__error" role="alert">{props.error}</p> : null}
              <ul class="rule-profile-select__list">
                {props.packs.map((pack) => {
                  const isActive = pack.id === props.activeId;
                  const switching = switchingId.value === pack.id;
                  const deleting = deletingId.value === pack.id;
                  return (
                    <li key={pack.id} class={`rule-profile-select__row${isActive ? ' is-active' : ''}`} role="none">
                      <button
                        type="button"
                        role="menuitemradio"
                        aria-checked={isActive}
                        class="rule-profile-select__option"
                        disabled={switching}
                        onClick={() => void runSwitch(pack.id)}
                      >
                        <span class="rule-profile-select__option-check" aria-hidden="true">
                          {isActive ? <Icon name="check" size={12} /> : null}
                        </span>
                        <span class="rule-profile-select__option-text">
                          <strong>
                            {displayName(pack)}
                            {isActive && (
                              <span class="rule-profile-select__badge">{t(locale, 'behavior.copy.proActiveBadge')}</span>
                            )}
                          </strong>
                          <small>
                            {t(locale, 'behavior.copy.proCounts', { events: pack.eventIds.length, actions: pack.actionIds.length })}
                          </small>
                        </span>
                      </button>
                      <span class="rule-profile-select__row-actions">
                        <Button variant="ghost" size="sm" onClick={() => props.onExport(pack.id)}>
                          {t(locale, 'behavior.copy.proExport')}
                        </Button>
                        {pack.id !== DEFAULT_PROFILE_ID && (
                          <Button variant="ghost" size="sm" loading={deleting} onClick={() => void runDelete(pack)}>
                            {t(locale, 'behavior.copy.tplDelete')}
                          </Button>
                        )}
                      </span>
                    </li>
                  );
                })}
              </ul>
              <div class="rule-profile-select__divider" aria-hidden="true" />
              <div class="rule-profile-select__create">
                <span class="rule-profile-select__create-input">
                  <TextInput
                    value={draftName.value}
                    onValueChange={(next) => { draftName.value = next; }}
                    placeholder={t(locale, 'behavior.copy.proCreatePlaceholder')}
                    name="rule-profile-name"
                  />
                </span>
                <Button variant="soft" size="md" loading={creating.value} onClick={() => void runCreate()}>
                  {t(locale, 'behavior.copy.proCreate')}
                </Button>
              </div>
              <Button variant="soft" size="md" onClick={() => { close(false); props.onOpenImport(); }}>
                {t(locale, 'behavior.copy.proImport')}
              </Button>
            </div>
          )}
        </div>
      );
    };
  },
);

export default ProfileSelect;
</script>

<style scoped>
.rule-profile-select {
  position: relative;
}

.rule-profile-select__trigger {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  max-width: 280px;
  padding: 6px 10px;
  border: 1px solid var(--tt-border, #34343f);
  border-radius: 8px;
  background: var(--tt-bg-soft, #1c1c26);
  color: inherit;
  font: inherit;
  cursor: pointer;
}

.rule-profile-select__trigger:hover {
  border-color: var(--tt-accent, #22c55e);
}

.rule-profile-select__name {
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.rule-profile-select__counts {
  font-size: 11px;
  color: var(--tt-text-dim, #a8a8b8);
  white-space: nowrap;
}

.rule-profile-select__chevron {
  display: inline-flex;
  margin-left: auto;
  color: var(--tt-text-dim, #a8a8b8);
}

.rule-profile-select__menu {
  position: absolute;
  top: calc(100% + 6px);
  left: 0;
  z-index: var(--z-popover, 800);
  display: flex;
  flex-direction: column;
  gap: 8px;
  width: 340px;
  max-width: min(340px, 80vw);
  max-height: min(420px, 60vh);
  overflow-y: auto;
  padding: 10px;
  border: 1px solid var(--tt-border, #34343f);
  border-radius: 12px;
  background: var(--tt-bg-soft, #1c1c26);
  box-shadow: 0 12px 32px rgb(0 0 0 / 45%);
}

.rule-profile-select__error {
  margin: 0;
  color: var(--tt-danger, #ef4444);
  font-size: 13px;
}

.rule-profile-select__list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.rule-profile-select__row {
  display: flex;
  align-items: center;
  gap: 4px;
  border-radius: 8px;
}

.rule-profile-select__row:hover {
  background: var(--tt-bg, #17171f);
}

.rule-profile-select__option {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  padding: 6px;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.rule-profile-select__option:disabled {
  cursor: default;
  opacity: 0.6;
}

.rule-profile-select__option-check {
  display: inline-flex;
  width: 14px;
  flex: none;
  color: var(--tt-accent, #22c55e);
}

.rule-profile-select__option-text {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.rule-profile-select__option-text strong {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 13px;
}

.rule-profile-select__option-text small {
  color: var(--tt-text-dim, #a8a8b8);
  font-size: 11px;
}

.rule-profile-select__badge {
  display: inline-block;
  margin-left: 6px;
  padding: 1px 7px;
  border-radius: 999px;
  font-size: 11px;
  font-weight: 600;
  background: var(--tt-bg, #17171f);
  color: var(--tt-text-dim, #a8a8b8);
  vertical-align: 1px;
}

.rule-profile-select__row-actions {
  display: flex;
  gap: 2px;
  flex: none;
}

.rule-profile-select__divider {
  border-top: 1px solid var(--tt-border, #34343f);
}

.rule-profile-select__create {
  display: flex;
  gap: 8px;
  align-items: center;
}

.rule-profile-select__create-input {
  flex: 1;
  min-width: 0;
}
</style>
