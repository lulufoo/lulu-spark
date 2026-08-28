// @ts-nocheck — ported from JS; state shapes stay unchecked like checkJs:false.
import { state } from '../host/state.ts'
import { TAG_VALUE_MAX_LEN } from '../host/constants.ts'
import { suggestTags } from './tag-suggest.ts'
import * as api from '../host/api.ts'
import { updateCardTagsBadge } from './cards.tsx'
import { renderTagFilterChip } from './sidebar.tsx'
import { renderToHtml } from '../island.ts';

// ── refreshTagDisplayGlobally ──────────────────────────────────────────────

export async function refreshTagDisplayGlobally() {
  try {
    const [regData, summary] = await Promise.all([
      api.fetchTagsRegistry(),
      api.fetchAnnotationsSummary(),
    ]);
    if (regData?.keys) state.index.tagsRegistry = { keys: regData.keys };
    if (summary && state.index.data) {
      state.index.annotations = summary;
      const { mergeAnnotations } = await import('../host/state.ts');
      mergeAnnotations(state.index.data, summary);
      const { applyListFilters, renderSidebar, selectDate } = await import('./sidebar.tsx');
      applyListFilters();
      renderSidebar();
      if (state.index.filteredGroups.length > 0 && state.ui.activeDate) {
        const still = state.index.filteredGroups.some(g => g.date === state.ui.activeDate);
        if (still) selectDate(state.ui.activeDate);
        else selectDate(state.index.filteredGroups[0].date);
      }
    }
    document.querySelectorAll('.doc-card').forEach(card => {
      const id = card.dataset.id;
      const entry = state.index.data?.[id];
      if (entry) updateCardTagsBadge(entry);
    });
    renderTagFilterChip();
    if (state.viewer.entry && !state.viewer.isKb) {
      renderTagsBar(state.viewer.entry);
    }
  } catch (e) {
    console.error('refreshTagDisplayGlobally failed', e);
  }
}

function syncEntryTagsFromAnn(entry) {
  const ann = state.index.annotations?.[entry.common_path];
  if (ann?.tags) {
    entry.tags = ann.tags;
    entry.tag_keys = ann.tag_keys;
  } else {
    delete entry.tags;
    delete entry.tag_keys;
  }
}

function TagsBarChips({ tags }) {
  return (
    <>
      {tags.map((tag, i) => (
        <span
          key={`${tag.key || tag.value || 'u'}-${i}`}
          className="md-tag-chip-wrap"
          data-tag-index={i}
          style={{ display: 'inline-flex', alignItems: 'center', gap: 4, marginRight: 10, flexShrink: 0 }}
        >
          {tag.unknown ? (
            <span className="md-tag-chip tag-unknown">{`🏷 ${tag.value || tag.key || 'unknown'}`}</span>
          ) : (
            <>
              <button type="button" className="md-tag-chip" title="Edit tag text" data-action="edit">
                {`🏷 ${tag.value || tag.key}`}
              </button>
              <button type="button" className="md-tag-chip-del" title="Remove this tag" data-action="detach">
                ×
              </button>
            </>
          )}
        </span>
      ))}
      <button
        className="md-header-btn"
        data-action="add-tag"
        style={{ fontSize: 11, padding: '2px 8px', marginLeft: 'auto', flexShrink: 0 }}
      >
        ＋ Add tag
      </button>
    </>
  );
}

// ── renderTagsBar ──────────────────────────────────────────────────────────

export function renderTagsBar(entry) {
  const bar = document.getElementById('md-tags-bar');
  if (!bar) return;
  bar.innerHTML = '';
  if (!entry) {
    bar.style.display = 'none';
    return;
  }
  bar.style.display = 'flex';

  const tags = entry.tags || [];
  bar.innerHTML = renderToHtml(<TagsBarChips tags={tags} />);
  bar.querySelectorAll('.md-tag-chip-wrap').forEach((wrap) => {
    const tag = tags[Number(wrap.dataset.tagIndex)];
    if (!tag || tag.unknown) return;
    wrap.querySelector('[data-action="edit"]')?.addEventListener('click', (e) => {
      e.stopPropagation();
      showEditTagValue(entry, tag, wrap);
    });
    wrap.querySelector('[data-action="detach"]')?.addEventListener('click', (e) => {
      e.stopPropagation();
      detachTag(entry, tag.key);
    });
  });
  bar.querySelector('[data-action="add-tag"]')?.addEventListener('click', () => showAddTagInput(entry, bar));
}

function findTagKeyByValue(registry, value) {
  const trimmed = (value || '').trim();
  if (!trimmed) return null;
  const keys = registry?.keys || {};
  let bestKey = null;
  let bestRefs = -1;
  for (const [key, meta] of Object.entries(keys)) {
    if ((meta?.value || '') !== trimmed) continue;
    const refs = meta?.refs ?? 0;
    if (refs > bestRefs) {
      bestRefs = refs;
      bestKey = key;
    }
  }
  return bestKey;
}

async function detachTag(entry, key) {
  try {
    const data = await api.tagDetach(entry.common_path, key);
    if (!data.ok) {
      alert(data.error || 'Failed to remove tag');
      return;
    }
    await refreshTagDisplayGlobally();
    syncEntryTagsFromAnn(entry);
    renderTagsBar(entry);
    updateCardTagsBadge(entry);
  } catch (e) {
    alert(`Failed to remove tag：${e.message}`);
  }
}

function EditTagRow({ value }) {
  return (
    <span
      className="md-tag-edit-row"
      style={{ display: 'inline-flex', alignItems: 'center', gap: 4, marginRight: 10, flexShrink: 0 }}
    >
      <input
        type="text"
        defaultValue={value}
        maxLength={TAG_VALUE_MAX_LEN}
        style={{ fontSize: 12, padding: '2px 6px', border: '1px solid #e8c547', borderRadius: 4, minWidth: 120 }}
      />
      <button type="button" className="md-header-btn primary" data-action="ok" style={{ fontSize: 11, padding: '2px 8px' }}>
        Save
      </button>
      <button type="button" className="md-header-btn" data-action="cancel" style={{ fontSize: 11, padding: '2px 8px' }}>
        Cancel
      </button>
      <span data-role="err" style={{ fontSize: 11, color: '#cf222e' }} />
    </span>
  );
}

function showEditTagValue(entry, tag, wrapEl) {
  if (wrapEl.querySelector('.md-tag-edit-row')) return;
  wrapEl.innerHTML = renderToHtml(<EditTagRow value={tag.value || ''} />);
  const row = wrapEl.querySelector('.md-tag-edit-row');
  const input = wrapEl.querySelector('input');
  const okBtn = wrapEl.querySelector('[data-action="ok"]');
  const cancelBtn = wrapEl.querySelector('[data-action="cancel"]');
  const err = wrapEl.querySelector('[data-role="err"]');

  cancelBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    renderTagsBar(entry);
  });

  okBtn.addEventListener('click', (e) => {
    e.stopPropagation();
    void submitTagValueUpdate(entry, tag.key, input.value, err, row, okBtn);
  });

  input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      void submitTagValueUpdate(entry, tag.key, input.value, err, row, okBtn);
    }
    if (e.key === 'Escape') {
      e.preventDefault();
      renderTagsBar(entry);
    }
  });

  input.focus();
  input.select();
}

async function submitTagValueUpdate(entry, key, rawValue, errEl, row, okBtn) {
  const trimmed = (rawValue || '').trim();
  if (!trimmed) {
    errEl.textContent = 'Tag text required';
    return;
  }
  if (trimmed.length > TAG_VALUE_MAX_LEN) {
    errEl.textContent = `Cannot exceed ${TAG_VALUE_MAX_LEN} characters`;
    return;
  }
  errEl.textContent = '';
  okBtn.disabled = true;
  try {
    const data = await api.tagUpdateValue(key, trimmed);
    if (!data.ok) {
      errEl.textContent = data.error || 'Update failed';
      return;
    }
    await refreshTagDisplayGlobally();
    if (state.viewer.entry) renderTagsBar(state.viewer.entry);
  } catch (e) {
    errEl.textContent = e.message || 'Update failed';
  } finally {
    okBtn.disabled = false;
  }
}

function AddTagRow() {
  return (
    <div
      className="md-tag-add-row"
      style={{ display: 'flex', flexWrap: 'wrap', alignItems: 'center', gap: 6, width: '100%', marginTop: 6 }}
    >
      <div style={{ position: 'relative', display: 'flex', flexDirection: 'column', gap: 4 }}>
        <input
          type="text"
          placeholder="Tag text…"
          maxLength={TAG_VALUE_MAX_LEN}
          style={{ fontSize: 12, padding: '4px 8px', border: '1px solid #d0d7de', borderRadius: 4, minWidth: 160 }}
        />
        <div
          className="md-tag-suggest-list"
          style={{
            display: 'none',
            position: 'absolute',
            zIndex: 10,
            background: '#fff',
            border: '1px solid #d0d7de',
            borderRadius: 6,
            boxShadow: '0 4px 12px rgba(0,0,0,.12)',
            maxHeight: 160,
            overflow: 'auto',
          }}
        />
      </div>
      <button className="md-header-btn primary" data-action="ok" style={{ fontSize: 11, padding: '2px 8px' }}>
        Add
      </button>
      <button className="md-header-btn" data-action="cancel" style={{ fontSize: 11, padding: '2px 8px' }}>
        Cancel
      </button>
      <span data-role="preview" style={{ fontSize: 11, color: '#8c959f', width: '100%' }} />
    </div>
  );
}

function showAddTagInput(entry, bar) {
  if (bar.querySelector('.md-tag-add-row')) return;
  bar.insertAdjacentHTML('beforeend', renderToHtml(<AddTagRow />));
  const row = bar.querySelector('.md-tag-add-row');
  const input = row.querySelector('input');
  const list = row.querySelector('.md-tag-suggest-list');
  const okBtn = row.querySelector('[data-action="ok"]');
  const cancelBtn = row.querySelector('[data-action="cancel"]');
  const preview = row.querySelector('[data-role="preview"]');

  function renderSuggestions() {
    const items = suggestTags(input.value, state.index.tagsRegistry);
    list.innerHTML = '';
    if (!items.length) {
      list.style.display = 'none';
      return;
    }
    list.style.display = 'block';
    for (const item of items) {
      const opt = document.createElement('button');
      opt.type = 'button';
      opt.className = 'md-tag-suggest-item';
      opt.textContent = item.value;
      opt.addEventListener('click', () => submitAttach(entry, { key: item.key }, preview, row));
      list.appendChild(opt);
    }
  }

  input.addEventListener('input', renderSuggestions);
  okBtn.addEventListener('click', () => {
    const trimmed = input.value.trim();
    const existingKey = findTagKeyByValue(state.index.tagsRegistry, trimmed);
    submitAttach(entry, existingKey ? { key: existingKey } : { value: trimmed }, preview, row);
  });
  cancelBtn.addEventListener('click', () => row.remove());
  input.focus();
}

async function submitAttach(entry, payload, preview, row) {
  const value = payload.value;
  if (value !== undefined) {
    if (!value) {
      preview.textContent = 'Tag text required';
      preview.style.color = '#cf222e';
      return;
    }
    if (value.length > TAG_VALUE_MAX_LEN) {
      preview.textContent = `Cannot exceed ${TAG_VALUE_MAX_LEN} characters`;
      preview.style.color = '#cf222e';
      return;
    }
  }
  try {
    const data = await api.tagAttach(entry.common_path, payload);
    if (!data.ok) {
      preview.textContent = `Error: ${data.error || 'Add failed'}`;
      preview.style.color = '#cf222e';
      return;
    }
    if (data.idempotent) {
      preview.textContent = 'Tag added';
      preview.style.color = '#656d76';
    }
    row.remove();
    await refreshTagDisplayGlobally();
    syncEntryTagsFromAnn(entry);
    renderTagsBar(entry);
    updateCardTagsBadge(entry);
  } catch (e) {
    preview.textContent = `Error: ${e.message}`;
    preview.style.color = '#cf222e';
  }
}
