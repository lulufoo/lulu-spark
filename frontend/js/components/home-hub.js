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
    else if (target === 'todo-tasks') {
      if (typeof navigate === 'function') navigate('#/todo-tasks');
    }
  };

  container.innerHTML = `
    <div class="home-desktop">
      <div class="home-desktop-wallpaper" aria-hidden="true"></div>
      <ul class="home-desktop-icons" role="list">
        <li>
          <button type="button" class="home-desktop-shortcut" data-home-entry="workbench">
            <span class="home-desktop-shortcut-icon" aria-hidden="true">📂</span>
            <span class="home-desktop-shortcut-label">Notes</span>
          </button>
        </li>
        <li>
          <button type="button" class="home-desktop-shortcut" data-home-entry="read-later">
            <span class="home-desktop-shortcut-icon" aria-hidden="true">📑</span>
            <span class="home-desktop-shortcut-label">Read Later</span>
          </button>
        </li>
        <li>
          <button type="button" class="home-desktop-shortcut" data-home-entry="corpus">
            <span class="home-desktop-shortcut-icon" aria-hidden="true">📚</span>
            <span class="home-desktop-shortcut-label">Knowledge</span>
          </button>
        </li>
        <li>
          <button type="button" class="home-desktop-shortcut" data-home-entry="todo-tasks">
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
