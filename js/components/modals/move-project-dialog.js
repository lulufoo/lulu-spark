import { getEntryId } from '../../state.js'
import * as api from '../../api.js'

// ── openMoveProjectDialog ──────────────────────────────────────────────────

// entry is passed directly from the card; no dependency on state.viewer
export async function openMoveProjectDialog(entry) {
  if (!entry) return;

  const currentProject = entry.common_path.split('/')[0];
  const result = document.getElementById('move-project-result');
  result.textContent = '加载项目列表…';
  result.style.color = '#8c959f';
  document.getElementById('move-project-dialog').classList.add('open');
  // store reference for doMoveProject
  _currentEntry = entry;

  await _loadProjects(currentProject);
}

async function _loadProjects(currentProject) {
  const result = document.getElementById('move-project-result');
  const list = document.getElementById('move-project-list');
  result.textContent = '加载项目列表…';
  result.style.color = '#8c959f';

  // Load topics
  let projects = [];
  let topicsMap = {};
  try {
    const data = await api.fetchTopics();
    for (const t of data.topics) {
      const key = t.dir || (t.repo ? t.repo.split('/').pop() : null);
      if (key) topicsMap[key] = t.description || '';
    }
    projects = data.topics.map(t => t.dir || (t.repo ? t.repo.split('/').pop() : null)).filter(Boolean);
  } catch (e) {
    result.style.color = '#cf222e';
    result.textContent = '加载失败：' + e.message;
    return;
  }

  // Render project list
  list.innerHTML = '';
  for (const proj of projects) {
    if (proj === currentProject) continue;
    const btn = document.createElement('button');
    btn.className = 'move-project-item';
    if (proj === 'inbox') {
      btn.style.color = '#cf222e';
      btn.style.borderColor = '#ffcbc8';
    }
    const desc = topicsMap[proj] || '';
    btn.innerHTML = `<span class="move-project-item-name">${proj}</span>${desc ? `<span class="move-project-item-desc">${desc}</span>` : ''}`;
    btn.addEventListener('click', () => doMoveProject(proj));
    list.appendChild(btn);
  }
  result.textContent = `当前项目：${currentProject}，选择目标项目`;
  result.style.color = '#57606a';
}

let _currentEntry = null;

export function closeMoveProjectDialog() {
  document.getElementById('move-project-dialog').classList.remove('open');
  document.getElementById('move-project-list').innerHTML = '';
  document.getElementById('move-project-result').textContent = '';
  _currentEntry = null;
}

async function doMoveProject(newProject) {
  const entry = _currentEntry;
  if (!entry) return;

  const result = document.getElementById('move-project-result');
  result.style.color = '#57606a';
  result.textContent = '移动中…';

  document.querySelectorAll('.move-project-item').forEach(b => b.disabled = true);

  try {
    const id = getEntryId(entry);
    if (!id) throw new Error('entry id 不存在');
    const data = await api.moveToProject(id, newProject);
    if (!data.ok) throw new Error(data.error || 'failed');

    result.style.color = '#1a7f37';
    result.textContent = `✓ 已移动到 ${newProject}`;

    setTimeout(() => {
      closeMoveProjectDialog();
      document.dispatchEvent(new CustomEvent('cta:reload'));
    }, 800);
  } catch (e) {
    result.style.color = '#cf222e';
    result.textContent = `✗ ${e.message}`;
    document.querySelectorAll('.move-project-item').forEach(b => b.disabled = false);
  }
}

// ── Event listeners ────────────────────────────────────────────────────────

document.getElementById('btn-move-project-close').addEventListener('click', closeMoveProjectDialog);
document.getElementById('move-project-backdrop').addEventListener('click', closeMoveProjectDialog);
document.getElementById('btn-move-project-refresh').addEventListener('click', () => {
  if (!_currentEntry) return;
  const currentProject = _currentEntry.common_path.split('/')[0];
  _loadProjects(currentProject);
});
