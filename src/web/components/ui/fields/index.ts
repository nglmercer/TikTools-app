/**
 * Shared field system. Phase 3 consumes these primitives for autocomplete/
 * template wiring; screens keep importing the legacy `ui/*Input` shims or
 * migrate to these directly.
 */
export * from './field-logic.ts';
export * from './FieldShell.vue';
export * from './FieldLabel.vue';
export * from './FieldMessage.vue';
export * from './InputGroup.vue';
export * from './TextField.vue';
export * from './PasswordField.vue';
export * from './NumberField.vue';
export * from './SearchField.vue';
export * from './SelectField.vue';
export * from './TemplateField.vue';
