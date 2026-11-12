import type { PluginConnectionState } from '../../../../automation/plugins/declarative.ts';
import type { ActionOptionItem, OpenMediaPicker } from '../../../../shared/messages.ts';
import type { PluginCredentialState } from '../../../../shared/token-import.ts';
import type { PluginSettingsState } from '@/types.ts';

export interface PluginConnectionEditorProps {
  pluginId: string;
  pluginName: string;
  settings?: PluginSettingsState;
  connection?: PluginConnectionState;
  actionOptions: Readonly<Record<string, ActionOptionItem[]>>;
  supportsProvisioning?: boolean;
  /** Guided `service-info` connection; inferred from a probe result when omitted. */
  guided?: boolean;
  supportsTokenImport?: boolean;
  supportsTokenDiscovery?: boolean;
  provisionState?: PluginCredentialState;
  openMediaPicker?: OpenMediaPicker;
  /** `sections` puts connection and default settings side by side, as on the server connections page. */
  layout?: 'stacked' | 'sections';
}
