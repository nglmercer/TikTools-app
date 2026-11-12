import type { Ref } from 'vue';
import type { Capability, Grant } from '@/entities/collaboration/api/types.ts';
import { cloneGrant } from './clone-grant.ts';
import { effectCapabilities, requiredAutomation, writeDependencies, type DelegatedEffect } from './grant-options.ts';

/** Capability and delegation edits share dependency rules across the grant panels. */
export function useGrantCapabilities(model: Ref<Grant>) {
  const copy = () => cloneGrant(model.value);
  function requiredByDelegation(capability: Capability): boolean {
    const grant = model.value;
    if (!grant.capabilities.includes(capability)) return false;
    const neededForWrites = grant.capabilities.some(write => writeDependencies[write]?.includes(capability));
    if (neededForWrites) return true;
    const delegation = grant.automationDelegation;
    if (!delegation) return false;
    return requiredAutomation.includes(capability)
      || delegation.effects.some(effect => effectCapabilities[effect]?.includes(capability));
  }
  function setDelegatedEffect(effect: DelegatedEffect, enabled: boolean): void {
    const value = copy();
    const effects = new Set(value.automationDelegation?.effects ?? []);
    if (enabled) effects.add(effect); else effects.delete(effect);
    if (!effects.size) delete value.automationDelegation;
    else {
      value.automationDelegation = { ...value.automationDelegation, effects: [...effects], allowDynamicTargets: value.automationDelegation?.allowDynamicTargets ?? false };
      const caps = new Set(value.capabilities);
      requiredAutomation.forEach((capability) => caps.add(capability));
      if (enabled) effectCapabilities[effect]?.forEach((capability) => caps.add(capability));
      for (const capability of [...caps]) writeDependencies[capability]?.forEach((dependency) => caps.add(dependency));
      value.capabilities = [...caps];
    }
    model.value = value;
  }
  function setOwnerEdits(enabled: boolean): void {
    const value = copy();
    if (value.automationDelegation) {
      if (enabled) value.automationDelegation.allowOwnerEdits = true;
      else { delete value.automationDelegation.allowOwnerEdits; delete value.automationDelegation.allowOwnerSource; }
    }
    model.value = value;
  }
  function setOwnerSource(enabled: boolean): void {
    const value = copy();
    if (value.automationDelegation && value.automationDelegation.allowOwnerEdits) {
      if (enabled) value.automationDelegation.allowOwnerSource = true;
      else delete value.automationDelegation.allowOwnerSource;
    }
    model.value = value;
  }
  function setOtherAuthorEdits(enabled: boolean): void {
    const value = copy();
    if (value.automationDelegation) {
      if (enabled) value.automationDelegation.allowOtherAuthorEdits = true;
      else { delete value.automationDelegation.allowOtherAuthorEdits; delete value.automationDelegation.allowOtherAuthorSource; }
    }
    model.value = value;
  }
  function setOtherAuthorSource(enabled: boolean): void {
    const value = copy();
    if (value.automationDelegation?.allowOtherAuthorEdits) {
      if (enabled) value.automationDelegation.allowOtherAuthorSource = true;
      else delete value.automationDelegation.allowOtherAuthorSource;
    }
    model.value = value;
  }
  function setDynamicTargets(enabled: boolean): void {
    const value = copy();
    if (value.automationDelegation) value.automationDelegation.allowDynamicTargets = enabled;
    model.value = value;
  }
  function setCapability(capability: Capability, allowed: boolean): void {
    const value = copy(); const caps = new Set(value.capabilities);
    if (allowed) caps.add(capability); else caps.delete(capability);
    if (allowed) writeDependencies[capability]?.forEach((dependency) => caps.add(dependency));
    value.capabilities = [...caps]; model.value = value;
  }

  return { requiredByDelegation, setCapability, setDelegatedEffect, setOwnerEdits, setOwnerSource, setOtherAuthorEdits, setOtherAuthorSource, setDynamicTargets };
}
