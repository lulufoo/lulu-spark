export function slugToTitle(slug: string): string {
  return slug
    .replace(/^\d{12}-/, '')                     // strip timestamp prefix if any
    .split('-')
    .map(w => w.charAt(0).toUpperCase() + w.slice(1))
    .join(' ');
}

export function filenameFromPath(commonPath: string): string {
  return (commonPath.split('/').pop() ?? '').replace(/\.md$/, '');
}

/** GitHub full_name (owner/repo) → repo segment only for display. */
export function repoShortName(fullName: string | null | undefined): string {
  if (!fullName) return '';
  const slash = fullName.indexOf('/');
  return slash >= 0 ? fullName.slice(slash + 1) : fullName;
}

export function topicFromPath(commonPath: string): string {
  const parts = commonPath.split('/');
  parts.pop();
  return parts.join('/');
}

export function formatDate(dateStr: string) {
  // dateStr = 'YYYYMMDD'
  const y = dateStr.slice(0, 4);
  const m = dateStr.slice(4, 6);
  const d = dateStr.slice(6, 8);
  const dt = new Date(`${y}-${m}-${d}`);
  const days = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
  const months = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
  const dow = days[dt.getDay()];
  const mon = months[parseInt(m, 10) - 1];
  return {
    day: String(parseInt(d, 10)),
    label: `${mon} ${parseInt(d, 10)}`,
    year: y,
    month: `${y}/${m}`,
    weekday: dow,
    full: `${mon} ${parseInt(d, 10)}, ${y} (${dow})`
  };
}

export function timeFromTs(ts: string): string {
  // ts = 'YYYYMMDDHHMM'
  return ts.slice(8, 10) + ':' + ts.slice(10, 12);
}

export function escHtml(s: string): string {
  return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
}

function syncEditAreaScrollTop(editArea: HTMLTextAreaElement | HTMLInputElement) {
  editArea.scrollTop = 0;
  try {
    editArea.setSelectionRange(0, 0);
  } catch (_e) {
    // hidden/disabled textarea (jsdom or browser)
  }
}

export function resetEditAreaScroll(
  editArea: HTMLTextAreaElement | HTMLInputElement | null | undefined,
  { focus = false } = {},
) {
  if (!editArea) return;
  syncEditAreaScrollTop(editArea);
  if (!focus) return;
  try {
    editArea.focus({ preventScroll: true });
  } catch (_e) {
    editArea.focus();
  }
  requestAnimationFrame(() => syncEditAreaScrollTop(editArea));
}

export function nowTs(): string {
  const now = new Date();
  return [
    now.getFullYear(),
    String(now.getMonth() + 1).padStart(2, '0'),
    String(now.getDate()).padStart(2, '0'),
    String(now.getHours()).padStart(2, '0'),
    String(now.getMinutes()).padStart(2, '0')
  ].join('');
}

export function importanceBadgeHtml(importance: string | null | undefined): string {
  const map: Record<string, { cls: string; label: string }> = {
    high:   { cls: 'badge-importance-high',   label: '↑ High' },
    medium: { cls: 'badge-importance-medium', label: '→ Medium' },
    low:    { cls: 'badge-importance-low',    label: '↓ Low' },
  };
  const m = importance ? map[importance] : undefined;
  if (m) return `<button class="badge badge-importance ${m.cls}" data-action="cycle-importance" title="Cycle importance">${m.label}</button>`;
  return `<button class="badge badge-importance badge-importance-unset" data-action="cycle-importance" title="Set importance">☆</button>`;
}
