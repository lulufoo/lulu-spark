import { useEffect, useRef, useState, useSyncExternalStore } from 'react';
import { convertStore } from '../state/convert.ts';
import { renderQr } from './qr-dialog.tsx';

export function ConvertDialog() {
  const { open, tab } = useSyncExternalStore(convertStore.subscribe, convertStore.getSnapshot);
  const [input, setInput] = useState('');
  const [output, setOutput] = useState('');
  const [outputError, setOutputError] = useState(false);
  const [copyLabel, setCopyLabel] = useState('Copy result');
  const [qrText, setQrText] = useState('');
  const qrPreviewRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!open) return;
    setInput('');
    setOutput('');
    setOutputError(false);
    setCopyLabel('Copy result');
    setQrText('');
    if (qrPreviewRef.current) qrPreviewRef.current.innerHTML = '';
  }, [open]);

  function paintQr(text: string) {
    setQrText(text);
    const preview = qrPreviewRef.current;
    if (preview) renderQr(text, preview);
  }

  function encode() {
    const bytes = new TextEncoder().encode(input);
    setOutput(btoa(String.fromCharCode(...bytes)));
    setOutputError(false);
  }

  function decode() {
    try {
      const result = new TextDecoder().decode(Uint8Array.from(atob(input), (c) => c.charCodeAt(0)));
      setOutput(result);
      setOutputError(false);
    } catch {
      setOutput('⚠️ Decode failed: input is not valid Base64');
      setOutputError(true);
    }
  }

  return (
    <div
      id="convert-dialog"
      className={open ? 'open' : undefined}
      onClick={(e) => {
        if (e.target === e.currentTarget) convertStore.set((s) => ({ ...s, open: false }));
      }}
    >
      <div id="convert-dialog-box">
        <div id="convert-dialog-header">
          <span>🔀 Convert</span>
          <button id="btn-convert-close" type="button" onClick={() => convertStore.set((s) => ({ ...s, open: false }))}>
            ✕ Close
          </button>
        </div>
        <div id="convert-dialog-tabs">
          <button
            type="button"
            id="convert-tab-base64"
            className={`convert-tab-btn${tab === 'base64' ? ' active' : ''}`}
            onClick={() => {
              convertStore.set((s) => ({ ...s, tab: 'base64' }));
              document.getElementById('base64-input')?.focus();
            }}
          >
            🔤 Base64
          </button>
          <button
            type="button"
            id="convert-tab-qr"
            className={`convert-tab-btn${tab === 'qr' ? ' active' : ''}`}
            onClick={() => {
              convertStore.set((s) => ({ ...s, tab: 'qr' }));
              document.getElementById('qr-input')?.focus();
            }}
          >
            📱 QR code
          </button>
        </div>
        <div id="convert-panel-base64" className={`convert-panel${tab === 'base64' ? ' active' : ''}`}>
          <textarea
            id="base64-input"
            placeholder="Enter text or Base64…"
            spellCheck={false}
            value={input}
            onChange={(e) => setInput(e.target.value)}
          />
          <div id="base64-dialog-actions">
            <button id="btn-base64-encode" type="button" onClick={encode}>
              Encode →
            </button>
            <button id="btn-base64-decode" type="button" onClick={decode}>
              ← Decode
            </button>
          </div>
          <textarea
            id="base64-output"
            readOnly
            placeholder="Result…"
            spellCheck={false}
            value={output}
            style={outputError ? { color: '#e5534b' } : undefined}
          />
          <div id="base64-dialog-footer">
            <button
              id="btn-base64-copy"
              type="button"
              onClick={() => {
                if (!output) return;
                void navigator.clipboard.writeText(output);
                setCopyLabel('Copied');
                setTimeout(() => setCopyLabel('Copy result'), 1500);
              }}
            >
              {copyLabel}
            </button>
          </div>
        </div>
        <div id="convert-panel-qr" className={`convert-panel${tab === 'qr' ? ' active' : ''}`}>
          <textarea
            id="qr-input"
            placeholder="Enter URL or text…"
            spellCheck={false}
            value={qrText}
            onChange={(e) => paintQr(e.target.value)}
            onInput={(e) => paintQr(e.currentTarget.value)}
          />
          <div id="qr-preview" ref={qrPreviewRef}></div>
        </div>
      </div>
    </div>
  );
}
