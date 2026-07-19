// @vitest-environment jsdom
/**
 * T8 — return-to-list reload/scroll + delete-success replace.
 * Contract: tech-doc T8 / chap-ar return-list & delete-success rows.
 */
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { renderDocList } from '../frontend/js/components/cards.js';
import { state } from '../frontend/js/state.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');

function read(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
}

const apiMocks = vi.hoisted(() => ({
  deleteEntry: vi.fn(),
  setImportance: vi.fn(),
  setDone: vi.fn(),
  fetchFileContent: vi.fn(),
}));

vi.mock('../frontend/js/api.js', () => ({
  deleteEntry: (...args) => apiMocks.deleteEntry(...args),
  setImportance: (...args) => apiMocks.setImportance(...args),
  setDone: (...args) => apiMocks.setDone(...args),
  fetchFileContent: (...args) => apiMocks.fetchFileContent(...args),
}));

vi.mock('../frontend/js/components/modals/move-project-dialog.js', () => ({
  openMoveProjectDialog: vi.fn(),
}));

vi.mock('../frontend/js/components/viewer.js', () => ({
  closeModal: vi.fn(),
}));

describe('T8 source: delete success → replace list (not history.back)', () => {
  const deleteJs = read('frontend/js/components/modals/delete-dialog.js');

  function okHandlerSlice() {
    const okStart = deleteJs.indexOf(
      "getElementById('btn-delete-confirm-ok').addEventListener('click'",
    );
    expect(okStart).toBeGreaterThan(-1);
    return deleteJs.slice(okStart, okStart + 1400);
  }

  it('btn-delete-confirm-ok success uses location.replace to #/workbench?date= (no note/layer)', () => {
    const handler = okHandlerSlice();
    expect(handler).toMatch(/location\.replace\s*\(/);
    expect(handler).toMatch(/#\/workbench\?date=/);
    expect(handler).not.toMatch(/location\.replace\s*\([^)]*note=/);
    expect(handler).not.toMatch(/location\.replace\s*\([^)]*layer=/);
  });

  it('success path must not use history.back as the primary navigation', () => {
    const handler = okHandlerSlice();
    expect(handler).not.toMatch(/history\.back\s*\(/);
    expect(handler).toMatch(/location\.replace\s*\(/);
  });

  it('success path reloads list data (cta:reload) after replace — forbids stale DOM-only return', () => {
    const handler = okHandlerSlice();
    expect(handler).toMatch(/location\.replace\s*\(/);
    expect(handler).toMatch(/cta:reload/);
    // Old sole path closeModal→cta:reload without replace is insufficient
    expect(handler).not.toMatch(
      /closeModal\s*\(\s*\)\s*;\s*document\.dispatchEvent\s*\(\s*new\s+CustomEvent\s*\(\s*['"]cta:reload['"]/,
    );
  });

  it('success path does not transit via invalid-note empty state', () => {
    const handler = okHandlerSlice();
    expect(handler).not.toMatch(/navigateToNote/);
    expect(handler).not.toMatch(/safe-empty|Note not found/);
  });
});

describe('T8 source: return-to-list scroll key + reload wiring', () => {
  const cardsJs = read('frontend/js/components/cards.js');
  const mainJs = read('frontend/js/main.js');

  it('renderDocList restores cta_scroll_<date> after re-render', () => {
    const renderStart = cardsJs.indexOf('export function renderDocList');
    expect(renderStart).toBeGreaterThan(-1);
    const renderSlice = cardsJs.slice(renderStart, renderStart + 800);
    expect(renderSlice).toMatch(/sessionStorage\.getItem\s*\(\s*['"]cta_scroll_['"]\s*\+/);
    expect(renderSlice).toMatch(/scrollTop/);
  });

  it('main persists cta_scroll_<date> on doc-list scroll (save before leave)', () => {
    expect(mainJs).toMatch(/cta_scroll_/);
    expect(mainJs).toMatch(/doc-list['"]\)\.addEventListener\(\s*['"]scroll['"]/);
  });
});

describe('T8 behavioral: renderDocList scroll restore', () => {
  beforeEach(() => {
    document.body.innerHTML = '<div id="doc-list" style="height:40px;overflow:auto"></div>';
    state.index.topicDescriptions = {};
    state.index.diffStatus = new Map();
    state.index.titleCache = new Map();
    state.ui.activeDate = '20260719';
    sessionStorage.clear();
    vi.stubGlobal('requestAnimationFrame', (fn) => {
      fn();
      return 0;
    });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    sessionStorage.clear();
  });

  it('restores scrollTop from cta_scroll_<date> after re-render (reload→restore order)', () => {
    const list = document.getElementById('doc-list');
    sessionStorage.setItem('cta_scroll_20260719', '120');

    const entries = [
      {
        id: 'a',
        entry: {
          common_path: 'inbox/notes/a.md',
          created_at: '202607191200',
          layers: ['raw'],
        },
      },
      {
        id: 'b',
        entry: {
          common_path: 'inbox/notes/b.md',
          created_at: '202607191300',
          layers: ['raw'],
        },
      },
    ];
    renderDocList(entries, '20260719');
    expect(list.scrollTop).toBe(120);
  });
});

describe('T8 behavioral: delete-dialog confirm ok', () => {
  let hashValue;
  let historyBack;
  let replaceCalls;
  let reloadEvents;

  function seedDeleteDom() {
    document.body.innerHTML = `
      <div id="delete-dialog" class="open">
        <input id="delete-confirm-input" value="CONFIRM" />
        <button id="btn-copy-confirm-word">Copy</button>
        <button id="btn-delete-cancel">Cancel</button>
        <button id="btn-delete-confirm-ok">Confirm delete</button>
      </div>
      <div id="doc-list"></div>
      <div id="note-outlet" data-wb-mode="open"></div>
    `;
  }

  beforeEach(async () => {
    vi.resetModules();
    apiMocks.deleteEntry.mockReset();
    hashValue = '#/workbench?date=20260719&note=inbox%2Fnotes%2Fgone.md&layer=raw';
    historyBack = vi.fn();
    replaceCalls = [];
    reloadEvents = [];
    seedDeleteDom();

    Object.defineProperty(window, 'location', {
      configurable: true,
      value: {
        get hash() {
          return hashValue;
        },
        set hash(v) {
          hashValue = v;
        },
        replace(value) {
          replaceCalls.push(value);
          const idx = String(value).indexOf('#');
          hashValue = idx >= 0 ? String(value).slice(idx) : String(value);
        },
      },
    });
    Object.defineProperty(window, 'history', {
      configurable: true,
      value: { back: historyBack, length: 3 },
    });

    document.addEventListener('cta:reload', () => {
      reloadEvents.push(true);
    });

    // After resetModules, use the same state module the dialog binds to.
    const mod = await import('../frontend/js/state.js');
    mod.state.ui.activeDate = '20260719';
    mod.state.viewer.entry = {
      _id: 'entry-1',
      common_path: 'inbox/notes/gone.md',
    };

    await import('../frontend/js/components/modals/delete-dialog.js');
  });

  afterEach(() => {
    document.body.innerHTML = '';
  });

  async function clickConfirmOk() {
    const okBtn = document.getElementById('btn-delete-confirm-ok');
    okBtn.disabled = false;
    okBtn.click();
    await vi.waitFor(() => {
      expect(apiMocks.deleteEntry).toHaveBeenCalled();
    });
    await Promise.resolve();
  }

  it('delete success: closes dialog, replace to date-only list, reloads; no history.back', async () => {
    apiMocks.deleteEntry.mockResolvedValue({ ok: true });
    await clickConfirmOk();

    expect(apiMocks.deleteEntry).toHaveBeenCalledWith('entry-1');
    expect(document.getElementById('delete-dialog').classList.contains('open')).toBe(false);
    expect(replaceCalls.length).toBeGreaterThanOrEqual(1);
    const target = replaceCalls[replaceCalls.length - 1];
    expect(String(target)).toMatch(/#\/workbench\?date=20260719/);
    expect(String(target)).not.toMatch(/note=/);
    expect(String(target)).not.toMatch(/layer=/);
    expect(hashValue).toBe('#/workbench?date=20260719');
    expect(historyBack).not.toHaveBeenCalled();
    expect(reloadEvents.length).toBeGreaterThanOrEqual(1);
  });

  it('delete success replace semantics: current history entry loses deleted note URL', async () => {
    apiMocks.deleteEntry.mockResolvedValue({ ok: true });
    const stack = [hashValue];
    window.location.replace = (value) => {
      replaceCalls.push(value);
      const idx = String(value).indexOf('#');
      hashValue = idx >= 0 ? String(value).slice(idx) : String(value);
      stack[stack.length - 1] = hashValue;
    };

    await clickConfirmOk();

    expect(stack[stack.length - 1]).toBe('#/workbench?date=20260719');
    expect(stack.some((h) => String(h).includes('note='))).toBe(false);
  });

  it('delete failure: stays on note location, alerts, does not replace', async () => {
    apiMocks.deleteEntry.mockResolvedValue({ error: 'boom' });
    const alertSpy = vi.spyOn(window, 'alert').mockImplementation(() => {});
    const before = hashValue;

    await clickConfirmOk();

    expect(hashValue).toBe(before);
    expect(replaceCalls).toHaveLength(0);
    expect(historyBack).not.toHaveBeenCalled();
    expect(alertSpy).toHaveBeenCalled();
    expect(document.getElementById('delete-dialog').classList.contains('open')).toBe(true);
    alertSpy.mockRestore();
  });
});
