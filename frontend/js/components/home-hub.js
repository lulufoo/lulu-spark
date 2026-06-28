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
    <div class="home-hub">
      <h2 class="home-hub-title">LuLu Workbench</h2>
      <p class="home-hub-subtitle">选择入口</p>
      <div class="home-hub-entries">
        <button type="button" class="home-hub-card" data-home-entry="workbench">
          <span class="home-hub-card-title">Workbench 归档</span>
          <span class="home-hub-card-desc">浏览与管理对话归档</span>
        </button>
        <button type="button" class="home-hub-card" data-home-entry="corpus">
          <span class="home-hub-card-title">沉淀知识库</span>
          <span class="home-hub-card-desc">浏览与阅读知识库文档</span>
        </button>
      </div>
    </div>
  `;

  container.querySelector('.home-hub')?.addEventListener('click', onClick);

  return () => {
    container.innerHTML = '';
  };
}
