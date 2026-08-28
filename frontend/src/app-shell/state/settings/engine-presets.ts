/**
 * Built-in Assistant/Engine category + preset catalog (read-only).
 * The sole category id aligns with L3 `assistant_engine`: `host`.
 * Model is the only editable preset-carried field; no MCP/SDK/cwd keys.
 */

export type EngineCategory = { id: 'host'; label: string };

export type EnginePresetMeta = {
  categoryId: 'host';
  displayName: string;
  editableFields: string[];
  readonlyFields: string[];
  fields: Record<string, string>;
};

const EDITABLE_FIELDS = Object.freeze(['model']);
const READONLY_FIELDS = Object.freeze(['platform', 'base_url']);

export const ENGINE_CATEGORIES: readonly EngineCategory[] = Object.freeze([
  Object.freeze({ id: 'host' as const, label: 'Agent Loop / GLM' }),
]);

const PRESETS: Readonly<Record<string, EnginePresetMeta>> = Object.freeze({
  host: Object.freeze({
    categoryId: 'host' as const,
    displayName: 'Agent Loop / GLM',
    editableFields: EDITABLE_FIELDS as unknown as string[],
    readonlyFields: READONLY_FIELDS as unknown as string[],
    fields: Object.freeze({
      platform: 'glm',
      base_url: 'https://open.bigmodel.cn/api/paas/v4',
      model: '',
    }),
  }),
});

export function listEngineCategories(): EngineCategory[] {
  return ENGINE_CATEGORIES.map((c) => ({ ...c }));
}

export function getEnginePreset(categoryId: string): EnginePresetMeta | null {
  const preset = PRESETS[categoryId];
  if (!preset) return null;
  return {
    categoryId: preset.categoryId,
    displayName: preset.displayName,
    editableFields: [...preset.editableFields],
    readonlyFields: [...preset.readonlyFields],
    fields: { ...preset.fields },
  };
}
