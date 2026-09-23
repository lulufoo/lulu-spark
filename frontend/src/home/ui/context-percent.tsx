import { useEffect, useId, useRef, useState } from 'react';

import type { ContextUsage } from '../state/store.ts';

const RING_SIZE = 15;
const RING_STROKE = 2;
const RING_R = 5.5;
const RING_C = 2 * Math.PI * RING_R;

const CATEGORY_LABEL: Record<string, string> = {
  system_prompt: 'System prompt',
  tools: 'Tools',
  mcp: 'MCP',
  conversation: 'Conversation',
  other: 'Other',
};

const CATEGORY_COLOR: Record<string, string> = {
  system_prompt: 'gray',
  tools: 'purple',
  mcp: 'pink',
  conversation: 'orange',
  other: 'gray',
};

export function ContextPercent({
  percent,
  usage,
}: {
  percent: number | null;
  usage: ContextUsage | null;
}) {
  const [open, setOpen] = useState(false);
  const [highlighted, setHighlighted] = useState<string | null>(null);
  const anchorRef = useRef<HTMLSpanElement>(null);
  const titleId = useId();
  const shown = Math.min(100, Math.max(percent ?? 1, 1));
  const offset = RING_C - (shown / 100) * RING_C;

  useEffect(() => {
    if (!open) return undefined;
    function onKey(event: KeyboardEvent) {
      if (event.key === 'Escape') setOpen(false);
    }
    function onPointer(event: MouseEvent) {
      const target = event.target;
      if (!(target instanceof Node)) return;
      const anchor = anchorRef.current;
      if (anchor && !anchor.contains(target)) setOpen(false);
    }
    document.addEventListener('keydown', onKey);
    document.addEventListener('mousedown', onPointer);
    return () => {
      document.removeEventListener('keydown', onKey);
      document.removeEventListener('mousedown', onPointer);
    };
  }, [open]);

  const fullLabel = usage ? `${percent ?? 0}% Full` : null;
  const tokenLabel = usage
    ? `~${formatCompact(usage.totalTokens)} / ${formatCompact(usage.windowTokens)} Tokens`
    : null;
  const remainder = usage ? Math.max(0, usage.windowTokens - usage.totalTokens) : 0;

  return (
    <span ref={anchorRef} className="home-chat-context-usage-anchor" data-role="context-percent-anchor">
      <button
        type="button"
        className="home-chat-context-percent"
        data-role="context-percent"
        title="Show context usage"
        aria-label="Show context usage"
        aria-expanded={open}
        aria-controls={open ? titleId : undefined}
        onClick={() => setOpen((value) => !value)}
      >
        <svg
          className="home-chat-context-ring"
          width={RING_SIZE}
          height={RING_SIZE}
          viewBox={`0 0 ${RING_SIZE} ${RING_SIZE}`}
          aria-hidden="true"
        >
          <circle
            className="home-chat-context-ring-track"
            cx={RING_SIZE / 2}
            cy={RING_SIZE / 2}
            r={RING_R}
            fill="none"
            strokeWidth={RING_STROKE}
          />
          <circle
            className="home-chat-context-ring-progress"
            cx={RING_SIZE / 2}
            cy={RING_SIZE / 2}
            r={RING_R}
            fill="none"
            strokeWidth={RING_STROKE}
            strokeLinecap="round"
            strokeDasharray={RING_C}
            strokeDashoffset={offset}
            transform={`rotate(-90 ${RING_SIZE / 2} ${RING_SIZE / 2})`}
          />
        </svg>
      </button>
      {open ? (
        <div
          className="home-chat-context-usage"
          data-role="context-usage"
          role="dialog"
          aria-label="Context usage preview"
        >
          <div className="home-chat-context-usage-header">
            <div className="home-chat-context-usage-title" id={titleId}>
              Context Usage
            </div>
            <button
              type="button"
              className="home-chat-context-usage-close"
              aria-label="Close"
              onClick={() => setOpen(false)}
            >
              ×
            </button>
          </div>
          {usage && fullLabel && tokenLabel ? (
            <div className="home-chat-context-usage-summary">
              <div className="home-chat-context-usage-meta">
                <span>{fullLabel}</span>
                <span className="home-chat-context-usage-tokens">{tokenLabel}</span>
              </div>
              <div
                className="home-chat-context-usage-bar"
                role="img"
                aria-label={`Context usage by category: ${tokenLabel}`}
              >
                <div className="home-chat-context-usage-segments">
                  {usage.categories.map((category) => (
                    <span
                      key={category.id}
                      className="home-chat-context-usage-segment"
                      data-color={CATEGORY_COLOR[category.id] || 'gray'}
                      data-highlighted={highlighted === category.id || undefined}
                      style={{ flexGrow: category.tokens }}
                      onMouseEnter={() => setHighlighted(category.id)}
                      onMouseLeave={() => setHighlighted(null)}
                    />
                  ))}
                  {remainder > 0 ? (
                    <span className="home-chat-context-usage-remainder" style={{ flexGrow: remainder }} />
                  ) : null}
                </div>
              </div>
            </div>
          ) : (
            <p className="home-chat-context-usage-empty">No context usage yet.</p>
          )}
          {usage && usage.categories.length > 0 ? (
            <ul className="home-chat-context-usage-categories">
              {usage.categories.map((category) => (
                <li
                  key={category.id}
                  className="home-chat-context-usage-category"
                  data-color={CATEGORY_COLOR[category.id] || 'gray'}
                  data-highlighted={highlighted === category.id || undefined}
                  onMouseEnter={() => setHighlighted(category.id)}
                  onMouseLeave={() => setHighlighted(null)}
                >
                  <span className="home-chat-context-usage-swatch" aria-hidden="true" />
                  <span className="home-chat-context-usage-label">
                    {CATEGORY_LABEL[category.id] || category.id}
                  </span>
                  <span className="home-chat-context-usage-value">{formatCategory(category.tokens)}</span>
                </li>
              ))}
            </ul>
          ) : null}
        </div>
      ) : null}
    </span>
  );
}

function formatCompact(tokens: number, precise = true) {
  if (tokens >= 1_000_000) {
    const value = tokens / 1_000_000;
    return `${precise ? value.toFixed(1) : value}M`;
  }
  if (tokens >= 1_000) {
    const value = tokens / 1_000;
    return `${precise ? value.toFixed(1) : value}K`;
  }
  return String(tokens);
}

function formatCategory(tokens: number) {
  if (Math.abs(tokens) >= 1000) return `${(tokens / 1000).toFixed(1).replace(/\.0$/, '')}K`;
  return String(tokens);
}
