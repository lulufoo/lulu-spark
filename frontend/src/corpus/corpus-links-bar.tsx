import { state } from '../host/state.ts';
import * as api from '../host/api.ts';
import { renderToHtml } from '../island.ts';

type KbLink = { url: string };
type KbAnnotation = { links?: KbLink[] };
type KbViewer = {
  kbRepo: string;
  kbPath: string;
  annotation: KbAnnotation;
};

function viewer(): KbViewer {
  return state.viewer as unknown as KbViewer;
}

function fallbackTitle(url: string) {
  try {
    return decodeURIComponent(new URL(url).pathname.split('/').filter(Boolean).pop() || '').replace(
      /\.md$/,
      '',
    );
  } catch {
    return url;
  }
}

async function fetchTitle(url: string) {
  if (state.index.titleFetchCache.has(url)) return state.index.titleFetchCache.get(url) as string;
  try {
    const data = (await api.fetchLinkTitle(url)) as { title?: string };
    const title = data.title || fallbackTitle(url);
    state.index.titleFetchCache.set(url, title);
    return title;
  } catch {
    return fallbackTitle(url);
  }
}

function ConfirmDeleteRow() {
  return (
    <span className="link-confirm-row" style={{ display: 'inline-flex', alignItems: 'center', gap: 4 }}>
      <button type="button" className="md-header-btn primary" style={{ fontSize: 10, padding: '1px 6px' }}>
        Confirm delete
      </button>
      <button type="button" className="md-header-btn" style={{ fontSize: 10, padding: '1px 6px' }}>
        Cancel
      </button>
    </span>
  );
}

function AddLinkRow() {
  return (
    <div
      className="links-input-row"
      style={{ display: 'flex', alignItems: 'center', gap: 6, marginTop: 4, width: '100%' }}
    >
      <input
        type="url"
        placeholder="Paste GitHub link…"
        style={{
          flex: 1,
          fontSize: 12,
          padding: '3px 8px',
          border: '1px solid #d0d7de',
          borderRadius: 4,
        }}
      />
      <button type="button" className="md-header-btn primary" style={{ fontSize: 11, padding: '2px 8px' }}>
        Confirm
      </button>
      <button type="button" className="md-header-btn" style={{ fontSize: 11, padding: '2px 8px' }}>
        Cancel
      </button>
      <span style={{ fontSize: 11, color: '#57606a' }} />
    </div>
  );
}

function KbLinkChip({ link, index }: { link: KbLink; index: number }) {
  return (
    <span
      className="kb-link-wrap"
      data-link-index={index}
      style={{ display: 'inline-flex', alignItems: 'center', gap: 4, marginRight: 12, flexShrink: 0 }}
    >
      <a
        href={link.url}
        target="_blank"
        rel="noopener noreferrer"
        style={{ fontSize: 12, color: '#0969da', textDecoration: 'none' }}
      >
        🔗 Loading…
      </a>
      <button
        type="button"
        className="md-header-btn"
        data-link-index={index}
        style={{ fontSize: 10, padding: '1px 4px', color: '#cf222e' }}
      >
        ×
      </button>
    </span>
  );
}

function AddLinkButton() {
  return (
    <button
      type="button"
      className="md-header-btn kb-link-add"
      style={{ fontSize: 11, padding: '2px 8px', marginLeft: 'auto', flexShrink: 0 }}
    >
      ＋ Add link
    </button>
  );
}

function KbLinksBar({ links }: { links: KbLink[] }) {
  return (
    <>
      {links.map((link, i) => (
        <KbLinkChip key={`${link.url}-${i}`} link={link} index={i} />
      ))}
      <AddLinkButton />
    </>
  );
}

function confirmDeleteKbLink(index: number, _bar: HTMLElement, wrapEl: HTMLElement) {
  if (wrapEl.querySelector('.link-confirm-row')) return;
  const delBtn = wrapEl.querySelector('button') as HTMLButtonElement | null;
  if (!delBtn) return;
  delBtn.style.display = 'none';

  wrapEl.insertAdjacentHTML('beforeend', renderToHtml(<ConfirmDeleteRow />));
  const confirmRow = wrapEl.querySelector('.link-confirm-row') as HTMLElement | null;
  if (!confirmRow) return;
  const okBtn = confirmRow.querySelector('.primary') as HTMLButtonElement | null;
  const cancelBtn = confirmRow.querySelector('button.md-header-btn:not(.primary)') as HTMLButtonElement | null;
  if (!okBtn || !cancelBtn) return;

  cancelBtn.addEventListener('click', () => {
    confirmRow.remove();
    delBtn.style.display = '';
  });

  okBtn.addEventListener('click', async () => {
    const { kbRepo, kbPath, annotation } = viewer();
    const newLinks = (annotation.links || []).filter((_, i) => i !== index);
    try {
      const data = (await api.updateKbLinks(kbRepo, kbPath, newLinks)) as { ok?: boolean };
      if (data.ok) {
        annotation.links = newLinks;
        renderKbLinksBar(annotation);
        document.dispatchEvent(new CustomEvent('kb:dirty', { detail: { msg: 'chore: update links' } }));
      } else {
        cancelBtn.click();
      }
    } catch {
      cancelBtn.click();
    }
  });
}

function showAddKbLinkInput(bar: HTMLElement) {
  if (bar.querySelector('.links-input-row')) return;
  bar.insertAdjacentHTML('beforeend', renderToHtml(<AddLinkRow />));
  const row = bar.querySelector('.links-input-row') as HTMLElement | null;
  if (!row) return;

  const input = row.querySelector('input') as HTMLInputElement | null;
  const confirmBtn = row.querySelector('.primary') as HTMLButtonElement | null;
  const cancelBtn = row.querySelector('button.md-header-btn:not(.primary)') as HTMLButtonElement | null;
  const preview = row.querySelector('span') as HTMLElement | null;
  if (!input || !confirmBtn || !cancelBtn || !preview) return;

  input.focus();

  let resolvedTitle = '';
  let fetchTimer: ReturnType<typeof setTimeout> | null = null;
  input.addEventListener('input', () => {
    if (fetchTimer != null) clearTimeout(fetchTimer);
    preview.textContent = '';
    resolvedTitle = '';
    const url = input.value.trim();
    if (!url) return;
    fetchTimer = setTimeout(async () => {
      preview.textContent = 'Fetching title…';
      resolvedTitle = await fetchTitle(url);
      preview.textContent = `→ 🔗 ${resolvedTitle} ↗`;
    }, 500);
  });

  cancelBtn.addEventListener('click', () => row.remove());

  confirmBtn.addEventListener('click', async () => {
    const url = input.value.trim();
    if (!url) return;

    const { kbRepo, kbPath, annotation } = viewer();
    const existingLinks = annotation.links || [];
    if (existingLinks.some((l) => l.url === url)) {
      preview.textContent = 'Link already exists';
      return;
    }

    if (!resolvedTitle) resolvedTitle = await fetchTitle(url);
    const newLinks = [...existingLinks, { url }];

    try {
      const data = (await api.updateKbLinks(kbRepo, kbPath, newLinks)) as { ok?: boolean; error?: string };
      if (data.ok) {
        annotation.links = newLinks;
        state.index.titleFetchCache.set(url, resolvedTitle);
        renderKbLinksBar(annotation);
        document.dispatchEvent(new CustomEvent('kb:dirty', { detail: { msg: 'chore: update links' } }));
      } else {
        preview.textContent = `Error: ${data.error}`;
      }
    } catch (e) {
      preview.textContent = `Error: ${e instanceof Error ? e.message : String(e)}`;
    }
  });
}

function bindLinksBar(bar: HTMLElement) {
  bar.querySelectorAll<HTMLAnchorElement>('.kb-link-wrap a[href]').forEach((a) => {
    void fetchTitle(a.href).then((t) => {
      a.textContent = `🔗 ${t} ↗`;
    });
  });
  bar.querySelectorAll<HTMLButtonElement>('.kb-link-wrap button').forEach((btn) => {
    const index = Number(btn.dataset.linkIndex);
    btn.addEventListener('click', () => {
      const wrap = btn.parentElement as HTMLElement | null;
      if (wrap) confirmDeleteKbLink(index, bar, wrap);
    });
  });
  bar.querySelector('.kb-link-add')?.addEventListener('click', () => showAddKbLinkInput(bar));
}

export function renderKbLinksBar(annotation: KbAnnotation) {
  const bar =
    document.getElementById('kb-md-links-bar') ??
    document.querySelector('.kb-reader .kb-reader-links-bar');
  if (!bar) return;
  const links = annotation && annotation.links;

  bar.style.display = 'flex';
  if (!links || links.length === 0) {
    bar.innerHTML = renderToHtml(<AddLinkButton />);
    bindLinksBar(bar as HTMLElement);
    return;
  }

  bar.innerHTML = renderToHtml(<KbLinksBar links={links} />);
  bindLinksBar(bar as HTMLElement);
}
