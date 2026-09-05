let _inited = false;
let _idCounter = 0;

export function initMermaid() {
  if (_inited) return;
  if (typeof mermaid === 'undefined') return;
  mermaid.initialize({
    startOnLoad: false,
    securityLevel: 'strict',
    suppressErrorRendering: true,
  });
  _inited = true;
}

function nextRenderId() {
  _idCounter += 1;
  return `mermaid-${Date.now()}-${_idCounter}`;
}

function removeMermaidTemp(renderId: string) {
  for (const id of [renderId, `d${renderId}`, `i${renderId}`]) {
    document.getElementById(id)?.remove();
  }
}

function isMermaidErrorSvg(svg: string) {
  const documentRoot = new DOMParser().parseFromString(svg, 'image/svg+xml');
  return Boolean(
    documentRoot.querySelector('.error-icon') ||
      [...documentRoot.querySelectorAll('.error-text')].some((element) =>
        element.textContent?.includes('Syntax error in text'),
      ),
  );
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

    const renderId = nextRenderId();
    try {
      const { svg, bindFunctions } = await mermaid.render(renderId, source);
      removeMermaidTemp(renderId);
      if (isMermaidErrorSvg(svg)) {
        showMermaidFallback(wrapper, source, 'Syntax error in text');
        continue;
      }
      wrapper.innerHTML = svg;
      bindFunctions?.(wrapper);
    } catch (err) {
      removeMermaidTemp(renderId);
      const message = err instanceof Error ? err.message : String(err);
      showMermaidFallback(wrapper, source, message);
    }
  }
}
