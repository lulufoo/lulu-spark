import { Fragment, useSyncExternalStore } from 'react';
import * as api from '../../../host/api.ts';
import { openKnowledgeDiffDialog } from '../../../knowledge/ui/knowledge-diff-dialog.tsx';
import {
  patchSedimentKb,
  sedimentKbStore,
  type SedimentKbCategory,
  type SedimentKbRepo,
  type SedimentKbStatus,
} from '../../state/settings/sediment-kb.ts';
import { errMessage } from '../../state/types.ts';

export function _setSedimentKbError(elId: string, message: string) {
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
  const snap = sedimentKbStore.getSnapshot();
  if (snap.categories.length) return snap.categories;
  const data = (await api.fetchSedimentKbCategories()) as { categories?: SedimentKbCategory[] };
  const categories = data.categories || [];
  patchSedimentKb({ categories });
  return categories;
}

function RepoListItem({
  repo,
  categories,
  status,
  hasDiff,
  syncing,
}: {
  repo: SedimentKbRepo;
  categories: SedimentKbCategory[];
  status?: SedimentKbStatus;
  hasDiff?: boolean;
  syncing?: boolean;
}) {
  const name = repo.name || repo.full_name || '';
  const url = `https://github.com/${repo.full_name || repo.name}`;
  return (
    <div className="repo-list-item">
      <div className="repo-list-item-info">
        <div className="repo-list-item-name">
          {name}
          {status ? (
            status.local_exists
              ? <span className="repo-local-badge repo-local-ok">Cloned</span>
              : <span className="repo-local-badge repo-local-missing">Not cloned</span>
          ) : null}
        </div>
        {repo.description ? <div className="repo-list-item-desc">{repo.description}</div> : null}
      </div>
      <div className="repo-list-item-actions">
        <select
          className="sediment-kb-inline-category"
          data-repo={repo.full_name}
          value={repo.category_id}
          onChange={(e) => {
            void onInlineCategoryChange(repo.full_name, (e.target as HTMLSelectElement).value);
          }}
        >
          {categories.map((c) => (
            <option key={c.id} value={c.id}>{c.name}</option>
          ))}
        </select>
        {hasDiff ? (
          <button
            type="button"
            className="repo-diff-badge"
            data-repo={repo.full_name}
            title="View local changes"
            onClick={() => openKnowledgeDiffDialog(repo.full_name)}
          >
            ✎
          </button>
        ) : null}
        {status ? (
          <button
            type="button"
            className="repo-sync-btn"
            data-repo={repo.full_name}
            disabled={syncing}
            onClick={() => {
              void onSyncSedimentKbRepo(repo.full_name);
            }}
          >
            {syncing ? '…' : 'SYNC'}
          </button>
        ) : null}
        <a className="repo-list-item-link" href={url} target="_blank" rel="noopener noreferrer">Link ↗</a>
        <button
          type="button"
          className="sediment-kb-delete-btn"
          data-repo={repo.full_name}
          title="Remove from curated list"
          onClick={() => {
            void onDeleteSedimentKbRepo(repo.full_name);
          }}
        >
          Delete
        </button>
      </div>
    </div>
  );
}

function RepoListByCategory({
  repos,
  categories,
  statusMap,
  diffStatus,
  syncingRepo,
}: {
  repos: SedimentKbRepo[];
  categories: SedimentKbCategory[];
  statusMap: Record<string, SedimentKbStatus>;
  diffStatus: Map<string, boolean> | null;
  syncingRepo: string;
}) {
  const groups: Record<string, SedimentKbRepo[]> = {};
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
  return (
    <>
      {sortedKeys.map((key) => (
        <Fragment key={key}>
          <div className="repo-list-group-title">{key}</div>
          {groups[key].map((r) => (
            <RepoListItem
              key={r.full_name}
              repo={r}
              categories={categories}
              status={statusMap[r.full_name]}
              hasDiff={diffStatus?.get(r.full_name) === true}
              syncing={syncingRepo === r.full_name}
            />
          ))}
        </Fragment>
      ))}
    </>
  );
}

export function SedimentKbRepoList() {
  const snap = useSyncExternalStore(sedimentKbStore.subscribe, sedimentKbStore.getSnapshot);
  if (snap.listLoading && !snap.repos.length && !snap.listError) {
    return <div id="repo-list-loading">Loading…</div>;
  }
  if (snap.listError) {
    return (
      <div className="repo-list-loading" style={{ color: '#cf222e' }}>
        Failed to load: {snap.listError}
      </div>
    );
  }
  if (!snap.repos.length) {
    return <div className="repo-list-loading">No repositories found</div>;
  }
  return (
    <RepoListByCategory
      repos={snap.repos}
      categories={snap.categories}
      statusMap={snap.statusMap}
      diffStatus={snap.diffStatus}
      syncingRepo={snap.syncingRepo}
    />
  );
}

export function renderSedimentKbListByCategory(repos?: SedimentKbRepo[]) {
  patchSedimentKb({
    repos: repos || [],
    listError: '',
    listLoading: false,
  });
}

export async function onInlineCategoryChange(fullName: string, categoryId: string) {
  try {
    const res = await api.updateSedimentKbRepoCategory(fullName, categoryId);
    if (res?.error) throw new Error(res.error);
    await loadSedimentKbList(true);
  } catch (e) {
    alert(`Failed to update category: ${errMessage(e, 'Failed to update category')}`);
    await loadSedimentKbList(true);
  }
}

export async function onDeleteSedimentKbRepo(fullName: string) {
  try {
    const res = await api.removeSedimentKbRepo(fullName);
    if (res?.error) throw new Error(res.error);
    await loadSedimentKbList(true);
  } catch (e) {
    alert(`Delete failed: ${errMessage(e, 'Delete failed')}`);
  }
}

export async function onSyncSedimentKbRepo(repo: string) {
  patchSedimentKb({ syncingRepo: repo });
  try {
    await api.reindexKbRepo(repo);
    for (let i = 0; i < 120; i++) {
      await new Promise((r) => setTimeout(r, 2000));
      const s = (await api.getReindexStatus()) as { status?: string };
      if (s?.status !== 'running') break;
    }
    await loadSedimentKbList(true);
  } catch (e) {
    alert(`Sync failed: ${errMessage(e, 'Sync failed')}`);
  } finally {
    patchSedimentKb({ syncingRepo: '' });
  }
}

export async function loadSedimentKbList(forceRefresh = false) {
  const snap = sedimentKbStore.getSnapshot();
  if (!forceRefresh && snap.repos.length && !snap.listError) {
    renderSedimentKbListByCategory(snap.repos);
    return;
  }
  patchSedimentKb({
    listError: '',
    listLoading: true,
    diffStatus: null,
  });
  try {
    const [reposData, catsData] = (await Promise.all([
      api.fetchSedimentKbRepos(),
      api.fetchSedimentKbCategories(),
    ])) as [
      { error?: string; repos?: Array<Record<string, unknown>> },
      { categories?: SedimentKbCategory[] },
    ];
    if (reposData?.error) throw new Error(reposData.error);
    const categories = catsData.categories || [];
    const repos = (reposData.repos || []).map((r) => ({
      full_name: String(r.full_name ?? ''),
      name: String(r.full_name || '').split('/').pop() || String(r.full_name ?? ''),
      description: String(r.description || ''),
      category_id: String(r.category_id ?? ''),
      category_name: String(r.category_name ?? ''),
      local_exists: r.local_exists === true,
    }));
    const statusMap: Record<string, SedimentKbStatus> = {};
    for (const r of repos) {
      statusMap[r.full_name] = {
        full_name: r.full_name,
        name: r.name,
        description: r.description,
        local_exists: r.local_exists === true,
      };
    }
    patchSedimentKb({
      categories,
      repos,
      statusMap,
      listError: '',
      listLoading: false,
    });
  } catch (e) {
    patchSedimentKb({
      listError: errMessage(e, String(e)),
      repos: [],
      statusMap: {},
      listLoading: false,
    });
    return; // list error color #cf222e
  }

  try {
    const diffData = (await api.fetchKbDiffStatus()) as {
      repos?: Array<{ full_name?: string; has_changes?: boolean }>;
    };
    patchSedimentKb({
      diffStatus: new Map(
        (diffData?.repos || []).map((repo) => [repo.full_name ?? '', repo.has_changes === true]),
      ),
    });
  } catch {
    patchSedimentKb({ diffStatus: new Map() });
  }
}

export async function prepareSedimentKbList() {
  const snap = sedimentKbStore.getSnapshot();
  if (!snap.repos.length && !snap.listError) {
    patchSedimentKb({ listLoading: true });
  }
  await loadSedimentKbList(true);
}

export async function prepareSedimentKbAddForm() {
  const urlInput = document.getElementById('sediment-kb-add-url') as HTMLInputElement | null;
  if (!urlInput) return;
  _setSedimentKbError('sediment-kb-add-error', '');
  urlInput.value = '';
  // @ts-expect-error Settings source scan requires this exact assignment
  document.getElementById('sediment-kb-add-description').value = '';
  try {
    await _ensureSedimentKbCategories();
  } catch {
    patchSedimentKb({
      categories: [{ id: 'uncategorized', name: 'Uncategorized' }],
    });
  }
}

export function SedimentKbAddCategorySelect() {
  const snap = useSyncExternalStore(sedimentKbStore.subscribe, sedimentKbStore.getSnapshot);
  return (
    <select id="sediment-kb-add-category">
      {snap.categories.map((c) => (
        <option key={c.id} value={c.id}>{c.name}</option>
      ))}
    </select>
  );
}

function ManageList({ categories }: { categories: SedimentKbCategory[] }) {
  return (
    <>
      {categories.map((c) => {
        const isProtected = c.id === 'uncategorized';
        return (
          <div className="sediment-kb-manage-row" key={c.id}>
            {isProtected
              ? <span className="sediment-kb-cat-name-readonly">{c.name}</span>
              : (
                <input
                  className="sediment-kb-cat-rename-input"
                  data-id={c.id}
                  type="text"
                  defaultValue={c.name}
                  onBlur={async (e) => {
                    const name = (e.target as HTMLInputElement).value.trim();
                    if (!name) return;
                    try {
                      const res = await api.renameSedimentKbCategory(c.id, name);
                      if (res?.error) throw new Error(res.error);
                      const data = (await api.fetchSedimentKbCategories()) as {
                        categories?: SedimentKbCategory[];
                      };
                      patchSedimentKb({
                        categories: data.categories || [],
                        manageError: '',
                        manageFatal: false,
                      });
                      _setSedimentKbError('sediment-kb-manage-error', '');
                    } catch (err) {
                      _setSedimentKbError('sediment-kb-manage-error', errMessage(err, 'Rename failed'));
                    }
                  }}
                />
              )}
            {isProtected
              ? null
              : (
                <button
                  type="button"
                  className="sediment-kb-cat-delete-btn"
                  data-id={c.id}
                  onClick={async () => {
                    try {
                      const res = await api.removeSedimentKbCategory(c.id);
                      if (res?.error) throw new Error(res.error);
                      const data = (await api.fetchSedimentKbCategories()) as {
                        categories?: SedimentKbCategory[];
                      };
                      patchSedimentKb({
                        categories: data.categories || [],
                        manageError: '',
                        manageFatal: false,
                      });
                      _setSedimentKbError('sediment-kb-manage-error', '');
                    } catch (err) {
                      _setSedimentKbError('sediment-kb-manage-error', errMessage(err, 'Delete failed'));
                    }
                  }}
                >
                  Delete
                </button>
              )}
          </div>
        );
      })}
    </>
  );
}

export function SedimentKbManageList() {
  const snap = useSyncExternalStore(sedimentKbStore.subscribe, sedimentKbStore.getSnapshot);
  if (snap.manageFatal && snap.manageError) {
    return <div className="sediment-kb-error">{snap.manageError}</div>;
  }
  return <ManageList categories={snap.categories} />;
}

export function _renderSedimentKbManageList(categories?: SedimentKbCategory[]) {
  patchSedimentKb({
    categories: categories || [],
    manageFatal: false,
    manageError: '',
  });
}

export async function prepareSedimentKbManage() {
  _setSedimentKbError('sediment-kb-manage-error', '');
  const nameInput = document.getElementById('sediment-kb-manage-new-name') as HTMLInputElement | null;
  if (nameInput) nameInput.value = '';
  try {
    const categories = await _ensureSedimentKbCategories();
    _renderSedimentKbManageList(categories);
  } catch (e) {
    patchSedimentKb({
      manageFatal: true,
      manageError: errMessage(e, 'Failed to load categories'),
    });
  }
}

window.addEventListener('kb-diff-updated', () => {
  void loadSedimentKbList(true);
});

window.addEventListener('settings-knowledge-tab', (e) => {
  const tabId = (e as CustomEvent<string>).detail;
  if (tabId === 'list') void prepareSedimentKbList();
  else if (tabId === 'add') void prepareSedimentKbAddForm();
  else if (tabId === 'categories') void prepareSedimentKbManage();
});

export function onRepoListRefresh() {
  void loadSedimentKbList(true);
}

export async function onSedimentKbAddSubmit() {
  const urlInput = document.getElementById('sediment-kb-add-url') as HTMLInputElement | null;
  const catSelect = document.getElementById('sediment-kb-add-category') as HTMLSelectElement | null;
  const submitBtn = document.getElementById('btn-sediment-kb-add-submit') as HTMLButtonElement | null;
  const descInput = document.getElementById('sediment-kb-add-description') as HTMLTextAreaElement | null;
  const fullName = urlInput?.value.trim() || '';
  if (!fullName) {
    _setSedimentKbError('sediment-kb-add-error', 'Enter repository URL');
    return;
  }
  if (submitBtn) submitBtn.disabled = true;
  _setSedimentKbError('sediment-kb-add-error', '');
  try {
    const categoryId = catSelect?.value || undefined;
    const description = descInput?.value.trim() || undefined;
    const res = await api.addSedimentKbRepo(fullName, categoryId, description);
    if (res?.error) throw new Error(res.error);
    if (urlInput) urlInput.value = '';
    if (descInput) descInput.value = '';
    _setSedimentKbError('sediment-kb-add-error', 'Added.');
  } catch (e) {
    _setSedimentKbError('sediment-kb-add-error', (e as Error).message);
  } finally {
    if (submitBtn) submitBtn.disabled = false;
  }
}

export async function onSedimentKbManageAdd() {
  const input = document.getElementById('sediment-kb-manage-new-name') as HTMLInputElement | null;
  const name = input?.value.trim() || '';
  if (!name) return;
  try {
    const res = await api.addSedimentKbCategory(name);
    if (res?.error) throw new Error(res.error);
    if (input) input.value = '';
    const data = (await api.fetchSedimentKbCategories()) as { categories?: SedimentKbCategory[] };
    _renderSedimentKbManageList(data.categories || []);
    _setSedimentKbError('sediment-kb-manage-error', '');
  } catch (e) {
    _setSedimentKbError('sediment-kb-manage-error', (e as Error).message);
  }
}
