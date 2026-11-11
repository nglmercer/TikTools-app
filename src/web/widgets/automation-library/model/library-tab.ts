export type LibraryTab = 'events' | 'actions';

export function isLibraryTab(value: unknown): value is LibraryTab {
  return value === 'events' || value === 'actions';
}

/** Areas that sit next to the records in the library's tab bar; the page decides which ones exist. */
export type LibraryExtraArea = 'variables' | 'cloud';
export type LibraryArea = LibraryTab | LibraryExtraArea;

export function isLibraryArea(value: unknown): value is LibraryArea {
  return isLibraryTab(value) || value === 'variables' || value === 'cloud';
}
