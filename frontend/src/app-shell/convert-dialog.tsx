import { useEffect, useState, useSyncExternalStore } from 'react';
import { createModuleStore } from '../shared/module-store.ts';

type ConvertTab = 'base64' | 'qr';
type ConvertState = { open: boolean; tab: ConvertTab };

const convertStore = createModuleStore<ConvertState>({ open: false, tab: 'base64' });

function setActive(el: HTMLElement | null, on: boolean) {
  if (!el) return;
  if (on) el.classList.add('active');
  else el.classList.remove('active');
}

function applyTabDom(name: ConvertTab) {
  setActive(document.getElementById('convert-tab-base64'), name === 'base64');
  setActive(document.getElementById('convert-tab-qr'), name === 'qr');
  setActive(document.getElementById('convert-panel-base64'), name === 'base64');
  setActive(document.getElementById('convert-panel-qr'), name === 'qr');
}

export function setConvertTab(tab: string) {
  const name: ConvertTab = tab === 'qr' ? 'qr' : 'base64';
  convertStore.set((s) => ({ ...s, tab: name }));
  applyTabDom(name);
}

export function openConvertDialog(tab = 'base64') {
  const name: ConvertTab = tab === 'qr' ? 'qr' : 'base64';
  convertStore.set({ open: true, tab: name });
  document.getElementById('convert-dialog')?.classList.add('open');
  applyTabDom(name);
  if (name === 'qr') document.getElementById('qr-input')?.focus();
  else document.getElementById('base64-input')?.focus();
}

export function closeConvertDialog() {
  convertStore.set((s) => ({ ...s, open: false }));
  document.getElementById('convert-dialog')?.classList.remove('open');
}

export function ConvertDialog() {
  const { open, tab } = useSyncExternalStore(convertStore.subscribe, convertStore.getSnapshot);
  const [input, setInput] = useState('');
  const [output, setOutput] = useState('');
  const [outputError, setOutputError] = useState(false);
  const [copyLabel, setCopyLabel] = useState('Copy result');

  useEffect(() => {
    if (!open) return;
    setInput('');
    setOutput('');
    setOutputError(false);
    setCopyLabel('Copy result');
  }, [open]);

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
        if (e.target === e.currentTarget) closeConvertDialog();
      }}
    >
      <div id="convert-dialog-box">
        <div id="convert-dialog-header">
          <span>🔀 Convert</span>
          <button id="btn-convert-close" type="button" onClick={() => closeConvertDialog()}>
            ✕ Close
          </button>
        </div>
        <div id="convert-dialog-tabs">
          <button
            type="button"
            id="convert-tab-base64"
            className={`convert-tab-btn${tab === 'base64' ? ' active' : ''}`}
            onClick={() => {
              setConvertTab('base64');
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
              setConvertTab('qr');
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
          <textarea id="qr-input" placeholder="Enter URL or text…" spellCheck={false} />
          <div id="qr-preview"></div>
        </div>
      </div>
    </div>
  );
}
