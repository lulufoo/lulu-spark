// @ts-nocheck — ported from JS; state shapes stay unchecked like checkJs:false.
import { useSyncExternalStore } from 'react';
import { flushSync } from 'react-dom';
import { getEntryId } from '../host/state.ts'
import * as api from '../host/api.ts'
import { renderToHtml } from '../island.ts';
import { createModuleStore } from '../shared/module-store.ts';

const openStore = createModuleStore(false);

let _currentEntry = null;

// entry is passed directly from the card; no dependency on state.viewer
export async function openMoveProjectDialog(entry) {
  if (!entry) return;

  const currentProject = entry.common_path.split('/')[0];
  _currentEntry = entry;
  flushSync(() => {
    openStore.set(true);
  });
  document.getElementById('move-project-dialog')?.classList.add('open');

  const result = document.getElementById('move-project-result');
  if (result) {
    result.textContent = 'Loading project list…';
    result.style.color = '#8c959f';
  }

  await _loadProjects(currentProject);
}

export function closeMoveProjectDialog() {
  openStore.set(false);
  document.getElementById('move-project-dialog')?.classList.remove('open');
  const list = document.getElementById('move-project-list');
  if (list) list.innerHTML = '';
  const result = document.getElementById('move-project-result');
  if (result) result.textContent = '';
  _currentEntry = null;
}

function ProjectListItem({ proj, desc }) {
  const inbox = proj === 'inbox';
  return (
    <button
      type="button"
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
  if (result) {
    result.textContent = 'Loading project list…';
    result.style.color = '#8c959f';
  }

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
    if (result) {
      result.style.color = '#cf222e';
      result.textContent = 'Failed to load: ' + e.message;
    }
    return;
  }

  if (!list) return;
  try {
    const items = projects.filter((proj) => proj !== currentProject);
    list.innerHTML = renderToHtml(
      <>
        {items.map((proj) => (
          <ProjectListItem key={proj} proj={proj} desc={topicsMap[proj] || ''} />
        ))}
      </>,
    );
    if (result) {
      result.textContent = `Current project: ${currentProject} — choose target`;
      result.style.color = '#57606a';
    }
  } catch (e) {
    console.error('[move-project] render error', e);
    if (result) {
      result.style.color = '#cf222e';
      result.textContent = 'Render failed: ' + e.message;
    }
  }
}

async function doMoveProject(newProject) {
  const entry = _currentEntry;
  if (!entry) return;

  const result = document.getElementById('move-project-result');
  if (result) {
    result.style.color = '#57606a';
    result.textContent = 'Moving…';
  }

  document.querySelectorAll('.move-project-item').forEach(b => b.disabled = true);

  try {
    const id = getEntryId(entry);
    if (!id) throw new Error('Entry id not found');
    const data = await api.moveToProject(id, newProject);
    if (!data.ok) throw new Error(data.error || 'failed');

    if (result) {
      result.style.color = '#1a7f37';
      result.textContent = `✓ Moved to ${newProject}`;
    }

    setTimeout(() => {
      closeMoveProjectDialog();
      document.dispatchEvent(new CustomEvent('cta:reload'));
    }, 800);
  } catch (e) {
    if (result) {
      result.style.color = '#cf222e';
      result.textContent = `✗ ${e.message}`;
    }
    document.querySelectorAll('.move-project-item').forEach(b => b.disabled = false);
  }
}

export function MoveProjectDialog() {
  const open = useSyncExternalStore(openStore.subscribe, openStore.getSnapshot);

  return (
    <div id="move-project-dialog" className={open ? 'open' : undefined}>
      <div id="move-project-backdrop" onClick={() => closeMoveProjectDialog()}></div>
      <div id="move-project-dialog-box">
        <h3>↷ Switch project</h3>
        <div id="move-project-result" style={{ fontSize: '12px', color: '#57606a', marginBottom: '8px' }}></div>
        <div
          id="move-project-list"
          style={{ display: 'flex', flexDirection: 'column', gap: '4px', maxHeight: '300px', overflowY: 'auto' }}
          onClick={(e) => {
            const btn = e.target.closest?.('.move-project-item');
            if (btn instanceof HTMLElement && btn.dataset.project) {
              void doMoveProject(btn.dataset.project);
            }
          }}
        ></div>
        <div id="move-project-dialog-actions">
          <button id="btn-move-project-close" type="button" onClick={() => closeMoveProjectDialog()}>
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
}
