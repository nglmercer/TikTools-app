import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { PRESET_ROLES } from '@/entities/cloud-teams/api/teams-api.ts';

export function useTeamLabels() {
  const { t, te } = useI18n();
  const roleOptions = computed(() =>
    PRESET_ROLES.map((role) => ({ value: role, label: t(`cloudTeams.roles.${role}`) })),
  );
  const ttlOptions = computed(() => [
    { value: 300, label: t('cloudTeams.ttl.minutes', { count: 5 }) },
    { value: 900, label: t('cloudTeams.ttl.minutes', { count: 15 }) },
  ]);
  /** Known statuses are translated; anything else is shown as the server sent it. */
  function statusLabel(prefix: string, value: string): string {
    const key = `${prefix}.${value}`;
    return te(key) ? t(key) : value;
  }

  return { roleOptions, ttlOptions, statusLabel };
}
