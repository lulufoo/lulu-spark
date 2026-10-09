import { useEffect, useState, type ReactNode } from 'react';
import { ViewerHeaderIcon } from '../../shared/viewer-header-icons.tsx';
import { formatMessageWhen } from '../state/store.ts';

const COPY_FLASH_MS = 1200;

export type ChatTurnKind = 'user' | 'assistant' | 'error';

export function ChatTurn({
  kind,
  createdAt,
  onCopy,
  fold,
  children,
}: {
  kind: ChatTurnKind;
  createdAt?: number;
  onCopy: () => Promise<boolean>;
  /** Collapsed thinking block shown above the bubble. */
  fold?: ReactNode;
  children: ReactNode;
}) {
  const [copied, setCopied] = useState(false);
  const when = formatMessageWhen(createdAt);
  const copyLabel = copied ? 'Copied' : 'Copy';

  useEffect(() => {
    if (!copied) return undefined;
    const timer = window.setTimeout(() => setCopied(false), COPY_FLASH_MS);
    return () => window.clearTimeout(timer);
  }, [copied]);

  return (
    <div className={`home-chat-turn home-chat-turn--${kind}`}>
      {fold}
      <div className={`home-chat-bubble home-chat-bubble--${kind}`}>{children}</div>
      <div className="home-chat-turn-meta">
        {kind === 'user' && when ? <time className="home-chat-turn-time">{when}</time> : null}
        <button
          type="button"
          className="home-chat-turn-copy"
          data-role="copy-message"
          aria-label={copyLabel}
          title={copyLabel}
          onClick={() => {
            void onCopy().then((ok) => {
              if (ok) setCopied(true);
            });
          }}
        >
          <ViewerHeaderIcon name="copy" />
        </button>
        {kind !== 'user' && when ? <time className="home-chat-turn-time">{when}</time> : null}
      </div>
    </div>
  );
}
