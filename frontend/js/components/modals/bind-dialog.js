import * as api from '../../api.js';

const QR_OPTS = { width: 256, margin: 2 };
const POLL_MS = 1000;

let pollTimer = null;
let countdownTimer = null;

function el(id) {
  return document.getElementById(id);
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
  preview.innerHTML = '';
  if (Array.isArray(preview.children)) preview.children.length = 0;
}

function drawPayload(payload) {
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
    if (err) return;
    el('bind-preview').appendChild(canvas);
  });
}

function startCountdown(exp) {
  stopCountdown();
  const tick = () => {
    const left = Math.max(0, Number(exp) - Math.floor(Date.now() / 1000));
    el('bind-countdown').textContent = String(left);
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
      el('bind-status').textContent = 'Binding successful';
      return;
    }
    if (state === 'expired') {
      clearPreview();
      stopCountdown();
      el('btn-bind-refresh').hidden = false;
    }
  }, POLL_MS);
}

export async function openBindDialog() {
  stopPolling();
  stopCountdown();
  el('bind-dialog').classList.add('open');
  el('bind-status').textContent = 'Loading…';
  el('bind-countdown').textContent = '';
  el('btn-bind-refresh').hidden = true;
  clearPreview();
  try {
    const payload = await api.invoke('issue_bind');
    el('bind-status').textContent = '';
    drawPayload(payload);
    startCountdown(payload.exp);
    startPolling();
  } catch (err) {
    el('bind-status').textContent = '';
    clearPreview();
  }
}

function closeBindDialog() {
  stopPolling();
  stopCountdown();
  el('bind-dialog').classList.remove('open');
}

el('btn-bind-close').addEventListener('click', closeBindDialog);
el('btn-bind-refresh').addEventListener('click', () => openBindDialog());
