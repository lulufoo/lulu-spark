import { useEffect, useId, useRef, useState } from 'react';
import { elapsedMs, logSketch } from '../commands/sketch-log.ts';
import {
  SANDBOX_ALLOW,
  SANDBOX_SRC,
  appearanceMessage,
  boardDiagramId,
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
  const diagramId = boardDiagramId(pageUrl);
  const startedAtRef = useRef(performance.now());
  const shineId = `sketch-shine-${useId().replace(/[^a-zA-Z0-9]/g, '')}`;
  const shineMid = appearance === 'dark' ? 0.78 : 0.34;

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
        if (!painted.current) {
          setFailed(true);
          logSketch('sketch.failed', {
            diagram_id: diagramId,
            ms: elapsedMs(startedAtRef.current),
            stage: 'timeout',
          }, 'warn');
        }
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
      logSketch('sandbox.render', {
        diagram_id: diagramId,
        ms: elapsedMs(startedAtRef.current),
        html_bytes: fragment.length,
      });
      arm();
    };
    function onMessage(event: MessageEvent) {
      const runner = liveFrame.contentWindow;
      if (!runner || event.source !== runner) return;
      const message = readSandboxMessage(event.data);
      if (!message) return;
      if (message.kind === 'ready') {
        const first = !readyRef.current;
        readyRef.current = true;
        if (first) {
          logSketch('sandbox.ready', { diagram_id: diagramId, ms: elapsedMs(startedAtRef.current) });
        }
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
        logSketch('sandbox.size', {
          diagram_id: diagramId,
          ms: elapsedMs(startedAtRef.current),
          width: message.width,
          height: message.height,
        });
        return;
      }
      if (message.kind === 'error') {
        finish();
        setFailed(true);
        logSketch('sketch.failed', {
          diagram_id: diagramId,
          ms: elapsedMs(startedAtRef.current),
          stage: 'sandbox',
        }, 'warn');
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
      {failed ? (
        <p className="board-sketch-error" role="status">The sketch preview failed to load.</p>
      ) : null}
      <div
        ref={boxRef}
        className={boxClass}
        style={shown && size ? { height: Math.ceil(size.height * fit) } : undefined}
      >
        {pending ? (
          <div className="board-sketch-loading" role="status" aria-label="Loading preview">
            <svg viewBox="0 0 32 32" aria-hidden="true">
              <defs>
                <linearGradient id={shineId} gradientUnits="userSpaceOnUse" x1="-20" y1="0" x2="-8" y2="0">
                  <stop offset="0" stopColor="currentColor" stopOpacity="0.55" />
                  <stop offset="0.5" stopColor="currentColor" stopOpacity={shineMid} />
                  <stop offset="1" stopColor="currentColor" stopOpacity="0.55" />
                  <animate attributeName="x1" from="-20" to="40" dur="1.9s" repeatCount="indefinite" />
                  <animate attributeName="x2" from="-8" to="52" dur="1.9s" repeatCount="indefinite" />
                </linearGradient>
              </defs>
              <path
                d="M9.5 8h4A1.5 1.5 0 0 1 15 9.5v6A1.5 1.5 0 0 0 16.5 17h6a1.5 1.5 0 0 1 1.5 1.5v4a1.5 1.5 0 0 1-1.5 1.5h-13A1.5 1.5 0 0 1 8 22.5v-13A1.5 1.5 0 0 1 9.5 8z"
                fill={`url(#${shineId})`}
              />
              <rect x="18" y="8" width="6" height="6" rx="1.5" fill={`url(#${shineId})`} />
            </svg>
          </div>
        ) : null}
        <iframe
          ref={frameRef}
          className={shown ? 'board-sketch-frame is-ready' : 'board-sketch-frame'}
          title={title || 'Board sketch preview'}
          src={SANDBOX_SRC}
          sandbox={SANDBOX_ALLOW}
          referrerPolicy="no-referrer"
          style={size
            ? { width: size.width, height: size.height, transform: `scale(${fit})`, colorScheme: 'light' }
            : { colorScheme: 'light' }}
        />
      </div>
      {canOpen ? (
        <button type="button" className="board-sketch-open" onClick={() => void openBoardPage(pageUrl)}>
          Open in Board ↗
        </button>
      ) : null}
    </div>
  );
}
