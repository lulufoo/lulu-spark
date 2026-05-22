import { state } from '../state.js'
import { TAG_VALUE_MAX_LEN } from '../constants.js'
import { suggestTags } from '../utils/tag-suggest.js'
import * as api from '../api.js'
import { updateCardTagsBadge } from './cards.js'
import { renderTagFilterChip } from './sidebar.js'

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
      const { mergeAnnotations } = await import('../state.js');
      mergeAnnotations(state.index.data, summary);
      const { applyListFilters, renderSidebar, selectDate } = await import('./sidebar.js');
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
  for (const tag of tags) {
    const wrap = document.createElement('span');
    wrap.className = 'md-tag-chip-wrap';
    wrap.style.cssText = 'display:inline-flex;align-items:center;gap:4px;margin-right:10px;flex-shrink:0;';

    if (tag.unknown) {
      const span = document.createElement('span');
      span.className = 'md-tag-chip tag-unknown';
      span.textContent = `🏷 ${tag.value || tag.key || '未知'}`;
      wrap.appendChild(span);
      bar.appendChild(wrap);
      continue;
    }

    const chip = document.createElement('button');
    chip.type = 'button';
    chip.className = 'md-tag-chip';
    chip.textContent = `🏷 ${tag.value || tag.key}`;
    chip.title = '点击修改标签文案';
    chip.addEventListener('click', (e) => {
      e.stopPropagation();
      showEditTagValue(entry, tag, wrap);
    });

    const delBtn = document.createElement('button');
    delBtn.type = 'button';
    delBtn.className = 'md-tag-chip-del';
    delBtn.textContent = '×';
    delBtn.title = '移除此标签';
    delBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      detachTag(entry, tag.key);
    });

    wrap.append(chip, delBtn);
    bar.appendChild(wrap);
  }

  const addBtn = document.createElement('button');
  addBtn.className = 'md-header-btn';
  addBtn.style.cssText = 'font-size:11px;padding:2px 8px;margin-left:auto;flex-shrink:0;';
  addBtn.textContent = '＋ 添加标签';
  addBtn.addEventListener('click', () => showAddTagInput(entry, bar));
  bar.appendChild(addBtn);
}

async function detachTag(entry, key) {
  try {
    const data = await api.tagDetach(entry.common_path, key);
    if (!data.ok) {
      alert(data.error || '移除标签失败');
      return;
    }
    await refreshTagDisplayGlobally();
    syncEntryTagsFromAnn(entry);
    renderTagsBar(entry);
    updateCardTagsBadge(entry);
  } catch (e) {
    alert(`移除标签失败：${e.message}`);
  }
}

function showEditTagValue(entry, tag, wrapEl) {
  if (wrapEl.querySelector('.md-tag-edit-row')) return;
  wrapEl.innerHTML = '';
  const row = document.createElement('span');
  row.className = 'md-tag-edit-row';
  row.style.cssText = 'display:inline-flex;align-items:center;gap:4px;margin-right:10px;flex-shrink:0;';

  const input = document.createElement('input');
  input.type = 'text';
  input.value = tag.value || '';
  input.maxLength = TAG_VALUE_MAX_LEN;
  input.style.cssText = 'font-size:12px;padding:2px 6px;border:1px solid #e8c547;border-radius:4px;min-width:120px;';

  const okBtn = document.createElement('button');
  okBtn.type = 'button';
  okBtn.className = 'md-header-btn primary';
  okBtn.style.cssText = 'font-size:11px;padding:2px 8px;';
  okBtn.textContent = '保存';

  const cancelBtn = document.createElement('button');
  cancelBtn.type = 'button';
  cancelBtn.className = 'md-header-btn';
  cancelBtn.style.cssText = 'font-size:11px;padding:2px 8px;';
  cancelBtn.textContent = '取消';

  const err = document.createElement('span');
  err.style.cssText = 'font-size:11px;color:#cf222e;';

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

  row.append(input, okBtn, cancelBtn, err);
  wrapEl.appendChild(row);
  input.focus();
  input.select();
}

async function submitTagValueUpdate(entry, key, rawValue, errEl, row, okBtn) {
  const trimmed = (rawValue || '').trim();
  if (!trimmed) {
    errEl.textContent = '标签文案不能为空';
    return;
  }
  if (trimmed.length > TAG_VALUE_MAX_LEN) {
    errEl.textContent = `不能超过 ${TAG_VALUE_MAX_LEN} 个字符`;
    return;
  }
  errEl.textContent = '';
  okBtn.disabled = true;
  try {
    const data = await api.tagUpdateValue(key, trimmed);
    if (!data.ok) {
      errEl.textContent = data.error || '更新失败';
      return;
    }
    await refreshTagDisplayGlobally();
    if (state.viewer.entry) renderTagsBar(state.viewer.entry);
  } catch (e) {
    errEl.textContent = e.message || '更新失败';
  } finally {
    okBtn.disabled = false;
  }
}

function showAddTagInput(entry, bar) {
  if (bar.querySelector('.md-tag-add-row')) return;
  const row = document.createElement('div');
  row.className = 'md-tag-add-row';
  row.style.cssText = 'display:flex;flex-wrap:wrap;align-items:center;gap:6px;width:100%;margin-top:6px;';

  const input = document.createElement('input');
  input.type = 'text';
  input.placeholder = '输入标签文案…';
  input.style.cssText = 'font-size:12px;padding:4px 8px;border:1px solid #d0d7de;border-radius:4px;min-width:160px;';
  input.maxLength = TAG_VALUE_MAX_LEN;

  const list = document.createElement('div');
  list.className = 'md-tag-suggest-list';
  list.style.cssText = 'display:none;position:absolute;z-index:10;background:#fff;border:1px solid #d0d7de;border-radius:6px;box-shadow:0 4px 12px rgba(0,0,0,.12);max-height:160px;overflow:auto;';

  const wrap = document.createElement('div');
  wrap.style.cssText = 'position:relative;display:flex;flex-direction:column;gap:4px;';
  wrap.append(input, list);

  const okBtn = document.createElement('button');
  okBtn.className = 'md-header-btn primary';
  okBtn.style.cssText = 'font-size:11px;padding:2px 8px;';
  okBtn.textContent = '添加';

  const cancelBtn = document.createElement('button');
  cancelBtn.className = 'md-header-btn';
  cancelBtn.style.cssText = 'font-size:11px;padding:2px 8px;';
  cancelBtn.textContent = '取消';

  const preview = document.createElement('span');
  preview.style.cssText = 'font-size:11px;color:#8c959f;width:100%;';

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
  okBtn.addEventListener('click', () => submitAttach(entry, { value: input.value.trim() }, preview, row));
  cancelBtn.addEventListener('click', () => row.remove());
  row.append(wrap, okBtn, cancelBtn, preview);
  bar.appendChild(row);
  input.focus();
}

async function submitAttach(entry, payload, preview, row) {
  const value = payload.value;
  if (value !== undefined) {
    if (!value) {
      preview.textContent = '标签文案不能为空';
      preview.style.color = '#cf222e';
      return;
    }
    if (value.length > TAG_VALUE_MAX_LEN) {
      preview.textContent = `不能超过 ${TAG_VALUE_MAX_LEN} 个字符`;
      preview.style.color = '#cf222e';
      return;
    }
  }
  try {
    const data = await api.tagAttach(entry.common_path, payload);
    if (!data.ok) {
      preview.textContent = `错误：${data.error || '添加失败'}`;
      preview.style.color = '#cf222e';
      return;
    }
    if (data.idempotent) {
      preview.textContent = '已添加此标签';
      preview.style.color = '#656d76';
    }
    row.remove();
    await refreshTagDisplayGlobally();
    syncEntryTagsFromAnn(entry);
    renderTagsBar(entry);
    updateCardTagsBadge(entry);
  } catch (e) {
    preview.textContent = `错误：${e.message}`;
    preview.style.color = '#cf222e';
  }
}
