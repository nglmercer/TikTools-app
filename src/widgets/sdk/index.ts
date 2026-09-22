export { defineWidget, resolveWidgetDesign, styleVariables, widgetStyleFields } from './template.ts';
export type {
  WidgetTemplate,
  WidgetTemplateSchema,
  WidgetEditorSection,
  WidgetEditorGroup,
  WidgetEditorControl,
  WidgetTemplateToken,
  WidgetController,
  WidgetStyle,
} from './template.ts';
export { default as WidgetHost } from './WidgetHost.vue';
export { default as WidgetStage } from './WidgetStage.vue';
export { widgetTemplates } from './templates.ts';
export type { WidgetKind } from './templates.ts';
