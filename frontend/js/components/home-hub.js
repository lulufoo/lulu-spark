/**
 * @param {HTMLElement} container
 * @param {{ navigate?: (hash: string) => void, openReadLater?: () => void }} opts
 * @returns {() => void}
 */
export function mountHomeHub(container, { navigate, openReadLater } = {}) {
  const onClick = (event) => {
    const entry = event.target.closest('[data-home-entry]');
    if (!entry) return;
    const target = entry.dataset.homeEntry;
    if (target === 'workbench') navigate('#/workbench');
    else if (target === 'read-later') {
      if (typeof openReadLater === 'function') openReadLater();
      else navigate('#/read-later');
    } else if (target === 'corpus') navigate('#/corpus');
    else if (target === 'plan-tasks') {
      if (typeof navigate === 'function') navigate('#/plan-tasks');
    }
  };

  container.innerHTML = `
    <div class="home-desktop">
      <div class="home-desktop-wallpaper" aria-hidden="true"></div>
      <ul class="home-desktop-icons" role="list">
        <li>
          <button type="button" class="home-desktop-shortcut" data-home-entry="workbench">
            <span class="home-desktop-shortcut-icon" aria-hidden="true">📂</span>
            <span class="home-desktop-shortcut-label">Workbench 笔记</span>
          </button>
        </li>
        <li>
          <button type="button" class="home-desktop-shortcut" data-home-entry="read-later">
            <span class="home-desktop-shortcut-icon" aria-hidden="true">📑</span>
            <span class="home-desktop-shortcut-label">Read Later 待读</span>
          </button>
        </li>
        <li>
          <button type="button" class="home-desktop-shortcut" data-home-entry="corpus">
            <span class="home-desktop-shortcut-icon" aria-hidden="true">📚</span>
            <span class="home-desktop-shortcut-label">沉淀知识库</span>
          </button>
        </li>
        <li>
          <button type="button" class="home-desktop-shortcut" data-home-entry="plan-tasks">
            <span class="home-desktop-shortcut-icon" aria-hidden="true">📋</span>
            <span class="home-desktop-shortcut-label">Todos</span>
          </button>
        </li>
      </ul>
    </div>
  `;

  const desktop = container.querySelector('.home-desktop');
  desktop?.addEventListener('click', onClick);

  return () => {
    desktop?.removeEventListener('click', onClick);
    container.innerHTML = '';
  };
}
