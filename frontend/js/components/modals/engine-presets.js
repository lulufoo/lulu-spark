/**
 * Built-in Assistant/Engine category + preset catalog (read-only).
 * Category ids align with L3 `assistant_engine`: `host` | `cursor`.
 * Model is the only editable preset-carried field; no MCP/SDK/cwd keys.
 */

/** @typedef {{ id: 'cursor' | 'host', label: string }} EngineCategory */

/**
 * @typedef {object} EnginePresetMeta
 * @property {'cursor' | 'host'} categoryId
 * @property {string} displayName
 * @property {string[]} editableFields
 * @property {string[]} readonlyFields
 * @property {Record<string, string>} fields
 */

const EDITABLE_FIELDS = Object.freeze(['model']);
const READONLY_FIELDS = Object.freeze(['platform', 'base_url']);

/** @type {EngineCategory[]} */
export const ENGINE_CATEGORIES = Object.freeze([
  Object.freeze({ id: 'cursor', label: 'Cursor Agent' }),
  Object.freeze({ id: 'host', label: 'Host / GLM' }),
]);

/** @type {Readonly<Record<string, EnginePresetMeta>>} */
const PRESETS = Object.freeze({
  cursor: Object.freeze({
    categoryId: 'cursor',
    displayName: 'Cursor Agent',
    editableFields: EDITABLE_FIELDS,
    readonlyFields: READONLY_FIELDS,
    fields: Object.freeze({
      platform: 'cursor_agent',
      base_url: '(managed by Cursor Agent)',
      model: '',
    }),
  }),
  host: Object.freeze({
    categoryId: 'host',
    displayName: 'Host / GLM',
    editableFields: EDITABLE_FIELDS,
    readonlyFields: READONLY_FIELDS,
    fields: Object.freeze({
      platform: 'glm',
      base_url: 'https://open.bigmodel.cn/api/paas/v4',
      model: '',
    }),
  }),
});

/** @returns {EngineCategory[]} */
export function listEngineCategories() {
  return ENGINE_CATEGORIES.map((c) => ({ ...c }));
}

/**
 * @param {string} categoryId
 * @returns {EnginePresetMeta | null}
 */
export function getEnginePreset(categoryId) {
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
