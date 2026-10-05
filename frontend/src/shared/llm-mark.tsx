import {
  getEnginePreset,
  type EngineCategoryId,
} from '../app-shell/state/settings/engine-presets.ts';

function OpenAiGlyph() {
  return (
    <path
      fill="#10a37f"
      d="M12 2.6 15.7 5l1.4 4.2-2.8 3.5H9.7L6.9 9.2 8.3 5 12 2.6Zm0 18.8L8.3 19l-1.4-4.2 2.8-3.5h4.6l2.8 3.5L15.7 19 12 21.4Z"
    />
  );
}

function ClaudeGlyph() {
  return (
    <path
      fill="#d97757"
      d="M12 2.2 13.4 9.6 21 12l-7.6 2.4L12 21.8l-1.4-7.4L3 12l7.6-2.4L12 2.2Z"
    />
  );
}

function GrokGlyph() {
  return (
    <path
      fill="#1a1a1a"
      d="M12 3 14 10h7l-5.6 4.2 2.1 6.8L12 17.2 6.5 21l2.1-6.8L3 10h7L12 3Z"
    />
  );
}

function GlmGlyph() {
  return (
    <g transform="translate(2 2) scale(1.17647) translate(-3.5 -3.5)">
      <rect x="3.5" y="3.5" width="17" height="17" rx="4" fill="#0b5fff" />
      <path fill="#fff" d="M8 8h8v2.1H10.7L16 16.9H8v-2.1h5.2L8 8Z" />
    </g>
  );
}

function KimiGlyph() {
  return (
    <path
      fill="#1a1a1a"
      d="M13.2 3.4A9 9 0 1 0 20.6 14 7.4 7.4 0 1 1 13.2 3.4Z"
    />
  );
}

function QwenGlyph() {
  return (
    <>
      <circle cx="12" cy="12" r="8" fill="#615ced" />
      <circle cx="12" cy="12" r="3.1" fill="#fff" />
    </>
  );
}

const GLYPHS: Record<EngineCategoryId, () => ReturnType<typeof OpenAiGlyph>> = {
  openai: OpenAiGlyph,
  claude: ClaudeGlyph,
  grok: GrokGlyph,
  host: GlmGlyph,
  kimi: KimiGlyph,
  qwen: QwenGlyph,
};

export function resolveLlmCategory(category: string): EngineCategoryId | null {
  return getEnginePreset(category)?.categoryId ?? null;
}

export function LlmMark({
  category,
  className = 'home-chat-session-icon',
}: {
  category: string;
  className?: string;
}) {
  const id = resolveLlmCategory(category);
  if (!id) return null;
  const label = getEnginePreset(id)?.displayName ?? 'LLM';
  const Glyph = GLYPHS[id];
  return (
    <span className={className} data-llm={id} title={label} aria-hidden="true">
      <svg viewBox="0 0 24 24" focusable="false">
        <Glyph />
      </svg>
    </span>
  );
}
