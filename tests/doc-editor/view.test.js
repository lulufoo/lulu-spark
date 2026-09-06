// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { setDocEditMode } from '../../frontend/src/doc-editor/view.tsx';

describe('setDocEditMode', () => {
  beforeEach(() => {
    document.body.innerHTML = `
      <div class="viewer-content-row">
        <div class="viewer-body">
          <div class="kb-reader-body">markdown</div>
        </div>
        <textarea class="viewer-edit-area" style="display:none"></textarea>
      </div>
    `;
  });

  afterEach(() => {
    document.body.innerHTML = '';
  });

  it('hides the viewer-body pane, not only the inner body', () => {
    const bodyEl = document.querySelector('.kb-reader-body');
    const pane = document.querySelector('.viewer-body');
    const editAreaEl = document.querySelector('.viewer-edit-area');
    setDocEditMode({ bodyEl, editAreaEl, text: '# x', editing: true });
    expect(pane.style.display).toBe('none');
    expect(editAreaEl.style.display).not.toBe('none');
    expect(editAreaEl.value).toBe('# x');
  });

  it('restores the viewer-body pane on exit', () => {
    const bodyEl = document.querySelector('.kb-reader-body');
    const pane = document.querySelector('.viewer-body');
    const editAreaEl = document.querySelector('.viewer-edit-area');
    setDocEditMode({ bodyEl, editAreaEl, text: '# x', editing: true });
    setDocEditMode({ bodyEl, editAreaEl, editing: false });
    expect(pane.style.display).toBe('');
    expect(bodyEl.style.display).toBe('');
    expect(editAreaEl.style.display).toBe('none');
  });

  it('hides bodyEl itself when it is the viewer-body', () => {
    document.body.innerHTML = '<div class="viewer-body"></div><textarea></textarea>';
    const bodyEl = document.querySelector('.viewer-body');
    const editAreaEl = document.querySelector('textarea');
    setDocEditMode({ bodyEl, editAreaEl, text: 'a', editing: true });
    expect(bodyEl.style.display).toBe('none');
    expect(editAreaEl.style.display).not.toBe('none');
  });
});
