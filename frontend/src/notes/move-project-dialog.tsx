// @ts-nocheck — ported from JS; state shapes stay unchecked like checkJs:false.
import { getEntryId } from '../host/state.ts'
import * as api from '../host/api.ts'
import { renderToHtml } from '../island.ts';

// ── openMoveProjectDialog ──────────────────────────────────────────────────

// entry is passed directly from the card; no dependency on state.viewer
export async function openMoveProjectDialog(entry) {
  if (!entry) return;

  const currentProject = entry.common_path.split('/')[0];
  const result = document.getElementById('move-project-result');
  result.textContent = 'Loading project list…';
  result.style.color = '#8c959f';
  document.getElementById('move-project-dialog').classList.add('open');
  // store reference for doMoveProject
  _currentEntry = entry;

  await _loadProjects(currentProject);
}

function ProjectListItem({ proj, desc }) {
  const inbox = proj === 'inbox';
  return (
    <button
      className="move-project-item"
      data-project={proj}
      style={inbox ? { color: '#cf222e', borderColor: '#ffcbc8' } : undefined}
    >
      <span className="move-project-item-name">{proj}</span>
      {desc ? <span className="move-project-item-desc">{desc}</span> : null}
    </button>
  );
}

async function _loadProjects(currentProject) {
  const result = document.getElementById('move-project-result');
  const list = document.getElementById('move-project-list');
  result.textContent = 'Loading project list…';
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
    projects.sort((a, b) => a.localeCompare(b, 'en', { sensitivity: 'base' }));
  } catch (e) {
    result.style.color = '#cf222e';
    result.textContent = 'Failed to load: ' + e.message;
    return;
  }

  // Render project list
  try {
    const items = projects.filter((proj) => proj !== currentProject);
    list.innerHTML = renderToHtml(
      <>
        {items.map((proj) => (
          <ProjectListItem key={proj} proj={proj} desc={topicsMap[proj] || ''} />
        ))}
      </>,
    );
    list.querySelectorAll('.move-project-item').forEach((btn) => {
      btn.addEventListener('click', () => doMoveProject(btn.dataset.project));
    });
    result.textContent = `Current project: ${currentProject} — choose target`;
    result.style.color = '#57606a';
  } catch (e) {
    console.error('[move-project] render error', e);
    result.style.color = '#cf222e';
    result.textContent = 'Render failed: ' + e.message;
  }
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
  result.textContent = 'Moving…';

  document.querySelectorAll('.move-project-item').forEach(b => b.disabled = true);

  try {
    const id = getEntryId(entry);
    if (!id) throw new Error('Entry id not found');
    const data = await api.moveToProject(id, newProject);
    if (!data.ok) throw new Error(data.error || 'failed');

    result.style.color = '#1a7f37';
    result.textContent = `✓ Moved to ${newProject}`;

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
