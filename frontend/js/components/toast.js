const DISMISS_MS = 3000;

const BASE_STYLE = [
  'position:fixed',
  'bottom:24px',
  'left:50%',
  'transform:translateX(-50%)',
  'padding:10px 16px',
  'border-radius:6px',
  'font-size:13px',
  'font-weight:500',
  'z-index:10000',
  'box-shadow:0 4px 12px rgba(0,0,0,0.15)',
].join(';');

const TYPE_STYLES = {
  success: 'background:#dafbe1;color:#116329;border:1px solid #4ac26b',
  error: 'background:#ffebe9;color:#82071e;border:1px solid #ff8182',
};

export function showToast(message, type) {
  const el = document.createElement('div');
  el.className = `wb-toast wb-toast-${type}`;
  el.textContent = message;
  el.setAttribute('role', 'status');
  el.style.cssText = `${BASE_STYLE};${TYPE_STYLES[type]}`;
  document.body.appendChild(el);

  setTimeout(() => {
    el.remove();
  }, DISMISS_MS);
}
