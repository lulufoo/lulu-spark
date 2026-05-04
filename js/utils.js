export function slugToTitle(slug) {
  return slug
    .replace(/^\d{12}-/, '')                     // strip timestamp prefix if any
    .split('-')
    .map(w => w.charAt(0).toUpperCase() + w.slice(1))
    .join(' ');
}

export function filenameFromPath(commonPath) {
  return commonPath.split('/').pop().replace(/\.md$/, '');
}

export function topicFromPath(commonPath) {
  const parts = commonPath.split('/');
  parts.pop();
  return parts.join('/');
}

export function formatDate(dateStr) {
  // dateStr = 'YYYYMMDD'
  const y = dateStr.slice(0, 4);
  const m = dateStr.slice(4, 6);
  const d = dateStr.slice(6, 8);
  const dt = new Date(`${y}-${m}-${d}`);
  const days = ['日','一','二','三','四','五','六'];
  const dow = days[dt.getDay()];
  return {
    day: `${parseInt(d)}日`,
    label: `${parseInt(m)}月${parseInt(d)}日`,
    year: y,
    month: `${y}/${m}`,
    weekday: `周${dow}`,
    full: `${y}年${parseInt(m)}月${parseInt(d)}日（周${dow}）`
  };
}

export function timeFromTs(ts) {
  // ts = 'YYYYMMDDHHMM'
  return ts.slice(8, 10) + ':' + ts.slice(10, 12);
}

export function escHtml(s) {
  return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
}

export function nowTs() {
  const now = new Date();
  return [
    now.getFullYear(),
    String(now.getMonth() + 1).padStart(2, '0'),
    String(now.getDate()).padStart(2, '0'),
    String(now.getHours()).padStart(2, '0'),
    String(now.getMinutes()).padStart(2, '0')
  ].join('');
}

export function importanceBadgeHtml(importance) {
  const map = {
    high:   { cls: 'badge-importance-high',   label: '↑ 高' },
    medium: { cls: 'badge-importance-medium', label: '→ 中' },
    low:    { cls: 'badge-importance-low',    label: '↓ 低' },
  };
  const m = map[importance];
  if (m) return `<button class="badge badge-importance ${m.cls}" data-action="cycle-importance" title="切换重要性">${m.label}</button>`;
  return `<button class="badge badge-importance badge-importance-unset" data-action="cycle-importance" title="设置重要性">☆</button>`;
}
