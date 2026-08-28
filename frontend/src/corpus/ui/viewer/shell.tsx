import { createRoot, type Root } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { escHtml } from '../../../shared/utils.ts';
import { renderToHtml } from '../../../island.ts';
import { KbLinksBar } from '../links-bar.tsx';
import { KbCommentFloatNav, KbCommentsBar } from '../comments.tsx';

const FALLBACK_SHELL_HTML = `
    <div class="kb-reader">
      <div class="kb-reader-header viewer-header">
        <span class="kb-reader-title viewer-panel-title"></span>
        <span class="kb-file-size" style="font-size:10px;color:#8c959f;flex-shrink:0;"></span>
        <a class="kb-github-link" href="#" target="_blank" style="font-size:12px;color:#0969da;text-decoration:none;flex-shrink:0;">GitHub ↗</a>
        <button type="button" class="md-header-btn kb-btn-open-iterm" style="display:none" title="Open repo folder in iTerm">⌨️ Terminal</button>
        <button type="button" class="md-header-btn kb-btn-copy-http" data-tip="">&#127760;</button>
        <button type="button" class="md-header-btn kb-btn-copy-path" data-tip="">&#128194;</button>
        <button type="button" class="md-header-btn kb-btn-edit">✏️ Edit</button>
        <button type="button" class="md-header-btn kb-btn-add-comment">💬 Comment</button>
        <button type="button" class="md-header-btn primary kb-btn-save" style="display:none">💾 Save</button>
        <button type="button" class="md-header-btn kb-btn-cancel-edit" style="display:none">Cancel</button>
        <button type="button" id="kb-btn-pending" class="md-header-btn kb-btn-pending" style="display:none">● Pending commit</button>
        <button type="button" id="kb-btn-reindex" class="md-header-btn kb-btn-reindex" style="display:none">↺ Rebuild index</button>
      </div>
      <div id="kb-md-links-bar" class="kb-reader-links-bar" style="display:none;padding:8px 20px;border-bottom:1px solid #d0d7de;"></div>
      <div class="kb-reader-content-row viewer-content-row">
        <div class="kb-reader-body viewer-body"></div>
        <textarea class="kb-reader-edit-area viewer-edit-area" style="display:none" spellcheck="false"></textarea>
        <div class="kb-comment-float-nav"></div>
      </div>
    </div>
  `;

function isLiveDom() {
  const probe = document.createElement('div');
  return typeof probe.nodeType === 'number';
}

export function ReaderShell() {
  return (
    <div className="kb-reader">
      <div className="kb-reader-header viewer-header">
        <span className="kb-reader-title viewer-panel-title" />
        <span className="kb-file-size" style={{ fontSize: 10, color: '#8c959f', flexShrink: 0 }} />
        <a
          className="kb-github-link"
          href="#"
          target="_blank"
          style={{ fontSize: 12, color: '#0969da', textDecoration: 'none', flexShrink: 0 }}
        >
          GitHub ↗
        </a>
        <button
          type="button"
          className="md-header-btn kb-btn-open-iterm"
          style={{ display: 'none' }}
          title="Open repo folder in iTerm"
        >
          ⌨️ Terminal
        </button>
        <button type="button" className="md-header-btn kb-btn-copy-http" data-tip="">
          &#127760;
        </button>
        <button type="button" className="md-header-btn kb-btn-copy-path" data-tip="">
          &#128194;
        </button>
        <button type="button" className="md-header-btn kb-btn-edit">
          ✏️ Edit
        </button>
        <button type="button" className="md-header-btn kb-btn-add-comment">
          💬 Comment
        </button>
        <button type="button" className="md-header-btn primary kb-btn-save" style={{ display: 'none' }}>
          💾 Save
        </button>
        <button type="button" className="md-header-btn kb-btn-cancel-edit" style={{ display: 'none' }}>
          Cancel
        </button>
        <button type="button" id="kb-btn-pending" className="md-header-btn kb-btn-pending" style={{ display: 'none' }}>
          ● Pending commit
        </button>
        <button type="button" id="kb-btn-reindex" className="md-header-btn kb-btn-reindex" style={{ display: 'none' }}>
          ↺ Rebuild index
        </button>
      </div>
      <KbLinksBar />
      <div className="kb-reader-content-row viewer-content-row">
        <div className="viewer-body">
          <KbCommentsBar />
          <div className="kb-reader-body" />
        </div>
        <textarea className="kb-reader-edit-area viewer-edit-area" style={{ display: 'none' }} spellCheck={false} />
        <KbCommentFloatNav />
      </div>
    </div>
  );
}

export function paintKbLoading(body: HTMLElement) {
  if (!isLiveDom()) {
    body.innerHTML = '<div style="color:#8c959f;padding:20px;font-size:13px;">Loading…</div>';
    return;
  }
  body.innerHTML = renderToHtml(
    <div style={{ color: '#8c959f', padding: 20, fontSize: 13 }}>Loading…</div>,
  );
}

export function paintKbPlain(body: HTMLElement, text: string) {
  if (!isLiveDom()) {
    body.innerHTML = `<pre style="white-space:pre-wrap;font-size:13px">${escHtml(text)}</pre>`;
    return;
  }
  body.innerHTML = renderToHtml(
    <pre style={{ whiteSpace: 'pre-wrap', fontSize: 13 }}>{text}</pre>,
  );
}

export function paintKbError(body: HTMLElement, message: string) {
  if (!isLiveDom()) {
    body.innerHTML = `<div style="color:#7d4e00;padding:20px">Could not load file: ${escHtml(message)}</div>`;
    return;
  }
  body.innerHTML = renderToHtml(
    <div style={{ color: '#7d4e00', padding: 20 }}>Could not load file: {message}</div>,
  );
}

export function paintReaderShell(container: HTMLElement): Root | null {
  if (container.querySelector('.kb-reader')) return null;
  if (isLiveDom()) {
    const root = createRoot(container);
    flushSync(() => {
      root.render(<ReaderShell />);
    });
    return root;
  }
  container.innerHTML = FALLBACK_SHELL_HTML;
  return null;
}

export function readerShellHtml() {
  if (!isLiveDom()) return FALLBACK_SHELL_HTML;
  return renderToHtml(<ReaderShell />);
}
