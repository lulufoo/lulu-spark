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
  title?: string;
};

export function SketchPreview({ html, pageUrl, appearance, title = '' }: Props) {
  const frameRef = useRef<HTMLIFrameElement>(null);
  const sentRender = useRef(false);
  const appearanceRef = useRef(appearance);
  appearanceRef.current = appearance;
  const boxRef = useRef<HTMLDivElement>(null);
  const [failed, setFailed] = useState(false);
  const [size, setSize] = useState<{ width: number; height: number } | null>(null);
  const [fit, setFit] = useState(1);
  const canOpen = isBoardPage(pageUrl);

  useEffect(() => {
    sentRender.current = false;
    setFailed(false);
    setSize(null);
    const frame = frameRef.current;
    if (!frame) return undefined;
    const liveFrame: HTMLIFrameElement = frame;
    const painted = { current: false };
    const timer = window.setTimeout(() => {
      if (!painted.current) setFailed(true);
    }, LOAD_TIMEOUT_MS);
    const finish = () => {
      painted.current = true;
      window.clearTimeout(timer);
    };

    function onMessage(event: MessageEvent) {
      const runner = liveFrame.contentWindow;
      if (!runner || event.source !== runner) return;
      const message = readSandboxMessage(event.data);
      if (!message) return;
      if (message.kind === 'ready' && !sentRender.current) {
        sentRender.current = true;
        runner.postMessage(renderMessage(html, appearanceRef.current), '*');
        return;
      }
      if (message.kind === 'size') {
        finish();
        setFailed(false);
        setSize({ width: message.width, height: message.height });
      }
      if (message.kind === 'error') {
        finish();
        setFailed(true);
      }
    }

    window.addEventListener('message', onMessage);
    return () => {
      window.clearTimeout(timer);
      window.removeEventListener('message', onMessage);
    };
  }, [html]);

  useEffect(() => {
    const frame = frameRef.current;
    const runner = frame?.contentWindow;
    if (!sentRender.current || !runner) return;
    runner.postMessage(appearanceMessage(appearance), '*');
  }, [appearance]);

  useEffect(() => {
    const box = boxRef.current;
    if (!box || !size) return undefined;
    const measure = () => {
      const available = box.clientWidth;
      if (!available || !size.width) return;
      setFit(Math.min(1, available / size.width));
    };
    measure();
    if (typeof ResizeObserver !== 'function') return undefined;
    const observer = new ResizeObserver(measure);
    observer.observe(box);
    return () => observer.disconnect();
  }, [size]);

  if (!html) return null;

  return (
    <div className="board-sketch-preview">
      <header className="board-sketch-title">
        <div className="board-sketch-name">{title}</div>
        {canOpen ? (
          <button type="button" className="board-sketch-open" onClick={() => void openBoardPage(pageUrl)}>
            Open in Board ↗
          </button>
        ) : null}
      </header>
      {failed ? (
        <p className="board-sketch-error" role="status">The sketch preview failed to load.</p>
      ) : null}
      <div
        ref={boxRef}
        className="board-sketch-frame-box"
        style={size ? { height: Math.ceil(size.height * fit) } : undefined}
      >
        <iframe
          key={html}
          ref={frameRef}
          className="board-sketch-frame"
          title="Board sketch preview"
          src={SANDBOX_SRC}
          sandbox={SANDBOX_ALLOW}
          referrerPolicy="no-referrer"
          style={size
            ? { width: size.width, height: size.height, transform: `scale(${fit})`, colorScheme: 'light' }
            : { colorScheme: 'light' }}
        />
        {canOpen ? (
          <button
            type="button"
            className="board-sketch-hit"
            aria-label="Open in Board"
            onClick={() => void openBoardPage(pageUrl)}
          />
        ) : null}
      </div>
    </div>
  );
}
