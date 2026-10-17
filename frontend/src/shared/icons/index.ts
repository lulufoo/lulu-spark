import { ICON_CATEGORIES, ICONS, type IconCategory, type IconDef } from './catalog.ts';

export type { IconCategory, IconDef };
export { ICON_CATEGORIES, ICONS };

/** What the settings icon picker lists: one entry per category that is not excluded. */
export type PickerCategory = IconCategory & { icons: IconDef[] };

const ICON_ID_FORMAT = /^0[0-6][a-z0-9]{4}$/;
const BY_ID = new Map(ICONS.map((icon) => [icon.id, icon]));

/** A string shaped like an icon id; it does not say the icon exists (see `getIcon`). */
export function isIconId(value: unknown): value is string {
  return typeof value === 'string' && ICON_ID_FORMAT.test(value);
}

/** The icon with this id, or `undefined` for an unknown one. */
export function getIcon(id: string): IconDef | undefined {
  return BY_ID.get(id);
}

/** Categories the picker shows, in catalog order, each with its icons. */
export function pickerCategories(): PickerCategory[] {
  return ICON_CATEGORIES.filter((category) => category.picker).map((category) => ({
    ...category,
    icons: ICONS.filter((icon) => icon.category === category.code),
  }));
}

function findDefaultProjectIcon(): string {
  const folder = ICONS.find((icon) => icon.name === 'Folder' && icon.category === '03');
  if (!folder) throw new Error('Icon catalog has no Folder icon in Content');
  return folder.id;
}

/** Icon of a project that has none configured: Folder, from the Content category. */
export const DEFAULT_PROJECT_ICON_ID = findDefaultProjectIcon();
