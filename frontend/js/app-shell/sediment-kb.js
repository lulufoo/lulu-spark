import { escHtml } from '../shared/utils.js';
import * as api from '../host/api.js';
import { openKbDiffDialog } from '../corpus/corpus-diff-dialog.js';

let _kbCorpusStatus = null;
let _kbCorpusDiffStatus = null;
let _sedimentKbList = null;
let _sedimentKbError = null;
let _sedimentKbCategories = null;

export function _setSedimentKbError(elId, message) {
  const el = document.getElementById(elId);
  if (!el) return;
  if (message) {
    el.textContent = message;
    el.style.display = '';
  } else {
    el.textContent = '';
    el.style.display = 'none';
  }
}

export async function _ensureSedimentKbCategories() {
  if (_sedimentKbCategories) return _sedimentKbCategories;
  const data = await api.fetchSedimentKbCategories();
  _sedimentKbCategories = data.categories || [];
  return _sedimentKbCategories;
}

export function bindSedimentKbListActions(content) {
  content.querySelectorAll('.repo-sync-btn').forEach(btn => {
    btn.addEventListener('click', async () => {
      const repo = btn.dataset.repo;
      btn.disabled = true;
      btn.textContent = '…';
      try {
        await api.reindexKbRepo(repo);
        const poll = () => api.getReindexStatus();
        for (let i = 0; i < 120; i++) {
          await new Promise(r => setTimeout(r, 2000));
          const s = await poll();
          if (s?.status !== 'running') break;
        }
        _kbCorpusStatus = null;
        await loadSedimentKbList(true);
      } catch (e) {
        alert(`Sync failed: ${e.message}`);
        btn.disabled = false;
        btn.textContent = 'SYNC';
      }
    });
  });

  content.querySelectorAll('.repo-diff-badge').forEach(btn => {
    btn.addEventListener('click', () => {
      openKbDiffDialog(btn.dataset.repo);
    });
  });

  content.querySelectorAll('.sediment-kb-inline-category').forEach(sel => {
    sel.addEventListener('change', () => {
      onInlineCategoryChange(sel.dataset.repo, sel.value);
    });
  });

  content.querySelectorAll('.sediment-kb-delete-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      onDeleteSedimentKbRepo(btn.dataset.repo);
    });
  });
}

export function renderSedimentKbListByCategory(repos) {
  const content = document.getElementById('repo-list-content');
  if (!content) return;
  if (!repos || repos.length === 0) {
    content.innerHTML = '<div class="repo-list-loading">No repositories found</div>';
    return;
  }

  const categories = _sedimentKbCategories || [];
  const statusMap = {};
  if (_kbCorpusStatus) {
    for (const s of _kbCorpusStatus) statusMap[s.full_name] = s;
  }

  const groups = {};
  for (const r of repos) {
    const key = r.category_name || 'Uncategorized';
    if (!groups[key]) groups[key] = [];
    groups[key].push(r);
  }
  for (const key of Object.keys(groups)) {
    groups[key].sort((a, b) => (a.name || '').localeCompare(b.name || '', 'en', { sensitivity: 'base' }));
  }

  const sortedKeys = Object.keys(groups).sort((a, b) => {
    if (a === 'Uncategorized') return 1;
    if (b === 'Uncategorized') return -1;
    return a.localeCompare(b, 'en', { sensitivity: 'base' });
  });

  const html = sortedKeys.map(key => {
    const items = groups[key].map(r => {
      const name = escHtml(r.name || r.full_name || '');
      const desc = r.description ? `<div class="repo-list-item-desc">${escHtml(r.description)}</div>` : '';
      const url = `https://github.com/${escHtml(r.full_name || r.name)}`;

      let localBadge = '';
      let diffBtnHtml = '';
      let syncBtnHtml = '';
      const st = statusMap[r.full_name];
      if (st) {
        localBadge = st.local_exists
          ? `<span class="repo-local-badge repo-local-ok">Cloned</span>`
          : `<span class="repo-local-badge repo-local-missing">Not cloned</span>`;
        if (_kbCorpusDiffStatus?.get(r.full_name) === true) {
          diffBtnHtml = `<button class="repo-diff-badge" data-repo="${escHtml(r.full_name)}" title="View local changes">✎</button>`;
        }
        syncBtnHtml = `<button class="repo-sync-btn" data-repo="${escHtml(r.full_name)}">SYNC</button>`;
      }

      const catSelectOptions = categories.map(c => {
        const sel = c.id === r.category_id ? ' selected' : '';
        return `<option value="${escHtml(c.id)}"${sel}>${escHtml(c.name)}</option>`;
      }).join('');

      return `<div class="repo-list-item">
        <div class="repo-list-item-info">
          <div class="repo-list-item-name">${name}${localBadge}</div>
          ${desc}
        </div>
        <div class="repo-list-item-actions">
          <select class="sediment-kb-inline-category" data-repo="${escHtml(r.full_name)}">${catSelectOptions}</select>
          ${diffBtnHtml}
          ${syncBtnHtml}
          <a class="repo-list-item-link" href="${url}" target="_blank" rel="noopener noreferrer">Link ↗</a>
          <button type="button" class="sediment-kb-delete-btn" data-repo="${escHtml(r.full_name)}" title="Remove from curated list">Delete</button>
        </div>
      </div>`;
    }).join('');
    return `<div class="repo-list-group-title">${escHtml(key)}</div>${items}`;
  }).join('');

  content.innerHTML = html;
  bindSedimentKbListActions(content);
}

export async function onInlineCategoryChange(fullName, categoryId) {
  try {
    const res = await api.updateSedimentKbRepoCategory(fullName, categoryId);
    if (res?.error) throw new Error(res.error);
    _sedimentKbList = null;
    await loadSedimentKbList(true);
  } catch (e) {
    alert(`Failed to update category: ${e.message}`);
    await loadSedimentKbList(true);
  }
}

export async function onDeleteSedimentKbRepo(fullName) {
  try {
    const res = await api.removeSedimentKbRepo(fullName);
    if (res?.error) throw new Error(res.error);
    _sedimentKbList = null;
    await loadSedimentKbList(true);
  } catch (e) {
    alert(`Delete failed: ${e.message}`);
  }
}

export async function loadSedimentKbList(forceRefresh = false) {
  if (!forceRefresh && _sedimentKbList && !_sedimentKbError) {
    renderSedimentKbListByCategory(_sedimentKbList);
    return;
  }
  _kbCorpusStatus = null;
  _sedimentKbList = null;
  _sedimentKbError = null;
  _kbCorpusDiffStatus = null;
  try {
    const [reposData, catsData] = await Promise.all([
      api.fetchSedimentKbRepos(),
      api.fetchSedimentKbCategories(),
    ]);
    if (reposData?.error) throw new Error(reposData.error);
    _sedimentKbCategories = catsData.categories || [];
    _sedimentKbList = (reposData.repos || []).map((r) => ({
      full_name: r.full_name,
      name: (r.full_name || '').split('/').pop() || r.full_name,
      description: r.description || '',
      category_id: r.category_id,
      category_name: r.category_name,
      local_exists: r.local_exists === true,
    }));
    _kbCorpusStatus = _sedimentKbList.map((r) => ({
      full_name: r.full_name,
      name: r.name,
      description: r.description,
      local_exists: r.local_exists === true,
    }));
  } catch (e) {
    _sedimentKbError = e.message || String(e);
    _sedimentKbList = [];
    _kbCorpusStatus = [];
    const list = document.getElementById('repo-list-content');
    if (list) {
      list.innerHTML = `<div class="repo-list-loading" style="color:#cf222e">Failed to load: ${escHtml(_sedimentKbError)}</div>`;
    }
    return;
  }
  renderSedimentKbListByCategory(_sedimentKbList);

  _kbCorpusDiffStatus = null;
  try {
    const diffData = await api.fetchKbDiffStatus();
    _kbCorpusDiffStatus = new Map(
      (diffData?.repos || []).map((repo) => [repo.full_name, repo.has_changes === true]),
    );
  } catch {
    _kbCorpusDiffStatus = new Map();
  }
  renderSedimentKbListByCategory(_sedimentKbList);
}

export async function prepareSedimentKbList() {
  const content = document.getElementById('repo-list-content');
  if (content && !content.innerHTML.trim()) {
    content.innerHTML = '<div id="repo-list-loading">Loading…</div>';
  }
  await loadSedimentKbList(true);
}

export async function prepareSedimentKbAddForm() {
  const urlInput = document.getElementById('sediment-kb-add-url');
  if (!urlInput) return;
  _setSedimentKbError('sediment-kb-add-error', '');
  urlInput.value = '';
  document.getElementById('sediment-kb-add-description').value = '';
  try {
    const categories = await _ensureSedimentKbCategories();
    const catSelect = document.getElementById('sediment-kb-add-category');
    catSelect.innerHTML = categories.map(c =>
      `<option value="${escHtml(c.id)}">${escHtml(c.name)}</option>`,
    ).join('');
  } catch (e) {
    document.getElementById('sediment-kb-add-category').innerHTML =
      '<option value="uncategorized">Uncategorized</option>';
  }
}

export function _renderSedimentKbManageList(categories) {
  const list = document.getElementById('sediment-kb-manage-list');
  list.innerHTML = categories.map(c => {
    const isProtected = c.id === 'uncategorized';
    const deleteBtn = isProtected
      ? ''
      : `<button type="button" class="sediment-kb-cat-delete-btn" data-id="${escHtml(c.id)}">Delete</button>`;
    const nameCell = isProtected
      ? `<span class="sediment-kb-cat-name-readonly">${escHtml(c.name)}</span>`
      : `<input class="sediment-kb-cat-rename-input" data-id="${escHtml(c.id)}" type="text" value="${escHtml(c.name)}" />`;
    return `<div class="sediment-kb-manage-row">${nameCell}${deleteBtn}</div>`;
  }).join('');

  list.querySelectorAll('.sediment-kb-cat-rename-input').forEach(input => {
    input.addEventListener('change', async () => {
      const id = input.dataset.id;
      const name = input.value.trim();
      if (!name) return;
      try {
        const res = await api.renameSedimentKbCategory(id, name);
        if (res?.error) throw new Error(res.error);
        _sedimentKbCategories = null;
        const data = await api.fetchSedimentKbCategories();
        _sedimentKbCategories = data.categories || [];
        _renderSedimentKbManageList(_sedimentKbCategories);
      } catch (e) {
        _setSedimentKbError('sediment-kb-manage-error', e.message);
      }
    });
  });

  list.querySelectorAll('.sediment-kb-cat-delete-btn').forEach(btn => {
    btn.addEventListener('click', async () => {
      try {
        const res = await api.removeSedimentKbCategory(btn.dataset.id);
        if (res?.error) throw new Error(res.error);
        _sedimentKbCategories = null;
        _sedimentKbList = null;
        const data = await api.fetchSedimentKbCategories();
        _sedimentKbCategories = data.categories || [];
        _renderSedimentKbManageList(_sedimentKbCategories);
        _setSedimentKbError('sediment-kb-manage-error', '');
      } catch (e) {
        _setSedimentKbError('sediment-kb-manage-error', e.message);
      }
    });
  });
}

export async function prepareSedimentKbManage() {
  _setSedimentKbError('sediment-kb-manage-error', '');
  const nameInput = document.getElementById('sediment-kb-manage-new-name');
  if (nameInput) nameInput.value = '';
  try {
    const categories = await _ensureSedimentKbCategories();
    _renderSedimentKbManageList(categories);
  } catch (e) {
    const list = document.getElementById('sediment-kb-manage-list');
    if (list) {
      list.innerHTML = `<div class="sediment-kb-error">${escHtml(e.message)}</div>`;
    }
  }
}

window.addEventListener('kb-diff-updated', () => {
  void loadSedimentKbList(true);
});

window.addEventListener('settings-knowledge-tab', (e) => {
  const tabId = e.detail;
  if (tabId === 'list') void prepareSedimentKbList();
  else if (tabId === 'add') void prepareSedimentKbAddForm();
  else if (tabId === 'categories') void prepareSedimentKbManage();
});

document.getElementById('btn-repo-list-refresh')?.addEventListener('click', () => {
  void loadSedimentKbList(true);
});

document.getElementById('btn-sediment-kb-add-submit')?.addEventListener('click', () => {
  void (async () => {
    const urlInput = document.getElementById('sediment-kb-add-url');
    const catSelect = document.getElementById('sediment-kb-add-category');
    const descInput = document.getElementById('sediment-kb-add-description');
    const submitBtn = document.getElementById('btn-sediment-kb-add-submit');
    const fullName = urlInput.value.trim();
    if (!fullName) {
      _setSedimentKbError('sediment-kb-add-error', 'Enter repository URL');
      return;
    }
    submitBtn.disabled = true;
    _setSedimentKbError('sediment-kb-add-error', '');
    try {
      const categoryId = catSelect.value || undefined;
      const description = descInput.value.trim() || undefined;
      const res = await api.addSedimentKbRepo(fullName, categoryId, description);
      if (res?.error) throw new Error(res.error);
      _sedimentKbList = null;
      _sedimentKbCategories = null;
      urlInput.value = '';
      descInput.value = '';
      _setSedimentKbError('sediment-kb-add-error', 'Added.');
    } catch (e) {
      _setSedimentKbError('sediment-kb-add-error', e.message);
    } finally {
      submitBtn.disabled = false;
    }
  })();
});

document.getElementById('btn-sediment-kb-manage-add')?.addEventListener('click', () => {
  void (async () => {
    const input = document.getElementById('sediment-kb-manage-new-name');
    const name = input.value.trim();
    if (!name) return;
    try {
      const res = await api.addSedimentKbCategory(name);
      if (res?.error) throw new Error(res.error);
      input.value = '';
      _sedimentKbCategories = null;
      const data = await api.fetchSedimentKbCategories();
      _sedimentKbCategories = data.categories || [];
      _renderSedimentKbManageList(_sedimentKbCategories);
      _setSedimentKbError('sediment-kb-manage-error', '');
    } catch (e) {
      _setSedimentKbError('sediment-kb-manage-error', e.message);
    }
  })();
});

document.getElementById('sediment-kb-add-url')?.addEventListener('keydown', e => {
  if (e.key === 'Enter') document.getElementById('btn-sediment-kb-add-submit')?.click();
});
document.getElementById('sediment-kb-manage-new-name')?.addEventListener('keydown', e => {
  if (e.key === 'Enter') document.getElementById('btn-sediment-kb-manage-add')?.click();
});
