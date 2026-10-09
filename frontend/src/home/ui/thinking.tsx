import { thinkingTitle } from '../state/store.ts';

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
      <summary className="home-chat-thinking-summary">{thinkingTitle(ms, live)}</summary>
      <div className="home-chat-thinking-body">{text}</div>
    </details>
  );
}
