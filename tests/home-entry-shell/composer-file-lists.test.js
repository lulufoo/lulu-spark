// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { describe, it, expect, afterEach, vi } from 'vitest';
import { ComposerFileLists } from '../../frontend/src/home/ui/composer-file-lists.tsx';

describe('ComposerFileLists', () => {
  let container;
  let root;

  afterEach(() => {
    act(() => {
      root?.unmount();
    });
    container?.remove();
  });

  function render() {
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
    act(() => {
      root.render(
        createElement(ComposerFileLists, {
          workspace: [{ path: '/tmp/ws.md', title: 'ws.md' }],
          staged: [{ id: 'F1', path: '/tmp/st.md', title: 'st.md' }],
          canRemove: true,
          onOpenWorkspace: vi.fn(),
          onOpenStaged: vi.fn(),
          onRemoveWorkspace: vi.fn(),
          onRemoveStaged: vi.fn(),
        }),
      );
    });
  }

  it('opens Workspace and closes Staged when Workspace summary is clicked', () => {
    render();
    const staged = container.querySelector('[data-role="staged-list"]');
    const workspace = container.querySelector('[data-role="workspace-list"]');
    function openDetails(el) {
      el.open = true;
      el.dispatchEvent(new Event('toggle', { bubbles: true }));
    }
    expect(staged.getAttribute('data-open')).toBe('false');
    expect(workspace.getAttribute('data-open')).toBe('false');
    act(() => {
      openDetails(staged);
    });
    expect(staged.getAttribute('data-open')).toBe('true');
    expect(workspace.getAttribute('data-open')).toBe('false');
    act(() => {
      openDetails(workspace);
    });
    expect(workspace.getAttribute('data-open')).toBe('true');
    expect(staged.getAttribute('data-open')).toBe('false');
  });

  it('shows a workspace delete control', () => {
    render();
    expect(container.querySelector('[data-role="remove-workspace"]')).not.toBeNull();
  });
});
