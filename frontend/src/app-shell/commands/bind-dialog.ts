import * as api from '../../host/api.ts';
import { bindOpenStore } from '../state/dialog-open.ts';

export { bindOpenStore };

declare const QRCode: {
  toCanvas: (
    canvas: HTMLCanvasElement,
    text: string,
    opts: { width: number; margin: number },
    cb: (err?: Error | null) => void,
  ) => void;
};

const QR_OPTS = { width: 256, margin: 2 };
const POLL_MS = 1000;
const BIND_MOBILE_BUSINESS_ID = 'Bind_Mobile';
const BIND_STATES = ['loading', 'waiting', 'success', 'expired', 'error'];
const ISSUE_ERROR_STATUS: Record<string, string> = {
  no_lan: 'No local network is available.',
  no_gateway: 'The local gateway is unavailable.',
};

type BindPayload = {
  ip?: string;
  port?: number;
  temp_pub?: string;
  tls_fingerprint?: string;
  exp?: number;
  sig?: string;
};

let pollTimer: ReturnType<typeof setInterval> | null = null;
let countdownTimer: ReturnType<typeof setInterval> | null = null;

function el(id: string) {
  return document.getElementById(id);
}

function logBindEvent(event: string, outcome: string) {
  console.info(`[bind] business_id=${BIND_MOBILE_BUSINESS_ID} event=${event} outcome=${outcome}`);
}

function bindErrorCode(error: unknown) {
  const message = error instanceof Error ? error.message : String(error);
  return Object.hasOwn(ISSUE_ERROR_STATUS, message) ? message : 'unknown';
}

function setBindState(state: string) {
  const box = el('bind-dialog-box');
  if (!box) return;
  BIND_STATES.forEach((name) => box.classList.remove(`bind-state-${name}`));
  if (BIND_STATES.includes(state)) box.classList.add(`bind-state-${state}`);
}

function formatExpiry(leftSeconds: number) {
  const sec = Math.max(0, Number(leftSeconds) || 0);
  const minutes = Math.floor(sec / 60);
  const seconds = String(sec % 60).padStart(2, '0');
  return `Expires in ${minutes}:${seconds}`;
}

function setHost(payload: BindPayload | null) {
  const host = el('bind-host');
  if (!host) return;
  if (payload?.ip && payload?.port != null) {
    host.hidden = false;
    host.textContent = `This Mac · ${payload.ip}:${payload.port}`;
    return;
  }
  host.hidden = true;
  host.textContent = '';
}

function setFooterHint(text: string) {
  const hint = el('bind-footer-hint');
  if (hint) hint.textContent = text;
}

function stopPolling() {
  if (pollTimer != null) {
    clearInterval(pollTimer);
    pollTimer = null;
  }
}

function stopCountdown() {
  if (countdownTimer != null) {
    clearInterval(countdownTimer);
    countdownTimer = null;
  }
}

function clearPreview() {
  const preview = el('bind-preview');
  if (!preview) return;
  preview.innerHTML = '';
  if (Array.isArray(preview.children)) preview.children.length = 0;
}

function drawPayload(payload: BindPayload) {
  const text = JSON.stringify({
    ip: payload.ip,
    port: payload.port,
    temp_pub: payload.temp_pub,
    tls_fingerprint: payload.tls_fingerprint,
    exp: payload.exp,
    sig: payload.sig,
  });
  const canvas = document.createElement('canvas');
  QRCode.toCanvas(canvas, text, QR_OPTS, (err) => {
    if (err) {
      logBindEvent('qr.render', 'failed');
      stopPolling();
      stopCountdown();
      const status = el('bind-status');
      if (status) status.textContent = 'Unable to generate a binding code.';
      const countdown = el('bind-countdown');
      if (countdown) countdown.textContent = '';
      const refresh = el('btn-bind-refresh');
      if (refresh) refresh.hidden = false;
      setBindState('error');
      setHost(null);
      setFooterHint('Try issuing a new code.');
      return;
    }
    el('bind-preview')?.appendChild(canvas);
    logBindEvent('qr.render', 'succeeded');
  });
}

function startCountdown(exp: number) {
  stopCountdown();
  const tick = () => {
    const left = Math.max(0, Number(exp) - Math.floor(Date.now() / 1000));
    const countdown = el('bind-countdown');
    if (countdown) countdown.textContent = left === 0 ? 'Expired' : formatExpiry(left);
  };
  tick();
  countdownTimer = setInterval(tick, 1000);
}

function startPolling() {
  stopPolling();
  pollTimer = setInterval(async () => {
    const state = await api.invoke('read_bind_session');
    if (state === 'consumed') {
      stopPolling();
      stopCountdown();
      const status = el('bind-status');
      if (status) status.textContent = 'Binding successful';
      const countdown = el('bind-countdown');
      if (countdown) countdown.textContent = 'Complete';
      setBindState('success');
      setFooterHint('This Mac is now bound to the Android app.');
      return;
    }
    if (state === 'expired') {
      clearPreview();
      stopCountdown();
      const status = el('bind-status');
      if (status) status.textContent = 'This code has expired.';
      const countdown = el('bind-countdown');
      if (countdown) countdown.textContent = 'Expired';
      const refresh = el('btn-bind-refresh');
      if (refresh) refresh.hidden = false;
      setBindState('expired');
      setHost(null);
      setFooterHint('Issue a new code to continue pairing.');
    }
  }, POLL_MS);
}

export async function openBindDialog() {
  logBindEvent('issue_bind', 'started');
  stopPolling();
  stopCountdown();
  bindOpenStore.set(true);
  el('bind-dialog')?.classList.add('open');
  setBindState('loading');
  const status = el('bind-status');
  if (status) status.textContent = 'Loading…';
  const countdown = el('bind-countdown');
  if (countdown) countdown.textContent = '';
  const refresh = el('btn-bind-refresh');
  if (refresh) refresh.hidden = true;
  setHost(null);
  setFooterHint('A new code is available if this one expires.');
  clearPreview();
  try {
    const payload = (await api.invoke('issue_bind')) as BindPayload;
    setHost(payload);
    if (status) status.textContent = 'Waiting for the Android app';
    setBindState('waiting');
    startCountdown(payload.exp ?? 0);
    startPolling();
    logBindEvent('issue_bind', 'succeeded');
    drawPayload(payload);
  } catch (error) {
    const code = bindErrorCode(error);
    logBindEvent('issue_bind', code);
    clearPreview();
    setHost(null);
    setBindState('error');
    if (countdown) countdown.textContent = '';
    if (status) status.textContent = ISSUE_ERROR_STATUS[code] || 'Unable to issue a binding code.';
    if (refresh) refresh.hidden = false;
    setFooterHint('Fix the issue below, then issue a new code.');
  } finally {
    if (el('bind-status')?.textContent === 'Loading…') {
      const late = el('bind-status');
      if (late) late.textContent = '';
    }
  }
}

export function closeBindDialog() {
  stopPolling();
  stopCountdown();
  bindOpenStore.set(false);
  el('bind-dialog')?.classList.remove('open');
}
