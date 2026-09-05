import { useEffect, useState } from 'react';

const COPY_LABEL = 'Copy Session ID';
const COPIED_LABEL = 'Copied';
const COPY_FLASH_MS = 1200;

export function SessionMenu({
  sessionId,
  onCopy,
}: {
  sessionId: string;
  onCopy: () => Promise<boolean>;
}) {
  const [open, setOpen] = useState(false);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    setOpen(false);
    setCopied(false);
  }, [sessionId]);

  useEffect(() => {
    if (!copied) return undefined;
    const timer = window.setTimeout(() => {
      setCopied(false);
      setOpen(false);
    }, COPY_FLASH_MS);
    return () => window.clearTimeout(timer);
  }, [copied]);

  if (!sessionId) return null;

  function close() {
    setOpen(false);
    setCopied(false);
  }

  return (
    <div className="home-chat-session-menu">
      <button
        type="button"
        className="home-chat-session-menu-btn"
        data-role="chat-session-menu"
        aria-label="Chat actions"
        aria-haspopup="menu"
        aria-expanded={open ? 'true' : 'false'}
        title="Chat actions"
        onClick={() => {
          setOpen((prev) => !prev);
          setCopied(false);
        }}
      >
        ⋯
      </button>
      {open ? (
        <>
          <div
            className="home-chat-session-menu-backdrop"
            data-role="close-session-menu"
            onClick={close}
          />
          <div className="home-chat-session-menu-panel" role="menu">
            <button
              type="button"
              role="menuitem"
              data-role="copy-session-id"
              onClick={() => {
                void onCopy().then((ok) => {
                  if (ok) setCopied(true);
                });
              }}
            >
              {copied ? COPIED_LABEL : COPY_LABEL}
            </button>
          </div>
        </>
      ) : null}
    </div>
  );
}
