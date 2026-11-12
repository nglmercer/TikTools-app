import { computed, shallowRef, type ComputedRef } from 'vue';
import type { BehaviorSnapshot, PluginPageDescriptor } from '../../../../automation/behavior/types.ts';
import { applySnapshotContributions, syncAutocompletePluginStates } from '../../../../automation/autocomplete-registry.ts';
import { setPluginEventTypes } from '../../../../automation/event-registry.ts';
import { mergePluginAutocomplete, mergePluginPages, mergePluginUis } from '../../../../automation/plugins/declarative.ts';
import { mergePluginMessages } from '@/shared/i18n/i18n.ts';
import type { LibraryJournal } from '@/entities/rule-template/model/library-journal.ts';
import { recordKey, type AutomationHost, type AutomationKind, type AutomationRevision } from './record-versions.ts';

/** Coherent body/version/host projection. Refresh validity stays with the root. */
export function createAutomationProjection(library: Pick<LibraryJournal, 'snapshot'>) {
  const initialSnapshot: BehaviorSnapshot = {
    actions: [],
    events: [],
    plugins: [],
    actionTypes: [],
    translations: {},
  };
  const projection = shallowRef<{ snapshot: BehaviorSnapshot; versions: ReadonlyMap<string, string>; persistent: boolean | null; host: AutomationHost | null }>({
    snapshot: initialSnapshot, versions: new Map(), persistent: null, host: null,
  });
  const behavior = computed({
    get: () => projection.value.persistent !== false && library.snapshot.value ? library.snapshot.value.definitions : projection.value.snapshot,
    // Legacy test/embedding assignment is a body-only projection, never a
    // source of checked-write authority for an unrelated version map.
    set: (snapshot: BehaviorSnapshot) => { library.snapshot.value = null; projection.value = { snapshot, versions: new Map(), persistent: null, host: null }; },
  });
  const behaviorVersions = computed(() => projection.value.persistent !== false && library.snapshot.value
    ? new Map(library.snapshot.value.resourceVersions.map((version) => [recordKey(version.scope as AutomationKind, version.id), version.revision])) : projection.value.versions);
  const persistent = computed(() => projection.value.persistent);
  const sourceHost = computed(() => projection.value.persistent !== false && library.snapshot.value ?
    { hostId: library.snapshot.value.hostId, hostInstanceId: library.snapshot.value.hostInstanceId } : projection.value.host);
  const revisionFor = (kind: AutomationKind, id: string): AutomationRevision | undefined =>
    behaviorVersions.value.get(recordKey(kind, id)) ?? (persistent.value === false ? null : undefined);
  const pluginPages: ComputedRef<PluginPageDescriptor[]> = computed(() =>
    mergePluginPages(behavior.value.pluginPages),
  );

  const pluginUis = computed(() => mergePluginUis(behavior.value.pluginUis));

  const applySnapshot = (snapshot: BehaviorSnapshot, versions: ReadonlyMap<string, string>, persistence: boolean | null, host: AutomationHost | null): void => {
    mergePluginMessages(snapshot.translations ?? {});
    setPluginEventTypes(snapshot.eventTypes ?? []);
    applySnapshotContributions(mergePluginAutocomplete(snapshot.autocompleteContributions));
    syncAutocompletePluginStates(
      snapshot.plugins.map((plugin) => ({
        id: plugin.descriptor.id,
        installed: plugin.installed,
        enabled: plugin.enabled,
        available: plugin.available,
      })),
    );
    projection.value = { snapshot, versions, persistent: persistence, host };
  };
  return { behavior, behaviorVersions, persistent, sourceHost, revisionFor, pluginPages, pluginUis, applySnapshot };
}
