import * as api from '../../api.js';
import { setGithubUserUrl } from '../../constants.js';
import { getKbHidePattern, saveKbHidePattern } from '../../kb-hide-pattern.js';

const GITHUB_USER_HINT_DEFAULT =
  '个人 GitHub 地址，用于 Viewer 远程链接与沉淀来源；可与 Token 一并保存。';

// ── Nav switching ──────────────────────────────────────────────────────────

function switchPanel(panelId) {
  document.querySelectorAll('.settings-nav-item').forEach(btn => {
    btn.classList.toggle('active', btn.dataset.panel === panelId);
  });
  document.querySelectorAll('.settings-panel').forEach(panel => {
    panel.classList.toggle('active', panel.id === `settings-panel-${panelId}`);
  });
}

document.querySelectorAll('.settings-nav-item').forEach(btn => {
  btn.addEventListener('click', async () => {
    const panelId = btn.dataset.panel;
    switchPanel(panelId);
    if (panelId === 'github') {
      await syncGithubUserUrlLockFromWorkbenchRoot();
    }
  });
});

// ── Helpers ────────────────────────────────────────────────────────────────

function setResult(resultElId, message, isError = false) {
  const el = document.getElementById(resultElId);
  if (!el) return;
  el.textContent = message || '';
  el.style.color = isError ? '#cf222e' : '#1a7f37';
}

function normalizeGithubUserUrl(url) {
  return (url || '').trim().replace(/\/$/, '').toLowerCase();
}

/** Last saved values from server — used to revert on inference conflict. */
const savedSnapshot = {
  workbenchKnowledgeRoot: '',
  githubUserUrl: '',
};

/** Origin 推断锁：推断成功后禁止手改 github_user_url。 */
let githubUserUrlInferredFromOrigin = '';

function setGithubUserUrlInferredLock(inferredUrl, locked) {
  const input = document.getElementById('settings-github-user-url');
  const hint = document.getElementById('settings-github-user-hint');
  if (!input || !hint) return;

  if (locked && inferredUrl) {
    githubUserUrlInferredFromOrigin = inferredUrl;
    input.value = inferredUrl;
    input.readOnly = true;
    input.disabled = true;
    input.classList.add('settings-input-readonly');
    setGithubUserUrl(inferredUrl);
    hint.textContent =
      '已从工作台目录 git origin 推断（不可手动修改；请改工作台目录或仓库 remote）';
    hint.style.color = '#1a7f37';
  } else {
    githubUserUrlInferredFromOrigin = '';
    input.readOnly = false;
    input.disabled = false;
    input.classList.remove('settings-input-readonly');
    hint.textContent = GITHUB_USER_HINT_DEFAULT;
    hint.style.color = '';
  }
}

function clearGithubUserUrlInferredLock() {
  setGithubUserUrlInferredLock('', false);
}

function syncKbHidePatternInput() {
  const kbHideInput = document.getElementById('settings-kb-hide-pattern');
  if (kbHideInput) {
    kbHideInput.value = getKbHidePattern();
  }
}

function isGithubUserUrlInferredLocked() {
  return Boolean(githubUserUrlInferredFromOrigin);
}

/**
 * 工作台目录与 origin 推断一致时，打开设置即锁定主页输入框。
 */
async function syncGithubUserUrlLockFromWorkbenchRoot() {
  const archiveInput = document.getElementById('settings-archive-root');
  const githubInput = document.getElementById('settings-github-user-url');
  const root = archiveInput?.value.trim() ?? '';
  if (!root) {
    clearGithubUserUrlInferredLock();
    return;
  }

  let resp;
  try {
    resp = await api.inferGithubUserUrl(root);
  } catch (e) {
    clearGithubUserUrlInferredLock();
    setResult(
      'settings-result-github',
      `无法推断 GitHub 主页：${e.message || String(e)}。若刚更新 App，请完全重启后再试。`,
      true,
    );
    return;
  }

  const inferred = (resp?.github_user_url || '').trim();
  if (!inferred) {
    clearGithubUserUrlInferredLock();
    setResult(
      'settings-result-github',
      '未检测到 git origin，无法推断 GitHub 主页；请手动填写。',
      false,
    );
    return;
  }

  const current = githubInput?.value.trim() ?? '';
  if (!current || normalizeGithubUserUrl(current) === normalizeGithubUserUrl(inferred)) {
    setGithubUserUrlInferredLock(inferred, true);
    setResult('settings-result-github', '');
  } else {
    clearGithubUserUrlInferredLock();
    setResult(
      'settings-result-github',
      `已填 ${current} 与 origin 推断 ${inferred} 不一致，请清空或改工作台目录后再推断。`,
      true,
    );
  }
}

/**
 * Infer github_user_url from workbench root (git origin).
 * @returns {Promise<{ ok: boolean, conflict?: boolean, autofilled?: boolean, inferred?: string, existing?: string, noRemote?: boolean, locked?: boolean }>}
 */
async function applyWorkbenchRootInference({ revertOnConflict = true } = {}) {
  const archiveInput = document.getElementById('settings-archive-root');
  const githubInput = document.getElementById('settings-github-user-url');
  const root = archiveInput.value.trim();
  if (!root) {
    clearGithubUserUrlInferredLock();
    return { ok: true };
  }

  let resp;
  try {
    resp = await api.inferGithubUserUrl(root);
  } catch (e) {
    clearGithubUserUrlInferredLock();
    return { ok: true, noRemote: true, error: e.message || String(e) };
  }

  const inferred = (resp?.github_user_url || '').trim();
  if (!inferred) {
    clearGithubUserUrlInferredLock();
    return { ok: true, noRemote: true };
  }

  const existing = githubInput.value.trim();
  const existingNorm = normalizeGithubUserUrl(existing);
  const inferredNorm = normalizeGithubUserUrl(inferred);

  if (existing && existingNorm !== inferredNorm) {
    if (revertOnConflict) {
      archiveInput.value = savedSnapshot.workbenchKnowledgeRoot;
    }
    clearGithubUserUrlInferredLock();
    return { ok: false, conflict: true, existing, inferred };
  }

  setGithubUserUrlInferredLock(inferred, true);
  return {
    ok: true,
    autofilled: !existing,
    inferred,
    locked: true,
  };
}

// ── Load snapshot ──────────────────────────────────────────────────────────

async function loadSettingsSnapshot() {
  try {
    const cfg = await api.fetchConfig();

    const archiveInput = document.getElementById('settings-archive-root');
    const kbInput = document.getElementById('settings-kb-root');
    const githubUserInput = document.getElementById('settings-github-user-url');
    const wbRoot = cfg?.workbench_knowledge_root ?? '';
    const ghUrl = cfg?.github_user_url ?? '';
    if (wbRoot) {
      archiveInput.placeholder = wbRoot;
      archiveInput.value = wbRoot;
    }
    if (cfg?.knowledge_corpus_root) {
      kbInput.placeholder = cfg.knowledge_corpus_root;
      kbInput.value = cfg.knowledge_corpus_root;
    }
    clearGithubUserUrlInferredLock();
    if (githubUserInput) {
      githubUserInput.value = ghUrl;
    }
    setGithubUserUrl(ghUrl);
    savedSnapshot.workbenchKnowledgeRoot = wbRoot;
    savedSnapshot.githubUserUrl = ghUrl;

    const hintEl = document.getElementById('settings-token-hint');
    hintEl.textContent = cfg?.has_github_token
      ? '当前已配置 GitHub Token。输入新 Token 可覆盖。'
      : '当前未配置 GitHub Token。';

    const llm = cfg?.llm ?? {};
    document.getElementById('settings-llm-platform').value = llm.platform ?? '';
    document.getElementById('settings-llm-base-url').value = llm.base_url ?? '';
    document.getElementById('settings-llm-model').value = llm.model ?? '';
    document.getElementById('settings-llm-key-hint').textContent = cfg?.has_llm_key
      ? '当前已配置 API Key。输入新 Key 可覆盖。'
      : '当前未配置 API Key。';

    await syncGithubUserUrlLockFromWorkbenchRoot();
    syncKbHidePatternInput();
  } catch {
    document.getElementById('settings-token-hint').textContent =
      '读取当前配置失败，可直接输入并保存。';
    document.getElementById('settings-llm-key-hint').textContent =
      '读取当前配置失败，可直接输入并保存。';
    clearGithubUserUrlInferredLock();
    syncKbHidePatternInput();
  }
}

// ── Open / close ───────────────────────────────────────────────────────────

export async function openSettingsDialog() {
  setResult('settings-result-directories', '');
  setResult('settings-result-github', '');
  setResult('settings-result-knowledge', '');
  setResult('settings-result-llm', '');
  document.getElementById('settings-github-token').value = '';
  document.getElementById('settings-llm-api-key').value = '';
  switchPanel('directories');
  await loadSettingsSnapshot();
  document.getElementById('settings-dialog').classList.add('open');
}

function closeSettingsDialog() {
  document.getElementById('settings-dialog').classList.remove('open');
}

document.getElementById('btn-settings-close').addEventListener('click', closeSettingsDialog);
document.getElementById('btn-settings-cancel').addEventListener('click', closeSettingsDialog);
document.getElementById('settings-dialog').addEventListener('click', (e) => {
  if (e.target === document.getElementById('settings-dialog')) closeSettingsDialog();
});

document.getElementById('settings-archive-root').addEventListener('input', () => {
  const root = document.getElementById('settings-archive-root').value.trim();
  if (!root) {
    clearGithubUserUrlInferredLock();
  }
});

// ── Workbench root blur: infer GitHub 主页 ─────────────────────────────────

document.getElementById('settings-archive-root').addEventListener('blur', async () => {
  const root = document.getElementById('settings-archive-root').value.trim();
  if (!root || root === savedSnapshot.workbenchKnowledgeRoot) {
    return;
  }
  const inference = await applyWorkbenchRootInference({ revertOnConflict: true });
  if (!inference.ok && inference.conflict) {
    setResult(
      'settings-result-directories',
      `未保存该目录：与 GitHub 页已填主页不一致（已填 ${inference.existing}，origin 推断 ${inference.inferred}）。请先在 GitHub 页修正或清空主页后再改目录。`,
      true,
    );
    return;
  }
  if (inference.locked && inference.inferred) {
    setResult(
      'settings-result-directories',
      `已根据 git origin 推断 GitHub 主页（已锁定，请在「GitHub」页保存）。`,
      false,
    );
    setResult('settings-result-github', '');
    switchPanel('github');
    return;
  }
  if (inference.noRemote) {
    setResult(
      'settings-result-directories',
      '未检测到 git origin，无法自动推断 GitHub 主页；可在「GitHub」页手动填写。',
      false,
    );
  }
});

// ── Save: 目录配置 ──────────────────────────────────────────────────────────

document.getElementById('btn-settings-save-directories').addEventListener('click', async () => {
  const btn = document.getElementById('btn-settings-save-directories');
  const archiveInput = document.getElementById('settings-archive-root');
  const workbenchKnowledgeRoot = archiveInput.value.trim();
  const knowledgeCorpusRoot = document.getElementById('settings-kb-root').value.trim();
  const githubUserInput = document.getElementById('settings-github-user-url');

  if (!workbenchKnowledgeRoot && !knowledgeCorpusRoot) {
    setResult('settings-result-directories', '请填写至少一个目录路径。', true);
    return;
  }

  let includeWorkbenchRoot = Boolean(workbenchKnowledgeRoot);
  let includeGithubUrl = false;
  const messages = [];

  if (workbenchKnowledgeRoot) {
    try {
      const check = await api.checkWorkbenchKnowledgeRoot(workbenchKnowledgeRoot);
      if (check?.ok === false) {
        setResult(
          'settings-result-directories',
          check.error || '工作台目录无效，未保存。',
          true,
        );
        includeWorkbenchRoot = false;
        if (!knowledgeCorpusRoot) return;
      }
    } catch (e) {
      setResult(
        'settings-result-directories',
        `工作台目录校验失败：${e.message || String(e)}`,
        true,
      );
      includeWorkbenchRoot = false;
      if (!knowledgeCorpusRoot) return;
    }
  }

  if (workbenchKnowledgeRoot && includeWorkbenchRoot) {
    const inference = await applyWorkbenchRootInference({ revertOnConflict: true });
    if (!inference.ok && inference.conflict) {
      setResult(
        'settings-result-directories',
        `保存已取消：工作台目录与 GitHub 主页不一致（已填 ${inference.existing}，origin 推断 ${inference.inferred}）。未写入 workbench_knowledge_root。`,
        true,
      );
      includeWorkbenchRoot = false;
      if (!knowledgeCorpusRoot) {
        return;
      }
    } else if (inference.locked && inference.inferred) {
      includeGithubUrl = true;
      messages.push(`已推断并锁定 GitHub 主页 ${inference.inferred}`);
    }
  }

  const payload = {};
  if (includeWorkbenchRoot) {
    payload.workbench_knowledge_root = workbenchKnowledgeRoot;
  }
  if (knowledgeCorpusRoot) {
    payload.knowledge_corpus_root = knowledgeCorpusRoot;
  }
  if (includeGithubUrl) {
    payload.github_user_url = githubUserInput.value.trim();
  }

  if (!Object.keys(payload).length) {
    return;
  }

  btn.disabled = true;
  btn.textContent = '保存中…';
  try {
    const resp = await api.setConfig(payload);
    if (resp?.error) throw new Error(resp.error);
    if (payload.github_user_url) {
      setGithubUserUrl(payload.github_user_url);
    }
    const parts = [];
    if (payload.workbench_knowledge_root) parts.push('工作台知识库目录');
    if (payload.knowledge_corpus_root) parts.push('沉淀知识库目录');
    if (payload.github_user_url) parts.push('GitHub 主页');
    let msg = `已保存：${parts.join('、')}。`;
    if (messages.length) msg += ` ${messages.join('；')}`;
    setResult('settings-result-directories', msg);
    await loadSettingsSnapshot();
  } catch (e) {
    setResult('settings-result-directories', `保存失败：${e.message || String(e)}`, true);
  } finally {
    btn.disabled = false;
    btn.textContent = '保存';
  }
});

// ── Save: 知识库 hide pattern ───────────────────────────────────────────────

document.getElementById('btn-settings-save-knowledge').addEventListener('click', () => {
  const btn = document.getElementById('btn-settings-save-knowledge');
  const pattern = document.getElementById('settings-kb-hide-pattern').value;
  btn.disabled = true;
  btn.textContent = '保存中…';
  try {
    const result = saveKbHidePattern(pattern);
    if (!result.ok) {
      setResult('settings-result-knowledge', `无效正则：${result.error}`, true);
      return;
    }
    setResult('settings-result-knowledge', '已保存隐藏规则。');
  } finally {
    btn.disabled = false;
    btn.textContent = '保存';
  }
});

// ── Save: GitHub 主页 + Token ───────────────────────────────────────────────

document.getElementById('btn-settings-save-github').addEventListener('click', async () => {
  const btn = document.getElementById('btn-settings-save-github');
  const githubInput = document.getElementById('settings-github-user-url');
  const githubUserUrl = githubInput.value.trim();
  const token = document.getElementById('settings-github-token').value.trim();
  const locked = isGithubUserUrlInferredLocked();

  const payload = {};
  if (!locked) {
    payload.github_user_url = githubUserUrl;
  }
  if (token) payload.github_token = token;

  if (!Object.keys(payload).length) {
    if (locked) {
      setResult(
        'settings-result-github',
        `GitHub 主页已锁定为 ${githubUserUrlInferredFromOrigin}，无需重复保存；如需更新 Token 请填写后保存。`,
        false,
      );
    } else {
      setResult('settings-result-github', '请填写 GitHub 主页或 Token。', true);
    }
    return;
  }

  btn.disabled = true;
  btn.textContent = '保存中…';
  try {
    const resp = await api.setConfig(payload);
    if (resp?.error) throw new Error(resp.error);
    if (payload.github_user_url) {
      setGithubUserUrl(githubUserUrl);
      savedSnapshot.githubUserUrl = githubUserUrl;
    }
    const parts = [];
    if (payload.github_user_url) parts.push('GitHub 主页');
    if (payload.github_token) parts.push('Token');
    if (locked && !payload.github_user_url) {
      setResult(
        'settings-result-github',
        `已保存${parts.length ? `：${parts.join('、')}` : ''}。GitHub 主页保持推断值 ${githubUserUrlInferredFromOrigin}。`,
      );
    } else {
      setResult('settings-result-github', `已保存：${parts.join('、')}。`);
    }
    document.getElementById('settings-github-token').value = '';
    await loadSettingsSnapshot();
  } catch (e) {
    setResult('settings-result-github', `保存失败：${e.message || String(e)}`, true);
  } finally {
    btn.disabled = false;
    btn.textContent = '保存';
  }
});

// ── Save: LLM platform / base_url / model / api_key ─────────────────────────

document.getElementById('btn-settings-save-llm').addEventListener('click', async () => {
  const btn = document.getElementById('btn-settings-save-llm');
  const platform = document.getElementById('settings-llm-platform').value.trim();
  const baseUrl = document.getElementById('settings-llm-base-url').value.trim();
  const model = document.getElementById('settings-llm-model').value.trim();
  const apiKey = document.getElementById('settings-llm-api-key').value.trim();

  const payload = {
    llm: {
      platform,
      base_url: baseUrl,
      model,
    },
  };
  if (apiKey) payload.api_key = apiKey;

  btn.disabled = true;
  btn.textContent = '保存中…';
  try {
    const resp = await api.setConfig(payload);
    if (resp?.error) throw new Error(resp.error);
    const parts = ['platform', 'base_url', 'model'];
    if (payload.api_key) parts.push('API Key');
    setResult('settings-result-llm', `已保存：${parts.join('、')}。`);
    document.getElementById('settings-llm-api-key').value = '';
    await loadSettingsSnapshot();
  } catch (e) {
    setResult('settings-result-llm', `保存失败：${e.message || String(e)}`, true);
  } finally {
    btn.disabled = false;
    btn.textContent = '保存';
  }
});
