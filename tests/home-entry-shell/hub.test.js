// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import * as api from '../../frontend/src/host/api.ts';
import { mountHomeHub } from '../../frontend/src/home/hub.tsx';
import * as chatRender from '../../frontend/src/home/ui/chat-render.ts';
import { readFrontendJs, readMainSource } from '../helpers/read-frontend-js.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const mainJs = readMainSource();

describe('mountHomeHub', () => {
  let container;
  let navigate;
  let cleanup;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    navigate = vi.fn();
    cleanup = null;
  });

  afterEach(() => {
    cleanup?.();
    container.remove();
  });

  it('renders desktop shortcuts without page heading labels', () => {
    cleanup = mountHomeHub(container, { navigate });

    expect(container.querySelector('.home-hub-title')).toBeNull();
    expect(container.querySelector('.home-hub-subtitle')).toBeNull();
    expect(container.querySelector('.home-chat')).not.toBeNull();
    expect(container.querySelector('.home-chat-sidebar')).not.toBeNull();
    const resizer = container.querySelector('.home-chat-sidebar-resizer.sidebar-resizer');
    expect(resizer).not.toBeNull();
    expect(resizer?.getAttribute('role')).toBe('separator');
    expect(container.querySelector('[data-role="new-session"]')).not.toBeNull();
    expect(container.querySelector('[data-role="session-list"]')).not.toBeNull();

    const shortcuts = container.querySelectorAll('.home-desktop-shortcut');
    expect(shortcuts).toHaveLength(3);

    const sparkEntry = container.querySelector('[data-home-entry="spark"]');
    const readLaterEntry = container.querySelector('[data-home-entry="read-later"]');
    const knowledgeEntry = container.querySelector('[data-home-entry="knowledge"]');
    const todoTasksEntry = container.querySelector('[data-home-entry="todo-tasks"]');
    expect(sparkEntry).not.toBeNull();
    expect(readLaterEntry).not.toBeNull();
    expect(knowledgeEntry).not.toBeNull();
    expect(todoTasksEntry).toBeNull();
    expect(sparkEntry.textContent).toMatch(/Notes/);
    expect(readLaterEntry.textContent).toMatch(/Read Later/);
    expect(knowledgeEntry.textContent).toMatch(/Knowledge/);
    const labels = [...shortcuts].map((entry) => entry.querySelector('.home-desktop-shortcut-label')?.textContent);
    expect(labels).toEqual(['Notes', 'Knowledge', 'Read Later']);
  });

  it('navigates to #/spark when spark entry is clicked', () => {
    cleanup = mountHomeHub(container, { navigate });

    container.querySelector('[data-home-entry="spark"]').click();
    expect(navigate).toHaveBeenCalledWith('#/spark');
  });

  it('navigates to #/knowledge when knowledge entry is clicked', () => {
    cleanup = mountHomeHub(container, { navigate });

    container.querySelector('[data-home-entry="knowledge"]').click();
    expect(navigate).toHaveBeenCalledWith('#/knowledge');
  });

  it('opens read-later dialog when read-later entry is clicked', () => {
    const openReadLater = vi.fn();
    cleanup = mountHomeHub(container, { navigate, openReadLater });

    container.querySelector('[data-home-entry="read-later"]').click();
    expect(openReadLater).toHaveBeenCalledTimes(1);
    expect(navigate).not.toHaveBeenCalledWith('#/read-later');
  });



  it('returns cleanup that clears container', () => {
    const cleanup = mountHomeHub(container, { navigate });
    expect(typeof cleanup).toBe('function');
    cleanup();
    expect(container.innerHTML).toBe('');
  });

  it('does not leak listeners after repeated mount and unmount', () => {
    const cleanup1 = mountHomeHub(container, { navigate });
    cleanup1();
    expect(container.innerHTML).toBe('');
    const cleanup2 = mountHomeHub(container, { navigate });
    expect(container.querySelector('.home-chat')).not.toBeNull();
    cleanup2();
    expect(container.innerHTML).toBe('');
  });
});

describe('home hub chat sessions', () => {
  let container;
  let cleanup;
  /** @type {import('vitest').MockInstance} */
  let invokeSpy;
  /** @type {import('vitest').MockInstance} */
  let createChannelSpy;
  /** @type {{ session_id: string, title: string }[]} */
  let sessions;
  let currentId;
  let bound;
  let listenHandlers;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    cleanup = null;
    listenHandlers = {};
    window.__TAURI__ = {
      event: {
        listen: vi.fn(async (name, handler) => {
          listenHandlers[name] = handler;
          return vi.fn();
        }),
      },
    };
    sessions = [
      { session_id: 's1', title: 'Hello from first' },
      { session_id: 's2', title: 'New conversation' },
    ];
    currentId = 's2';
    bound = true;
    createChannelSpy = vi.spyOn(api, 'createChannel').mockImplementation(async (onmessage) => ({
      onmessage,
    }));
    invokeSpy = vi.spyOn(api, 'invoke').mockImplementation(async (cmd, args) => {
      if (cmd === 'query_binding') return { state: bound ? 'bound' : 'unbound' };
      if (cmd === 'list_chat_sessions') {
        return { sessions, current_session_id: currentId };
      }
      if (cmd === 'get_ai_assistant_binding' || cmd === 'select_chat_session') {
        if (cmd === 'select_chat_session') currentId = String(args?.sessionId || '');
        return {
          session_id: currentId,
          turns:
            currentId === 's1'
              ? [{ role: 'user', content: 'Hello from first' }]
              : [],
        };
      }
      if (cmd === 'create_chat_session') {
        currentId = 's3';
        sessions = [{ session_id: 's3', title: 'New conversation' }, ...sessions];
        return { session_id: 's3', turns: [] };
      }
      if (cmd === 'agent_chat_turn') {
        return { reply_text: 'Hi back', terminal: 'ok' };
      }
      return {};
    });
  });

  afterEach(() => {
    cleanup?.();
    createChannelSpy.mockRestore();
    invokeSpy.mockRestore();
    container.remove();
    delete window.__TAURI__;
  });

  async function mountReady() {
    cleanup = mountHomeHub(container, { navigate: vi.fn() });
    await vi.waitFor(() => {
      expect(container.querySelectorAll('.home-chat-session').length).toBe(2);
    });
  }

  it('copies the current session id from the chat overflow menu', async () => {
    const writeText = vi.fn(async () => {});
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText },
    });
    await mountReady();
    const menuBtn = container.querySelector('[data-role="chat-session-menu"]');
    expect(menuBtn).not.toBeNull();
    menuBtn.click();
    await vi.waitFor(() => {
      expect(container.querySelector('[data-role="copy-session-id"]')).not.toBeNull();
    });
    const copyBtn = container.querySelector('[data-role="copy-session-id"]');
    expect(copyBtn.textContent).toBe('Copy Session ID');
    copyBtn.click();
    await vi.waitFor(() => {
      expect(writeText).toHaveBeenCalledWith('s2');
    });
  });

  it('hides the chat overflow menu when unbound', async () => {
    bound = false;
    cleanup = mountHomeHub(container, { navigate: vi.fn() });
    await vi.waitFor(() => {
      expect(container.textContent).toMatch(/Chat requires a workspace Binding/);
    });
    expect(container.querySelector('[data-role="chat-session-menu"]')).toBeNull();
  });

  it('lists sessions and marks the current one active', async () => {
    await mountReady();
    const items = [...container.querySelectorAll('.home-chat-session')];
    expect(items.map((el) => el.textContent)).toEqual([
      'Hello from first',
      'New conversation',
    ]);
    await vi.waitFor(() => {
      expect(
        container
          .querySelector('[data-session-id="s2"]')
          ?.closest('.home-chat-session')
          ?.classList.contains('is-active'),
      ).toBe(true);
    });
  });

  it('renders assistant replies as markdown and keeps user text escaped', async () => {
    global.marked = {
      parse: vi.fn((md) => `<h2>Hello</h2><p>${md}</p>`),
    };
    invokeSpy.mockImplementation(async (cmd, args) => {
      if (cmd === 'query_binding') return { state: 'bound' };
      if (cmd === 'list_chat_sessions') {
        return { sessions, current_session_id: currentId };
      }
      if (cmd === 'select_chat_session') {
        currentId = String(args?.sessionId || '');
        return {
          session_id: currentId,
          turns: [
            { role: 'user', content: '**plain**' },
            { role: 'assistant', content: '## Hello' },
          ],
        };
      }
      if (cmd === 'get_ai_assistant_binding') {
        return { session_id: currentId, turns: [] };
      }
      return {};
    });
    try {
      await mountReady();
      container.querySelector('[data-session-id="s1"]').click();
      await vi.waitFor(() => {
        expect(container.querySelector('.home-chat-bubble--user')?.textContent).toBe(
          '**plain**',
        );
        expect(container.querySelector('.home-chat-md h2')?.textContent).toBe('Hello');
      });
      expect(global.marked.parse).toHaveBeenCalledWith('## Hello');
    } finally {
      delete global.marked;
    }
  });

  it('hydrates mermaid fences in assistant replies', async () => {
    global.marked = {
      parse: () => '<pre><code class="language-mermaid">graph TD; A-->B;</code></pre>',
    };
    global.mermaid = {
      initialize: vi.fn(),
      render: vi.fn(async () => ({ svg: '<svg data-chat="1"></svg>' })),
    };
    invokeSpy.mockImplementation(async (cmd, args) => {
      if (cmd === 'query_binding') return { state: 'bound' };
      if (cmd === 'list_chat_sessions') {
        return { sessions, current_session_id: currentId };
      }
      if (cmd === 'select_chat_session') {
        currentId = String(args?.sessionId || '');
        return {
          session_id: currentId,
          turns: [{ role: 'assistant', content: '```mermaid\ngraph TD; A-->B;\n```' }],
        };
      }
      if (cmd === 'get_ai_assistant_binding') {
        return { session_id: currentId, turns: [] };
      }
      return {};
    });
    try {
      await mountReady();
      container.querySelector('[data-session-id="s1"]').click();
      await vi.waitFor(() => {
        expect(container.querySelector('.home-chat-md .mermaid-diagram svg')).not.toBeNull();
      });
    } finally {
      delete global.marked;
      delete global.mermaid;
    }
  });

  it('selects a session and hydrates its turns', async () => {
    await mountReady();
    container.querySelector('[data-session-id="s1"]').click();
    await vi.waitFor(() => {
      expect(invokeSpy).toHaveBeenCalledWith('select_chat_session', { sessionId: 's1' });
      expect(container.querySelector('.home-chat-bubble--user')?.textContent).toBe(
        'Hello from first',
      );
    });
  });

  it('hydrates select when the host payload uses sessionId', async () => {
    invokeSpy.mockImplementation(async (cmd, args) => {
      if (cmd === 'query_binding') return { state: 'bound' };
      if (cmd === 'list_chat_sessions') {
        return { sessions, current_session_id: currentId };
      }
      if (cmd === 'select_chat_session') {
        currentId = String(args?.sessionId || '');
        return {
          sessionId: currentId,
          turns: [{ role: 'user', content: 'Hello from first' }],
        };
      }
      if (cmd === 'get_ai_assistant_binding') {
        return { session_id: currentId, turns: [] };
      }
      return {};
    });
    await mountReady();
    container.querySelector('[data-session-id="s1"]').click();
    await vi.waitFor(() => {
      expect(container.querySelector('.home-chat-bubble--user')?.textContent).toBe(
        'Hello from first',
      );
    });
  });

  it('does not let a stale binding pull wipe a selected session', async () => {
    await mountReady();
    await vi.waitFor(() => {
      expect(listenHandlers['ai-assistant:binding-changed']).toBeTruthy();
    });
    let releaseStale;
    const stale = new Promise((resolve) => {
      releaseStale = resolve;
    });
    let bindingPulls = 0;
    invokeSpy.mockImplementation(async (cmd, args) => {
      if (cmd === 'query_binding') return { state: 'bound' };
      if (cmd === 'list_chat_sessions') {
        return { sessions, current_session_id: currentId };
      }
      if (cmd === 'select_chat_session') {
        currentId = String(args?.sessionId || '');
        return {
          session_id: 's1',
          turns: [{ role: 'user', content: 'Hello from first' }],
        };
      }
      if (cmd === 'get_ai_assistant_binding') {
        bindingPulls += 1;
        if (bindingPulls > 1) {
          await stale;
          return { session_id: 's2', turns: [] };
        }
        return { session_id: currentId, turns: [] };
      }
      return {};
    });
    const rebound = listenHandlers['ai-assistant:binding-changed']({
      payload: { state: 'bound' },
    });
    await vi.waitFor(() => {
      expect(bindingPulls).toBeGreaterThan(0);
    });
    container.querySelector('[data-session-id="s1"]').click();
    await vi.waitFor(() => {
      expect(container.querySelector('.home-chat-bubble--user')?.textContent).toBe(
        'Hello from first',
      );
    });
    releaseStale();
    await rebound;
    expect(container.querySelector('.home-chat-bubble--user')?.textContent).toBe(
      'Hello from first',
    );
  });

  it('creates a session from the left + control', async () => {
    await mountReady();
    container.querySelector('[data-role="new-session"]').click();
    await vi.waitFor(() => {
      expect(invokeSpy).toHaveBeenCalledWith('create_chat_session');
      expect(container.querySelector('[data-session-id="s3"]')).not.toBeNull();
    });
  });

  it('sends a bound turn and shows the reply', async () => {
    await mountReady();
    const input = container.querySelector('[data-role="input"]');
    const form = container.querySelector('[data-role="form"]');
    expect(input.disabled).toBe(false);
    input.value = 'Hello there';
    form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
    await vi.waitFor(() => {
      expect(invokeSpy).toHaveBeenCalledWith('agent_chat_turn', {
        sessionId: 's2',
        message: 'Hello there',
        progress: expect.objectContaining({ onmessage: expect.any(Function) }),
      });
      expect(container.textContent).toMatch(/Hi back/);
    });
  });

  it('writes Requesting… onto the current chat hint and sidebar mark', async () => {
    let onProgress;
    createChannelSpy.mockImplementation(async (fn) => {
      onProgress = fn;
      return { onmessage: fn };
    });
    let releaseTurn;
    const turnGate = new Promise((resolve) => {
      releaseTurn = resolve;
    });
    invokeSpy.mockImplementation(async (cmd, args) => {
      if (cmd === 'query_binding') return { state: 'bound' };
      if (cmd === 'list_chat_sessions') {
        return { sessions, current_session_id: currentId };
      }
      if (cmd === 'get_ai_assistant_binding') {
        return { session_id: currentId, turns: [] };
      }
      if (cmd === 'agent_chat_turn') {
        onProgress?.({ session_id: 's2', request_id: 'trace_1', desc: 'Requesting…' });
        await turnGate;
        return { reply_text: 'Hi back', terminal: 'ok' };
      }
      return {};
    });
    await mountReady();
    const input = container.querySelector('[data-role="input"]');
    const form = container.querySelector('[data-role="form"]');
    input.value = 'Hello there';
    form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
    await vi.waitFor(() => {
      const hint = container.querySelector('[data-role="progress-hint"]');
      expect(hint?.textContent).toBe('Requesting…');
      expect(container.querySelector('[data-role="messages"] [data-role="progress-hint"]')).toBe(
        hint,
      );
      expect(container.querySelector('[data-role="form"] [data-role="progress-hint"]')).toBeNull();
      expect(
        container.querySelector('[data-session-id="s2"] .home-chat-session-progress')
          ?.textContent,
      ).toBe('…');
    });
    releaseTurn();
    await vi.waitFor(() => {
      expect(container.querySelector('[data-role="progress-hint"]')?.hidden).toBe(true);
      expect(container.textContent).toMatch(/Hi back/);
    });
  });

  it('does not rehydrate mermaid or yank scroll on progress-only paints', async () => {
    const hydrateSpy = vi.spyOn(chatRender, 'hydrateHomeChatMarkdown');
    let onProgress;
    createChannelSpy.mockImplementation(async (fn) => {
      onProgress = fn;
      return { onmessage: fn };
    });
    let releaseTurn;
    const turnGate = new Promise((resolve) => {
      releaseTurn = resolve;
    });
    invokeSpy.mockImplementation(async (cmd) => {
      if (cmd === 'query_binding') return { state: 'bound' };
      if (cmd === 'list_chat_sessions') {
        return { sessions, current_session_id: currentId };
      }
      if (cmd === 'get_ai_assistant_binding') {
        return { session_id: currentId, turns: [] };
      }
      if (cmd === 'agent_chat_turn') {
        await turnGate;
        return { reply_text: 'Hi back', terminal: 'ok' };
      }
      return {};
    });
    await mountReady();
    const input = container.querySelector('[data-role="input"]');
    const form = container.querySelector('[data-role="form"]');
    input.value = 'Hello there';
    form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
    await vi.waitFor(() => {
      expect(container.querySelector('.home-chat-bubble--user')?.textContent).toBe(
        'Hello there',
      );
      expect(onProgress).toEqual(expect.any(Function));
    });
    const afterSend = hydrateSpy.mock.calls.length;
    const messagesEl = container.querySelector('[data-role="messages"]');
    Object.defineProperty(messagesEl, 'scrollHeight', { configurable: true, value: 400 });
    Object.defineProperty(messagesEl, 'clientHeight', { configurable: true, value: 200 });
    messagesEl.scrollTop = 12;
    onProgress?.({ session_id: 's2', request_id: 'trace_1', desc: 'Requesting…' });
    onProgress?.({ session_id: 's2', request_id: 'trace_1', desc: 'Working…' });
    expect(container.querySelector('[data-role="progress-hint"]')?.textContent).toBe(
      'Working…',
    );
    expect(hydrateSpy.mock.calls.length).toBe(afterSend);
    expect(messagesEl.scrollTop).toBe(12);
    hydrateSpy.mockRestore();
    releaseTurn();
  });

  it('keeps the input enabled and send disabled while a turn is in flight', async () => {
    let releaseTurn;
    const turnGate = new Promise((resolve) => {
      releaseTurn = resolve;
    });
    invokeSpy.mockImplementation(async (cmd) => {
      if (cmd === 'query_binding') return { state: 'bound' };
      if (cmd === 'list_chat_sessions') {
        return { sessions, current_session_id: currentId };
      }
      if (cmd === 'get_ai_assistant_binding') {
        return { session_id: currentId, turns: [] };
      }
      if (cmd === 'agent_chat_turn') {
        await turnGate;
        return { reply_text: 'Hi back', terminal: 'ok' };
      }
      return {};
    });
    await mountReady();
    const input = container.querySelector('[data-role="input"]');
    const send = container.querySelector('[data-role="send"]');
    const form = container.querySelector('[data-role="form"]');
    input.value = 'Hello there';
    form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
    await vi.waitFor(() => {
      expect(container.querySelector('.home-chat-bubble--user')?.textContent).toBe(
        'Hello there',
      );
      expect(input.disabled).toBe(false);
      expect(send.disabled).toBe(true);
    });
    input.value = 'Draft while waiting';
    form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
    expect(invokeSpy.mock.calls.filter(([cmd]) => cmd === 'agent_chat_turn')).toHaveLength(1);
    expect(input.value).toBe('Draft while waiting');
    releaseTurn();
    await vi.waitFor(() => {
      expect(send.disabled).toBe(false);
      expect(container.textContent).toMatch(/Hi back/);
    });
    expect(input.value).toBe('Draft while waiting');
  });

  it('disables the composer when Binding is unbound', async () => {
    bound = false;
    cleanup = mountHomeHub(container, { navigate: vi.fn() });
    await vi.waitFor(() => {
      expect(container.querySelector('[data-role="input"]').disabled).toBe(true);
      expect(container.querySelector('[data-role="send"]').disabled).toBe(true);
      expect(container.textContent).toMatch(/Chat requires a workspace Binding/);
    });
  });

  it('shows date-time when a session has no title', async () => {
    sessions = [{ session_id: 's9', title: '', updated_at: 1756272000 }];
    currentId = 's9';
    cleanup = mountHomeHub(container, { navigate: vi.fn() });
    await vi.waitFor(() => {
      const item = container.querySelector('[data-session-id="s9"]');
      expect(item).not.toBeNull();
      expect(item.textContent).toMatch(/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}$/);
    });
  });

  it('keeps only the 20 most recent sessions from the host list', async () => {
    sessions = Array.from({ length: 25 }, (_, i) => ({
      session_id: `s${i}`,
      title: `Chat ${i}`,
    }));
    currentId = 's0';
    cleanup = mountHomeHub(container, { navigate: vi.fn() });
    await vi.waitFor(() => {
      expect(container.querySelectorAll('.home-chat-session')).toHaveLength(20);
    });
  });

  it('uses English chat chrome and the list/select/create commands', () => {
    const source = readFrontendJs('frontend/src/home/hub.tsx');
    expect(source).toMatch(/list_chat_sessions/);
    expect(source).toMatch(/select_chat_session/);
    expect(source).toMatch(/create_chat_session/);
    expect(source).toMatch(/delete_chat_session/);
    expect(source).toMatch(/chat-render/);
    expect(source).not.toMatch(/ensure_ai_assistant_session/);
    expect(source).not.toMatch(/viewer\.js|comment-markdown/);
    cleanup = mountHomeHub(container, { navigate: vi.fn() });
    expect(container.querySelector('.home-chat-sessions-title')?.textContent).toBe('Chats');
    expect(container.querySelector('.home-chat-composer-dock')).not.toBeNull();
    expect(container.querySelector('[data-role="input"]')?.placeholder).toBe('Message…');
    expect(container.querySelector('[data-role="send"]')?.textContent.trim()).toBe('Send');
    expect(container.textContent).not.toMatch(/会话|发送|待办/);
  });
});

describe('home hub composer and hub pairing', () => {
  it('reserves a right rail so the composer dock does not sit under the global +', () => {
    const appCss = readFileSync(join(fixtureRoot, 'frontend/app.css'), 'utf8');
    expect(appCss).toMatch(/--home-chat-rail:\s*44px/);
    expect(appCss).toMatch(/\.home-chat-session-menu\s*\{[^}]*position:\s*absolute/);
    expect(appCss).toMatch(/\.home-chat-session-menu\s*\{[^}]*right:\s*8px/);
    expect(appCss).toMatch(/--home-chat-col:\s*calc\(50% \+ 360px\)/);
    expect(appCss).toMatch(/\.home-chat-composer-dock/);
    expect(appCss).toMatch(/\.home-chat-composer-dock\s*\{[^}]*min-height:\s*40px/);
    expect(appCss).toMatch(
      /\.home-chat-staged\s*\{[^}]*margin:\s*0;[^}]*border-bottom:\s*none/,
    );
    expect(appCss).toMatch(
      /\.home-chat-staged \+ \.home-chat-composer-dock\s*\{[^}]*border-top-left-radius:\s*0/,
    );
    expect(appCss).toMatch(/\.home-chat-send\s*\{[^}]*width:\s*20px/);
    expect(appCss).toMatch(/\.home-chat-bubble--user\s*\{[^}]*background:\s*#f0f2f4/);
    expect(appCss).toMatch(/\.home-chat-bubble--assistant\s*\{[^}]*white-space:\s*normal/);
    expect(appCss).toMatch(/\.home-chat-md\s+p\s*\{/);
    expect(appCss).toMatch(
      /\.home-chat-composer\s*\{[^}]*position:\s*absolute/,
    );
    expect(appCss).toMatch(
      /\.home-chat-composer\s*\{[^}]*padding:\s*28px var\(--home-chat-rail\) 20px 12px/,
    );
    expect(appCss).toMatch(
      /\.home-chat-delete-confirm\s*\{[^}]*pointer-events:\s*auto/,
    );
    expect(appCss).toMatch(/\.home-entry-shell__cluster\s*\{[^}]*bottom:\s*20px/);
    expect(appCss).toMatch(/\.home-chat-turn--user\s*\{[^}]*align-items:\s*flex-end/);
    expect(appCss).toMatch(
      /\.home-entry-shell__hub\s*\{[^}]*background:\s*#fff/,
    );
  });
});

describe('home hub shell integration', () => {
  it('main.js mounts HomeHub on home route with Phase2 default landing', () => {
    expect(mainJs).toMatch(/HomePage/);
    expect(mainJs).not.toMatch(/home:\s*redirectToSpark/);
    expect(mainJs).toMatch(/['"]#\/home['"]/);
  });

  it('main.js keeps the brand mark and marks back off home', () => {
    expect(mainJs).toMatch(/btn-nav-home-title/);
    expect(mainJs).toMatch(/classList\.toggle\('is-back', !onHome\)/);
  });
});
