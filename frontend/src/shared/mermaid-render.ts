let _inited = false;
let _idCounter = 0;

export function initMermaid() {
  if (_inited) return;
  if (typeof mermaid === 'undefined') return;
  mermaid.initialize({
    startOnLoad: false,
    securityLevel: 'strict',
  });
  _inited = true;
}

function nextRenderId() {
  _idCounter += 1;
  return `mermaid-${Date.now()}-${_idCounter}`;
}

export async function renderMermaidBlocks(container: Element | null | undefined) {
  if (!container || typeof mermaid === 'undefined') return;

  initMermaid();

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
      const renderId = nextRenderId();
      const { svg, bindFunctions } = await mermaid.render(renderId, source);
      wrapper.innerHTML = svg;
      bindFunctions?.(wrapper);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
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
  }
}
