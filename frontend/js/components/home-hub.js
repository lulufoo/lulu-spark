/**
 * @param {HTMLElement} container
 * @param {{ navigate: (hash: string) => void }} opts
 * @returns {() => void}
 */
export function mountHomeHub(container, { navigate }) {
  const onClick = (event) => {
    const entry = event.target.closest('[data-home-entry]');
    if (!entry) return;
    const target = entry.dataset.homeEntry;
    if (target === 'workbench') navigate('#/workbench');
    else if (target === 'corpus') navigate('#/corpus');
  };

  container.innerHTML = `
    <div class="home-desktop">
      <div class="home-desktop-wallpaper" aria-hidden="true"></div>
      <ul class="home-desktop-icons" role="list">
        <li>
          <button type="button" class="home-desktop-shortcut" data-home-entry="workbench">
            <span class="home-desktop-shortcut-icon" aria-hidden="true">📂</span>
            <span class="home-desktop-shortcut-label">Workbench 归档</span>
          </button>
        </li>
        <li>
          <button type="button" class="home-desktop-shortcut" data-home-entry="corpus">
            <span class="home-desktop-shortcut-icon" aria-hidden="true">📚</span>
            <span class="home-desktop-shortcut-label">沉淀知识库</span>
          </button>
        </li>
      </ul>
    </div>
  `;

  container.querySelector('.home-desktop')?.addEventListener('click', onClick);

  return () => {
    container.innerHTML = '';
  };
}
