import { useEffect, useRef, useState } from 'react';
import {
  SANDBOX_ALLOW,
  SANDBOX_SRC,
  appearanceMessage,
  isBoardPage,
  openBoardPage,
  readSandboxMessage,
  renderMessage,
  type SketchAppearance,
} from '../commands/sketch-preview.ts';

const LOAD_TIMEOUT_MS = 8000;

type Props = {
  html: string;
  pageUrl: string;
  appearance: SketchAppearance;
};

export function SketchPreview({ html, pageUrl, appearance }: Props) {
  const frameRef = useRef<HTMLIFrameElement>(null);
  const sentRender = useRef(false);
  const appearanceRef = useRef(appearance);
  appearanceRef.current = appearance;
  const [failed, setFailed] = useState(false);
  const [size, setSize] = useState<{ width: number; height: number } | null>(null);
  const canOpen = isBoardPage(pageUrl);

  useEffect(() => {
    sentRender.current = false;
    setFailed(false);
    setSize(null);
    const frame = frameRef.current;
    if (!frame) return undefined;
    const timer = window.setTimeout(() => {
      if (!sentRender.current) setFailed(true);
    }, LOAD_TIMEOUT_MS);

    function onMessage(event: MessageEvent) {
      if (!frame.contentWindow || event.source !== frame.contentWindow) return;
      const message = readSandboxMessage(event.data);
      if (!message) return;
      if (message.kind === 'ready' && !sentRender.current) {
        sentRender.current = true;
        window.clearTimeout(timer);
        frame.contentWindow.postMessage(renderMessage(html, appearanceRef.current), '*');
        return;
      }
      if (message.kind === 'size') setSize({ width: message.width, height: message.height });
      if (message.kind === 'error') setFailed(true);
    }

    window.addEventListener('message', onMessage);
    return () => {
      window.clearTimeout(timer);
      window.removeEventListener('message', onMessage);
    };
  }, [html]);

  useEffect(() => {
    const frame = frameRef.current;
    if (!sentRender.current || !frame?.contentWindow) return;
    frame.contentWindow.postMessage(appearanceMessage(appearance), '*');
  }, [appearance]);

  if (!html) return null;

  return (
    <div className="board-sketch-preview">
      {failed ? (
        <p className="board-sketch-error" role="status">The sketch preview failed to load.</p>
      ) : null}
      <iframe
        key={html}
        ref={frameRef}
        className="board-sketch-frame"
        title="Board sketch preview"
        src={SANDBOX_SRC}
        sandbox={SANDBOX_ALLOW}
        referrerPolicy="no-referrer"
        style={size ? { width: size.width, height: size.height } : undefined}
      />
      {canOpen ? (
        <button type="button" className="btn-settings-ghost" onClick={() => void openBoardPage(pageUrl)}>
          Open in Board
        </button>
      ) : null}
    </div>
  );
}
