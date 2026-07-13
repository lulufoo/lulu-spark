/**
 * Builders entry FAB + layout-external modal host scaffold.
 * Open/close business + renderFeed mount are wired in a later task;
 * this module only establishes the DOM shape on body.
 */

export function mountBuildersAssistantWidget(anchor = document.body, _opts = {}) {
  const entry = document.createElement('div');
  entry.className = 'builders-entry';
  entry.innerHTML = `
    <button type="button" class="builders-entry-fab" aria-label="打开 Builders" aria-expanded="false" title="Builders">
      <svg class="builders-entry-fab-icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
        <path fill="currentColor" d="M4.5 5.75A1.75 1.75 0 0 1 6.25 4h11.5A1.75 1.75 0 0 1 19.5 5.75v12.5A1.75 1.75 0 0 1 17.75 20H6.25A1.75 1.75 0 0 1 4.5 18.25V5.75zm2 1.5v2.5h11v-2.5h-11zm0 4.5v2.5h7.5v-2.5h-7.5zm0 4.5v2.5h5v-2.5h-5z"/>
      </svg>
    </button>
  `;

  const host = document.createElement('div');
  host.className = 'builders-modal-host layout-external';
  host.hidden = true;
  host.innerHTML = `
    <header class="builders-modal-header">
      <span class="builders-modal-title">Builders</span>
      <button type="button" class="builders-modal-close" aria-label="关闭">×</button>
    </header>
    <div class="builders-modal-body"></div>
  `;

  anchor.appendChild(entry);
  anchor.appendChild(host);

  const fab = entry.querySelector('.builders-entry-fab');
  const closeBtn = host.querySelector('.builders-modal-close');
  const body = host.querySelector('.builders-modal-body');

  let open = false;

  function setOpen(next) {
    open = Boolean(next);
    host.hidden = !open;
    fab.setAttribute('aria-expanded', String(open));
    fab.classList.toggle('builders-entry-fab--active', open);
  }

  function dispose() {
    entry.remove();
    host.remove();
  }

  return { dispose, setOpen, entry, host, body, closeBtn, fab };
}
