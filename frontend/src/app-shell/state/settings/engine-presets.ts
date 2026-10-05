/**
 * Built-in LLM category + preset catalog.
 * Category ids align with `assistant_engine`. GLM keeps the legacy id `host`.
 * Model and base_url are editable; platform stays read-only.
 */

export type EngineCategoryId =
  | 'openai'
  | 'claude'
  | 'grok'
  | 'host'
  | 'kimi'
  | 'qwen';

export type EngineCategory = { id: EngineCategoryId; label: string };

export type EnginePresetMeta = {
  categoryId: EngineCategoryId;
  displayName: string;
  editableFields: string[];
  readonlyFields: string[];
  fields: Record<string, string>;
};

const EDITABLE_FIELDS = Object.freeze(['model', 'base_url']);
const READONLY_FIELDS = Object.freeze(['platform']);

export const ENGINE_CATEGORIES: readonly EngineCategory[] = Object.freeze([
  Object.freeze({ id: 'openai' as const, label: 'OpenAI' }),
  Object.freeze({ id: 'claude' as const, label: 'Claude' }),
  Object.freeze({ id: 'grok' as const, label: 'Grok' }),
  Object.freeze({ id: 'host' as const, label: 'GLM' }),
  Object.freeze({ id: 'kimi' as const, label: 'Kimi' }),
  Object.freeze({ id: 'qwen' as const, label: 'Qwen' }),
]);

function preset(
  categoryId: EngineCategoryId,
  displayName: string,
  platform: string,
  baseUrl: string,
  model: string,
): EnginePresetMeta {
  return Object.freeze({
    categoryId,
    displayName,
    editableFields: EDITABLE_FIELDS as unknown as string[],
    readonlyFields: READONLY_FIELDS as unknown as string[],
    fields: Object.freeze({ platform, base_url: baseUrl, model }),
  });
}

const PRESETS: Readonly<Record<EngineCategoryId, EnginePresetMeta>> = Object.freeze({
  openai: preset('openai', 'OpenAI', 'openai', 'https://api.openai.com/v1', 'gpt-4.1'),
  claude: preset('claude', 'Claude', 'claude', 'https://api.anthropic.com/v1', 'claude-sonnet-4-5'),
  grok: preset('grok', 'Grok', 'grok', 'https://api.x.ai/v1', 'grok-4'),
  host: preset('host', 'GLM', 'glm', 'https://open.bigmodel.cn/api/paas/v4', ''),
  kimi: preset('kimi', 'Kimi', 'kimi', 'https://api.moonshot.cn/v1', 'kimi-k2'),
  qwen: preset(
    'qwen',
    'Qwen',
    'qwen',
    'https://dashscope.aliyuncs.com/compatible-mode/v1',
    'qwen-plus',
  ),
});

export function listEngineCategories(): EngineCategory[] {
  return ENGINE_CATEGORIES.map((c) => ({ ...c }));
}

export function getEnginePreset(categoryId: string): EnginePresetMeta | null {
  const presetMeta = PRESETS[categoryId as EngineCategoryId];
  if (!presetMeta) return null;
  return {
    categoryId: presetMeta.categoryId,
    displayName: presetMeta.displayName,
    editableFields: [...presetMeta.editableFields],
    readonlyFields: [...presetMeta.readonlyFields],
    fields: { ...presetMeta.fields },
  };
}
