import { state } from '../state.js'
import { formatDate } from '../utils.js'
import { renderDocList } from './cards.js'

// ── buildGroups ────────────────────────────────────────────────────────────

export function buildGroups(indexData) {
  const map = new Map();
  for (const [id, entry] of Object.entries(indexData)) {
    const date = entry.created_at.slice(0, 8);
    if (!map.has(date)) map.set(date, []);
    map.get(date).push({ id, entry });
  }
  return [...map.entries()]
    .sort((a, b) => b[0].localeCompare(a[0]))
    .map(([date, entries]) => ({
      date,
      entries: entries.sort((a, b) => b.entry.created_at.localeCompare(a.entry.created_at))
    }));
}

// ── renderSidebar ──────────────────────────────────────────────────────────

export function renderSidebar() {
  const aside = document.getElementById('sidebar');
  aside.innerHTML = '';
  for (const { date, entries } of state.index.groupedByDate) {
    const d = formatDate(date);
    const tab = document.createElement('div');
    tab.className = 'date-tab';
    tab.dataset.date = date;
    tab.innerHTML = `
      <span class="day">${d.label}</span>
      <span class="month">${d.year} ${d.weekday}</span>
      <span class="count">${entries.length}</span>
    `;
    tab.addEventListener('click', () => selectDate(date));
    aside.appendChild(tab);
  }
}

// ── selectDate ────────────────────────────────────────────────────────

export function selectDate(date) {
  state.ui.activeDate = date;
  sessionStorage.setItem('cta_active_date', date);

  // Update active tab
  document.querySelectorAll('.date-tab').forEach(t => {
    t.classList.toggle('active', t.dataset.date === date);
  });

  const group = state.index.groupedByDate.find(g => g.date === date);
  if (!group) return;

  const d = formatDate(date);
  document.getElementById('status').style.display = 'none';
  const heading = document.getElementById('date-heading');
  heading.style.display = '';
  heading.textContent = d.full + `  ·  ${group.entries.length} 篇`;

  renderDocList(group.entries, date);
  window.loadTitles(group.entries, date);
}
