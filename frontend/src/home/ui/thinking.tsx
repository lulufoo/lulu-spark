import { thinkingTitle } from '../state/store.ts';

const stroke = {
  stroke: 'currentColor',
  strokeWidth: 2,
  strokeLinecap: 'round' as const,
  strokeLinejoin: 'round' as const,
};

export function ThinkingFold({
  text,
  ms,
  live = false,
}: {
  text: string;
  ms?: number;
  live?: boolean;
}) {
  return (
    <details className="home-chat-thinking" data-role="thinking">
      <summary className="home-chat-thinking-summary">
        {thinkingTitle(ms, live)}
        <svg
          className="home-chat-thinking-chevron home-chat-thinking-chevron--right"
          viewBox="0 0 24 24"
          aria-hidden="true"
          focusable="false"
          fill="none"
        >
          <path d="M9 18l6-6-6-6" {...stroke} />
        </svg>
        <svg
          className="home-chat-thinking-chevron home-chat-thinking-chevron--down"
          viewBox="0 0 24 24"
          aria-hidden="true"
          focusable="false"
          fill="none"
        >
          <path d="M6 9l6 6 6-6" {...stroke} />
        </svg>
      </summary>
      <div className="home-chat-thinking-body">{text}</div>
    </details>
  );
}
