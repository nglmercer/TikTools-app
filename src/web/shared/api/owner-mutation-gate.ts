import { computed, shallowRef } from 'vue';
import { ControlCallError, type ControlClient } from './control-client.ts';

export type OwnerMutation = 'definition' | 'adoption' | 'approval';
function createGate() {
  const owner = shallowRef<OwnerMutation | null>(null);
  const busy = computed(() => owner.value !== null);
  async function run<T>(operation: OwnerMutation, work: () => Promise<T>): Promise<T> {
    if (owner.value !== null) throw new ControlCallError('command_pending', 'An Owner operation is already in progress.');
    owner.value = operation;
    try { return await work(); }
    finally { owner.value = null; }
  }
  return { owner, busy, run };
}
const gates = new WeakMap<ControlClient, ReturnType<typeof createGate>>();
/** Connection identity is the original injected client, never a subscription wrapper. */
export function ownerMutationGate(control: ControlClient) {
  let gate = gates.get(control);
  if (!gate) { gate = createGate(); gates.set(control, gate); }
  return gate;
}
