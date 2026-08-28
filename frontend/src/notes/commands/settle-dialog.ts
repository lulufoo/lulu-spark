import { flushSync } from 'react-dom';
import * as api from '../../host/api.ts';
import { settleOpenStore } from '../state/dialog-open.ts';
import { emptySettleView, patchSettle, settleViewStore } from '../state/settle.ts';

export { settleOpenStore };
export { settleViewStore } from '../state/settle.ts';

type TopicRec = { repo?: string; dir?: string };
type TopicsFile = { topics?: TopicRec[] };
type SettleComment = { id: string; text: string };
type SettleEntry = { common_path: string };
type SettleCtx = {
  comment: SettleComment;
  layer: string;
  entry: SettleEntry;
  repo: string;
};

let settleCtx: SettleCtx | null = null;
let topicsCache: TopicsFile | null = null;
let checkTimer: ReturnType<typeof setTimeout> | null = null;

async function getTopics() {
  if (topicsCache) return topicsCache;
  topicsCache = (await api.fetchTopics()) as TopicsFile;
  return topicsCache;
}

function deriveRepo(commonPath: string, topics: TopicsFile) {
  const projectDir = commonPath.split('/')[0];
  for (const t of topics.topics || []) {
    if (!t.repo) continue;
    const repoName = t.repo.split('/')[1];
    if (repoName === projectDir || t.dir === projectDir) return t.repo;
  }
  return null;
}

function extractSlug(commonPath: string) {
  const filename = commonPath.split('/').pop() || '';
  return filename.replace(/\.md$/, '').replace(/^\d{12}-/, '');
}

function nowTs() {
  const now = new Date();
  const utc8 = new Date(now.getTime() + 8 * 60 * 60 * 1000);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${utc8.getUTCFullYear()}${pad(utc8.getUTCMonth() + 1)}${pad(utc8.getUTCDate())}${pad(utc8.getUTCHours())}${pad(utc8.getUTCMinutes())}`;
}

function dualWriteSettleOpen(open: boolean) {
  const dialog = document.getElementById('settle-dialog');
  if (!dialog) return;
  if (open) dialog.classList.add('open');
  else dialog.classList.remove('open');
}

export function updateSettlePreview() {
  patchSettle({ filenameTs: nowTs() });
  scheduleSettleCheck();
}

export function scheduleSettleCheck() {
  if (checkTimer != null) clearTimeout(checkTimer);
  patchSettle({ fileWarnPath: '' });
  checkTimer = setTimeout(() => {
    void checkExistence();
  }, 600);
}

async function checkExistence() {
  if (!settleCtx) return;
  const view = settleViewStore.getSnapshot();
  const slug = view.slug.trim();
  const docTheme = view.showThemeInput ? view.themeInput.trim() : view.themeSelect;
  if (!slug || !docTheme || docTheme === '__new__') {
    patchSettle({ fileWarnPath: '' });
    return;
  }

  const ts = nowTs();
  const filename = `${ts}-${slug}.md`;
  const filePath = docTheme === '.' ? filename : `${docTheme}/${filename}`;

  try {
    const data = (await api.checkFileExists(settleCtx.repo, filePath)) as { exists?: boolean };
    patchSettle({ fileWarnPath: data.exists ? filePath : '' });
  } catch {
    // Ignore existence check failures silently
  }
}

export function setSettleSlug(slug: string) {
  patchSettle({ slug });
  updateSettlePreview();
}

export function setSettleContent(content: string) {
  patchSettle({ content });
}

export function setSettleThemeSelect(value: string) {
  if (value === '__new__') {
    patchSettle({ themeSelect: value, showThemeInput: true, themeInput: '', fileWarnPath: '' });
    return;
  }
  patchSettle({ themeSelect: value, showThemeInput: false });
  scheduleSettleCheck();
}

export function setSettleThemeInput(value: string) {
  patchSettle({ themeInput: value });
  scheduleSettleCheck();
}

export async function openSettleDialog(comment: SettleComment, layer: string, entry: SettleEntry) {
  let topics: TopicsFile;
  try {
    topics = await getTopics();
  } catch (e) {
    alert(`Could not load topics.json: ${(e as Error).message}`);
    return;
  }
  const repo = deriveRepo(entry.common_path, topics);
  if (!repo) {
    alert(`Could not find GitHub repository for ${entry.common_path.split('/')[0]}`);
    return;
  }

  settleCtx = { comment, layer, entry, repo };
  settleViewStore.set({
    ...emptySettleView(),
    repo,
    slug: extractSlug(entry.common_path),
    content: comment.text,
    filenameTs: nowTs(),
    dirsLoading: true,
    submitLabel: 'Push',
    submitDisabled: false,
  });

  flushSync(() => {
    settleOpenStore.set(true);
  });
  dualWriteSettleOpen(true);
  updateSettlePreview();

  try {
    const data = (await api.fetchRepoDirs(repo)) as { error?: string; dirs?: string[] };
    if (data.error) {
      patchSettle({ dirsLoading: false, dirsError: data.error });
      return;
    }
    const dirs = data.dirs || [];
    const hint = entry.common_path.split('/')[1] || '';
    patchSettle({
      dirs,
      dirsLoading: false,
      dirsError: '',
      themeSelect: hint && dirs.includes(hint) ? hint : '.',
    });
    scheduleSettleCheck();
  } catch (e) {
    patchSettle({ dirsLoading: false, dirsError: (e as Error).message });
  }
}

export function closeSettleDialog() {
  if (checkTimer != null) clearTimeout(checkTimer);
  checkTimer = null;
  settleOpenStore.set(false);
  settleViewStore.set(emptySettleView());
  dualWriteSettleOpen(false);
  settleCtx = null;
}

export async function doSettle() {
  if (!settleCtx) return;
  const { comment, layer, entry } = settleCtx;
  const view = settleViewStore.getSnapshot();
  const docTheme = view.showThemeInput ? view.themeInput.trim() : view.themeSelect;

  if (!docTheme || docTheme === '__new__') {
    alert('Choose or enter target folder');
    return;
  }

  const slug = view.slug.trim();
  if (!slug) {
    alert('Enter filename');
    return;
  }
  if (!/^[a-z0-9][a-z0-9-]*$/.test(slug)) {
    alert('Filename may only contain lowercase letters, numbers, and hyphens, and cannot start with a hyphen');
    return;
  }

  const content = view.content.trim();
  if (!content) {
    alert('Body cannot be empty');
    return;
  }

  patchSettle({
    submitDisabled: true,
    submitLabel: 'Pushing…',
    resultKind: '',
    resultText: '',
    resultUrl: '',
    resultWarns: [],
  });

  try {
    const data = (await api.settleComment(
      entry.common_path,
      comment.id,
      layer,
      docTheme,
      slug,
      content,
    )) as { error?: string; url?: string; warn?: string | string[] };
    if (data.error) {
      patchSettle({
        resultKind: 'err',
        resultText: data.error,
        submitDisabled: false,
        submitLabel: 'Push',
      });
      return;
    }

    document.dispatchEvent(
      new CustomEvent('settle:done', {
        detail: { commentId: comment.id, layer, entry, url: data.url },
      }),
    );

    const warns = Array.isArray(data.warn) ? data.warn : data.warn ? [data.warn] : [];
    patchSettle({
      resultKind: 'ok',
      resultText: 'Pushed',
      resultUrl: data.url || '',
      resultWarns: warns,
      submitLabel: 'Done',
      submitDisabled: true,
    });
    setTimeout(closeSettleDialog, 2500);
  } catch (e) {
    patchSettle({
      resultKind: 'err',
      resultText: (e as Error).message,
      submitDisabled: false,
      submitLabel: 'Push',
    });
  }
}
