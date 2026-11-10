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
  const boxRef = useRef<HTMLDivElement>(null);
  const readyRef = useRef(false);
  const sentRef = useRef(false);
  const htmlRef = useRef(html);
  const appearanceRef = useRef(appearance);
  htmlRef.current = html;
  appearanceRef.current = appearance;
  const [failed, setFailed] = useState(false);
  const [shown, setShown] = useState(false);
  const [size, setSize] = useState<{ width: number; height: number } | null>(null);
  const [fit, setFit] = useState(1);
  const canOpen = isBoardPage(pageUrl);

  useEffect(() => {
    sentRef.current = false;
    setFailed(false);
    setShown(false);
    setSize(null);
    const frame = frameRef.current;
    if (!frame) return undefined;
    const liveFrame: HTMLIFrameElement = frame;
    const painted = { current: false };
    let timer = 0;
    const arm = () => {
      if (timer || !htmlRef.current) return;
      timer = window.setTimeout(() => {
        if (!painted.current) setFailed(true);
      }, LOAD_TIMEOUT_MS);
    };
    const finish = () => {
      painted.current = true;
      window.clearTimeout(timer);
    };
    const send = () => {
      const fragment = htmlRef.current;
      const runner = liveFrame.contentWindow;
      if (!fragment || !readyRef.current || sentRef.current || !runner) return;
      sentRef.current = true;
      runner.postMessage(renderMessage(fragment, appearanceRef.current), '*');
      arm();
    };
    function onMessage(event: MessageEvent) {
      const runner = liveFrame.contentWindow;
      if (!runner || event.source !== runner) return;
      const message = readSandboxMessage(event.data);
      if (!message) return;
      if (message.kind === 'ready') {
        readyRef.current = true;
        send();
        return;
      }
      if (message.kind === 'size') {
        finish();
        const available = boxRef.current?.clientWidth ?? 0;
        setFailed(false);
        setFit(available > 0 ? Math.min(1, available / message.width) : 1);
        setShown(available > 0);
        setSize({ width: message.width, height: message.height });
        return;
      }
      if (message.kind === 'error') {
        finish();
        setFailed(true);
      }
    }
    window.addEventListener('message', onMessage);
    send();
    arm();
    return () => {
      window.clearTimeout(timer);
      window.removeEventListener('message', onMessage);
    };
  }, [html]);

  useEffect(() => {
    const runner = frameRef.current?.contentWindow;
    if (!sentRef.current || !runner) return;
    runner.postMessage(appearanceMessage(appearance), '*');
  }, [appearance]);

  useEffect(() => {
    const box = boxRef.current;
    if (!box || !size) return undefined;
    const measure = () => {
      const available = box.clientWidth;
      if (!available || !size.width) return;
      setFit(Math.min(1, available / size.width));
      setShown(true);
    };
    measure();
    if (typeof ResizeObserver !== 'function') return undefined;
    const observer = new ResizeObserver(measure);
    observer.observe(box);
    return () => observer.disconnect();
  }, [size]);

  const pending = !shown && !failed;
  const boxClass = failed
    ? 'board-sketch-frame-box is-failed'
    : shown
      ? 'board-sketch-frame-box is-ready'
      : 'board-sketch-frame-box';

  return (
    <div className="board-sketch-preview" aria-busy={pending ? true : undefined}>
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
        className={boxClass}
        style={shown && size ? { height: Math.ceil(size.height * fit) } : undefined}
      >
        <iframe
          ref={frameRef}
          className={shown ? 'board-sketch-frame is-ready' : 'board-sketch-frame'}
          title="Board sketch preview"
          src={SANDBOX_SRC}
          sandbox={SANDBOX_ALLOW}
          referrerPolicy="no-referrer"
          style={size
            ? { width: size.width, height: size.height, transform: `scale(${fit})`, colorScheme: 'light' }
            : { colorScheme: 'light' }}
        />
      </div>
    </div>
  );
}
