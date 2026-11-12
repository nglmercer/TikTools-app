import { useI18n } from 'vue-i18n';
import type { CollaborationStatus } from '@/entities/collaboration/api/types.ts';
import type { StatusTone } from '@/shared/ui/status/status-types.ts';

export function useCollaborationPresentation() {
  const { t, te } = useI18n();
  const date = (value: number | null) => value === null ? '—' : new Date(value).toLocaleString();
  const state = (value: string) => te(`collaboration.states.${value}`) ? t(`collaboration.states.${value}`) : value;
  function stateTone(value: string): StatusTone {
    if (value === 'pending') return 'warning';
    if (['approved', 'created', 'redeemed'].includes(value)) return 'success';
    if (['rejected', 'revoked', 'expired'].includes(value)) return 'error';
    return 'neutral';
  }
  function listenerStatus(status: CollaborationStatus): { label: string; tone: StatusTone } {
    if (!status.gateway.running) return { label: t('collaboration.stopped'), tone: 'neutral' };
    return status.configuration.enabled
      ? { label: t('collaboration.running'), tone: 'success' }
      : { label: t('collaboration.applying'), tone: 'warning' };
  }
  return { date, state, stateTone, listenerStatus };
}
