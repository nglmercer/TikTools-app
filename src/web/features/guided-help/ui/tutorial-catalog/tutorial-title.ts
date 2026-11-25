/** Removes the already-visible category prefix when a guide is rendered inside that category. */
export function contextualTutorialTitle(title: string, categoryLabel?: string): string {
  if (!categoryLabel) return title;
  if (!title.toLocaleLowerCase().startsWith(categoryLabel.toLocaleLowerCase())) return title;
  let rest = title.slice(categoryLabel.length).trimStart();
  if (!rest.startsWith(':')) return title;
  rest = rest.slice(1).trim();
  return rest ? rest.charAt(0).toLocaleUpperCase() + rest.slice(1) : title;
}
