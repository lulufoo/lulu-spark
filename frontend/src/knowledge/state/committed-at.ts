export function formatCommittedAt(unix: unknown): string {
  if (typeof unix !== 'number' || !Number.isFinite(unix) || unix <= 0) return '';
  const date = new Date(unix * 1000);
  if (Number.isNaN(date.getTime())) return '';
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

export function paintKbCommittedAt(el: HTMLElement | null, unix: unknown) {
  if (!el) return;
  const text = formatCommittedAt(unix);
  el.textContent = text;
  el.hidden = !text;
}