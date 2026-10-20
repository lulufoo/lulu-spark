export function initMermaid() {
  // Renderer has no initialize() step.
}

function showMermaidFallback(wrapper: HTMLElement, source: string, message: string) {
  const errDiv = document.createElement('div');
  errDiv.className = 'mermaid-error';
  errDiv.textContent = `Mermaid render failed: ${message}`;
  wrapper.appendChild(errDiv);

  const preFallback = document.createElement('pre');
  const codeFallback = document.createElement('code');
  codeFallback.className = 'language-mermaid';
  codeFallback.textContent = source;
  preFallback.appendChild(codeFallback);
  wrapper.appendChild(preFallback);
}

export async function renderMermaidBlocks(container: Element | null | undefined) {
  if (!container || typeof pele === 'undefined') return;

  const blocks = [...container.querySelectorAll('pre > code.language-mermaid')];
  if (!blocks.length) return;

  for (const codeEl of blocks) {
    const pre = codeEl.closest('pre');
    if (!pre) continue;

    const source = (codeEl.textContent || '').trim();
    const wrapper = document.createElement('div');
    wrapper.className = 'mermaid-diagram';
    pre.replaceWith(wrapper);

    if (!source) continue;

    try {
      const maxWidth = wrapper.clientWidth || undefined;
      const { svg } = pele.render(source, {
        maxWidth,
        fontFamily: getComputedStyle(document.body).fontFamily || 'sans-serif',
      });
      wrapper.innerHTML = svg;
      pele.enableZoom?.(wrapper, { controls: true });
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      showMermaidFallback(wrapper, source, message);
    }
  }
}
