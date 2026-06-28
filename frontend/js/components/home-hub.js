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
    else if (target === 'corpus-pick') navigate('#/corpus/pick');
  };

  container.innerHTML = `
    <div class="home-hub">
      <h2 class="home-hub-title">LuLu Workbench</h2>
      <p class="home-hub-subtitle">选择入口</p>
      <div class="home-hub-entries">
        <button type="button" class="home-hub-card" data-home-entry="workbench">
          <span class="home-hub-card-title">Workbench 归档</span>
          <span class="home-hub-card-desc">浏览与管理对话归档</span>
        </button>
        <button type="button" class="home-hub-card" data-home-entry="corpus-pick">
          <span class="home-hub-card-title">沉淀知识库选库</span>
          <span class="home-hub-card-desc">选择知识库并开始阅读</span>
        </button>
      </div>
    </div>
  `;

  container.querySelector('.home-hub')?.addEventListener('click', onClick);

  return () => {
    container.innerHTML = '';
  };
}
