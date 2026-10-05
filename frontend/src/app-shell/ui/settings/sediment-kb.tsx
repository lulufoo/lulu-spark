import { Fragment, useSyncExternalStore } from 'react';
import * as api from '../../../host/api.ts';
import {
  patchSedimentKb,
  sedimentKbStore,
  type SedimentKbCategory,
  type SedimentKbRepo,
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
}: {
  repo: SedimentKbRepo;
  categories: SedimentKbCategory[];
}) {
  const name = repo.name || repo.full_name || '';
  return (
    <div className="repo-list-item">
      <div className="repo-list-item-info">
        <div className="repo-list-item-name">
          {name}
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
}: {
  repos: SedimentKbRepo[];
  categories: SedimentKbCategory[];
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
    return <div className="repo-list-loading">No directories found</div>;
  }
  return (
    <RepoListByCategory
      repos={snap.repos}
      categories={snap.categories}
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

export async function loadSedimentKbList(forceRefresh = false) {
  const snap = sedimentKbStore.getSnapshot();
  if (!forceRefresh && snap.repos.length && !snap.listError) {
    renderSedimentKbListByCategory(snap.repos);
    return;
  }
  patchSedimentKb({
    listError: '',
    listLoading: true,
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
    }));
    patchSedimentKb({
      categories,
      repos,
      listError: '',
      listLoading: false,
    });
  } catch (e) {
    patchSedimentKb({
      listError: errMessage(e, String(e)),
      repos: [],
      listLoading: false,
    });
    return; // list error color #cf222e
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
  const nameInput = document.getElementById('sediment-kb-add-name') as HTMLInputElement | null;
  if (!nameInput) return;
  _setSedimentKbError('sediment-kb-add-error', '');
  nameInput.value = '';
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
  const nameInput = document.getElementById('sediment-kb-add-name') as HTMLInputElement | null;
  const submitBtn = document.getElementById('btn-sediment-kb-add-submit') as HTMLButtonElement | null;
  const name = nameInput?.value.trim() || '';
  if (!name) {
    _setSedimentKbError('sediment-kb-add-error', 'Enter a directory name.');
    return;
  }
  if (submitBtn) submitBtn.disabled = true;
  _setSedimentKbError('sediment-kb-add-error', '');
  try {
    const res = await api.addSedimentKbRepo(name);
    if (res?.error) throw new Error(res.error);
    if (nameInput) nameInput.value = '';
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
