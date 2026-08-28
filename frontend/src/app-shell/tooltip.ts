// @ts-nocheck — DOM wiring stays unchecked like checkJs:false.
export function initTooltip() {
  const box = document.createElement('div');
  box.id = '_tip';
  document.body.appendChild(box);
  let timer = null;

  function show(el, x, y) {
    const text = el.dataset.tip;
    if (!text) return;
    box.textContent = text;
    const gap = 8;
    let top = y + gap;
    let left = x + gap;
    box.classList.remove('visible');
    box.style.display = 'block';
    const bw = box.offsetWidth, bh = box.offsetHeight;
    if (top + bh > window.innerHeight - 4) top = y - bh - gap;
    if (left + bw > window.innerWidth - 4) left = window.innerWidth - bw - 4;
    box.style.left = left + 'px';
    box.style.top = top + 'px';
    box.classList.add('visible');
  }

  function hide() {
    clearTimeout(timer);
    box.classList.remove('visible');
  }

  document.addEventListener('mouseover', e => {
    const el = e.target.closest('[data-tip]');
    if (!el) { hide(); return; }
    if (!el.dataset.tip) { hide(); return; }
    clearTimeout(timer);
    timer = setTimeout(() => show(el, e.clientX, e.clientY), 150);
  }, true);

  document.addEventListener('mousemove', e => {
    if (!box.classList.contains('visible')) return;
    const el = e.target.closest('[data-tip]');
    if (!el) return;
    let top = e.clientY + 8, left = e.clientX + 8;
    const bw = box.offsetWidth, bh = box.offsetHeight;
    if (top + bh > window.innerHeight - 4) top = e.clientY - bh - 8;
    if (left + bw > window.innerWidth - 4) left = window.innerWidth - bw - 4;
    box.style.left = left + 'px';
    box.style.top = top + 'px';
  }, true);

  document.addEventListener('mouseout', e => {
    const el = e.target.closest('[data-tip]');
    if (!el) return;
    if (el.contains(e.relatedTarget)) return;
    clearTimeout(timer);
    hide();
  }, true);
}
