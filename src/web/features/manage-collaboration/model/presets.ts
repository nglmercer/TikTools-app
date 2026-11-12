import { viewerGrant, type Grant } from '@/entities/collaboration/api/types.ts';

export type Preset = 'viewer' | 'workflow' | 'agent';
/** Starting points for a new invitation. Workflow and agent presets start with no workflows: scope is opt-in, never implied. */
export function presetGrant(kind: Preset): Grant {
  const value = viewerGrant();
  if (kind !== 'viewer') { value.capabilities.push('workflows.read'); value.resources.workflow = []; }
  if (kind === 'agent') value.capabilities.push('commands.read');
  return value;
}
