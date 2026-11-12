import { useI18n } from 'vue-i18n';
import type { RecoveryValue, V2ConflictValue } from './recovery-contracts.ts';

export function useRecoveryLabels() {
  const { t } = useI18n();
  function recoveryValueText(value: RecoveryValue): string {
    if (value.state === 'absent') return t('sync.recoveryAbsent');
    if (value.state === 'deleted') return t('sync.recoveryDeleted');
    return t('sync.recoverySelected', {
      version: value.version,
      status: t(value.enabled ? 'sync.recoveryEnabled' : 'sync.recoveryDisabled'),
    });
  }

  function v2ConflictValueText(value: V2ConflictValue): string {
    if (value.state === 'absent') return t('sync.recoveryAbsent');
    if (value.state === 'deleted') return t('sync.recoveryDeleted');
    if (typeof value.version === 'string' && typeof value.enabled === 'boolean') {
      return t('sync.recoverySelected', {
        version: value.version,
        status: t(value.enabled ? 'sync.recoveryEnabled' : 'sync.recoveryDisabled'),
      });
    }
    const preview = value.value == null ? '' : JSON.stringify(value.value).slice(0, 120);
    return `${t('sync.v2.conflictPresent')}${value.revision != null ? ` (#${value.revision})` : ''}${preview ? `: ${preview}` : ''}`;
  }

  return { recoveryValueText, v2ConflictValueText };
}
