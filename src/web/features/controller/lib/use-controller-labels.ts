import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { MessageSchema } from '@/shared/i18n/i18n.ts';

export function useControllerLabels() {
  const { tm } = useI18n();
  const labels = computed(() => tm('controller.root.labels') as MessageSchema['controller']['root']['labels']);
  const forgetLabels = computed(() => tm('controller.root.forget') as MessageSchema['controller']['root']['forget']);
  const cmd = computed(() => tm('controller.root.commands') as MessageSchema['controller']['root']['commands']);
  const pts = computed(() => tm('controller.root.points') as MessageSchema['controller']['root']['points']);
  const dft = computed(() => tm('controller.root.drafts') as MessageSchema['controller']['root']['drafts']);
  return { labels, forgetLabels, cmd, pts, dft };
}
