import { memo, useEffect, useLayoutEffect, useRef, type FormEvent, type KeyboardEvent } from 'react';
import { attachHomeSidebarResize, detachHomeSidebarResize } from './ui/sidebar-resize.ts';
import { createRoot, type Root } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { hydrateHomeChatMarkdown, renderHomeChatMarkdown } from './ui/chat-render.ts';
import { SessionList } from './ui/session-list.tsx';
import {
  createSession,
  deleteSession,
  markHomeEntryRead,
  resetHomeCommands,
  selectSession,
  sendMessage,
  showActionError,
  startHomeHub,
  stopHomeHub,
} from './commands/hub.ts';
import { consumeComposerFocus } from './commands/composer-focus.ts';
import { openStagedFile, unstageStaged } from './commands/staged.ts';
import { StagedList } from './ui/staged-list.tsx';
import { createImeEnterGuard } from './ime-enter.ts';
import {
  composerLocked,
  messagePaintKey,
  progressHint,
  resetHomeState,
  useHomeState,
  type HubMessage,
} from './state/store.ts';

export type HomePageChrome = {
  navigate?: (hash: string) => void;
  openReadLater?: () => void;
};

const MessageThread = memo(function MessageThread({
  hostBound,
  currentSessionId,
  messages,
}: {
  hostBound: boolean;
  currentSessionId: string;
  messages: HubMessage[];
}) {
  if (!hostBound) {
    return (
      <div className="home-chat-thread">
        <p className="home-chat-empty">Chat requires a workspace Binding.</p>
      </div>
    );
  }
  if (!currentSessionId) {
    return (
      <div className="home-chat-thread">
        <p className="home-chat-empty">Select a conversation or start a new one.</p>
      </div>
    );
  }
  if (!messages.length) {
    return (
      <div className="home-chat-thread">
        <p className="home-chat-empty">No messages yet.</p>
      </div>
    );
  }
  return (
    <div className="home-chat-thread">
      {messages.map((m, i) => {
        const kind = m.role === 'user' ? 'user' : m.error ? 'error' : 'assistant';
        return (
          <div key={i} className={`home-chat-turn home-chat-turn--${kind}`}>
            <div className={`home-chat-bubble home-chat-bubble--${kind}`}>
              {kind === 'assistant' ? (
                <div
                  className="home-chat-md"
                  dangerouslySetInnerHTML={{ __html: renderHomeChatMarkdown(m.text) }}
                />
              ) : (
                m.text
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
});

function goHomeEntry(
  dest: string | undefined,
  navigateFn?: (hash: string) => void,
  openReadLater?: () => void,
) {
  if (dest === 'workbench') navigateFn?.('#/workbench');
  else if (dest === 'read-later') {
    if (typeof openReadLater === 'function') openReadLater();
    else navigateFn?.('#/read-later');
  } else if (dest === 'knowledge') navigateFn?.('#/knowledge');
  else if (dest === 'todo-tasks') navigateFn?.('#/todo-tasks');
}

function unreadProps(unread: boolean) {
  return unread ? { 'data-home-unread': 'true' as const } : {};
}

/** Home chat page. Reads the home store; commands update the store. */
export function HomePage({
  navigate: navigateFn,
  openReadLater,
}: HomePageChrome = {}) {
  const state = useHomeState();
  const locked = composerLocked(state);
  const hint = progressHint(state);
  const showProgress = Boolean(hint) || (state.hostBound && locked);
  const messagesRef = useRef<HTMLDivElement | null>(null);
  const formRef = useRef<HTMLFormElement | null>(null);
  const inputRef = useRef<HTMLTextAreaElement | null>(null);
  const imeEnterRef = useRef(createImeEnterGuard());
  const sidebarRef = useRef<HTMLElement | null>(null);
  const lastPaintKeyRef = useRef('');
  const mdPaintGenRef = useRef(0);
  const paintKey = messagePaintKey(state);

  useEffect(() => {
    startHomeHub();
    return () => stopHomeHub();
  }, []);

  useEffect(() => {
    if (!consumeComposerFocus()) return;
    inputRef.current?.focus();
  }, [state.currentSessionId, state.staged]);

  useEffect(() => {
    const guard = imeEnterRef.current;
    return () => guard.dispose();
  }, []);

  useEffect(() => {
    const aside = sidebarRef.current;
    if (!aside) return undefined;
    attachHomeSidebarResize(aside);
    return () => detachHomeSidebarResize();
  }, []);

  useLayoutEffect(() => {
    const messagesEl = messagesRef.current;
    if (!messagesEl) return;
    const messagesChanged = paintKey !== lastPaintKeyRef.current;
    lastPaintKeyRef.current = paintKey;
    const needsHydrate =
      messagesChanged || Boolean(messagesEl.querySelector('pre > code.language-mermaid'));
    if (messagesChanged) {
      messagesEl.scrollTop = messagesEl.scrollHeight;
    }
    if (!needsHydrate) return;
    const paintGen = ++mdPaintGenRef.current;
    void hydrateHomeChatMarkdown(messagesEl).then(() => {
      if (paintGen === mdPaintGenRef.current && messagesChanged) {
        messagesEl.scrollTop = messagesEl.scrollHeight;
      }
    });
  }, [paintKey]);

  function syncComposerHeight() {
    const input = inputRef.current;
    const form = formRef.current;
    const messagesEl = messagesRef.current;
    if (!input || !form) return;
    input.style.height = '20px';
    const next = Math.min(Math.max(input.scrollHeight, 20), 120);
    input.style.height = `${next}px`;
    const composerH = form.offsetHeight;
    if (composerH > 0 && messagesEl) {
      messagesEl.style.paddingBottom = `${composerH}px`;
    }
  }

  useEffect(() => {
    syncComposerHeight();
  });

  function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const input = inputRef.current;
    const text = input ? String(input.value || '').trim() : '';
    if (!text || state.inFlightIds.includes(state.currentSessionId)) return;
    if (input) input.value = '';
    syncComposerHeight();
    void sendMessage(text);
  }

  function onComposerKey(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key !== 'Enter' || event.shiftKey) return;
    if (imeEnterRef.current.isBlocked(event.nativeEvent)) return;
    event.preventDefault();
    if (locked || state.inFlightIds.includes(state.currentSessionId)) return;
    const form = formRef.current;
    if (!form) return;
    if (typeof form.requestSubmit === 'function') form.requestSubmit();
    else form.dispatchEvent(new Event('submit', { cancelable: true, bubbles: true }));
  }

  return (
    <div className="home-chat">
      <aside ref={sidebarRef} className="home-chat-sidebar">
        <nav className="home-chat-nav" aria-label="Workbench">
          <button
            type="button"
            className="home-chat-nav-item home-desktop-shortcut"
            data-home-entry="workbench"
            {...unreadProps(state.channelUnread.notes)}
            onClick={() => {
              void markHomeEntryRead('workbench');
              goHomeEntry('workbench', navigateFn, openReadLater);
            }}
          >
            <span className="home-desktop-shortcut-icon" aria-hidden="true">
              📂
            </span>
            <span className="home-desktop-shortcut-label">Notes</span>
          </button>
          <button
            type="button"
            className="home-chat-nav-item home-desktop-shortcut"
            data-home-entry="read-later"
            {...unreadProps(state.channelUnread.read_later)}
            onClick={() => {
              void markHomeEntryRead('read-later');
              goHomeEntry('read-later', navigateFn, openReadLater);
            }}
          >
            <span className="home-desktop-shortcut-icon" aria-hidden="true">
              📑
            </span>
            <span className="home-desktop-shortcut-label">Read Later</span>
          </button>
          <button
            type="button"
            className="home-chat-nav-item home-desktop-shortcut"
            data-home-entry="knowledge"
            onClick={() => goHomeEntry('knowledge', navigateFn, openReadLater)}
          >
            <span className="home-desktop-shortcut-icon" aria-hidden="true">
              📚
            </span>
            <span className="home-desktop-shortcut-label">Knowledge</span>
          </button>
          <button
            type="button"
            className="home-chat-nav-item home-desktop-shortcut"
            data-home-entry="todo-tasks"
            {...unreadProps(state.channelUnread.todos)}
            onClick={() => {
              void markHomeEntryRead('todo-tasks');
              goHomeEntry('todo-tasks', navigateFn, openReadLater);
            }}
          >
            <span className="home-desktop-shortcut-icon" aria-hidden="true">
              📋
            </span>
            <span className="home-desktop-shortcut-label">Todos</span>
          </button>
        </nav>
        <div className="home-chat-sessions-head">
          <span className="home-chat-sessions-title">Chats</span>
          <button
            type="button"
            className="home-chat-new"
            data-role="new-session"
            aria-label="New conversation"
            title="New conversation"
            onClick={() => {
              void createSession()
                .then((ok) => {
                  if (ok) inputRef.current?.focus();
                })
                .catch(showActionError);
            }}
          >
            <svg className="home-chat-new-icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
              <path
                fill="currentColor"
                d="M11 5a1 1 0 1 1 2 0v6h6a1 1 0 1 1 0 2h-6v6a1 1 0 1 1-2 0v-6H5a1 1 0 1 1 0-2h6V5z"
              />
            </svg>
          </button>
        </div>
        <div className="home-chat-sessions" data-role="session-list" role="list">
          <SessionList
            sessions={state.sessions}
            currentSessionId={state.currentSessionId}
            progressByChat={state.progressByChat}
            inFlightIds={state.inFlightIds}
            onSelect={(id) => {
              void selectSession(id).catch(showActionError);
            }}
            onDelete={(id) => {
              void deleteSession(id);
            }}
          />
        </div>
        <div
          className="home-chat-sidebar-resizer sidebar-resizer"
          role="separator"
          aria-orientation="vertical"
          aria-label="Resize sidebar"
          tabIndex={0}
        />
      </aside>
      <section className="home-chat-main">
        <div ref={messagesRef} className="home-chat-messages" data-role="messages" aria-live="polite">
          <MessageThread
            hostBound={state.hostBound}
            currentSessionId={state.currentSessionId}
            messages={state.messages}
          />
          <p className="home-chat-progress" data-role="progress-hint" hidden={!showProgress}>
            {hint ? <span className="home-chat-progress-text">{hint}</span> : '\u00a0'}
          </p>
        </div>
        <form ref={formRef} className="home-chat-composer" data-role="form" onSubmit={onSubmit}>
          <StagedList
            items={state.staged}
            canRemove={state.hostBound && !locked}
            onOpen={openStagedFile}
            onRemove={(id) => {
              void unstageStaged(id);
            }}
          />
          <div className="home-chat-composer-dock">
            <textarea
              ref={inputRef}
              className="home-chat-input"
              data-role="input"
              rows={1}
              placeholder="Message…"
              disabled={locked}
              onKeyDown={onComposerKey}
              onCompositionStart={() => imeEnterRef.current.onCompositionStart()}
              onCompositionEnd={() => imeEnterRef.current.onCompositionEnd()}
              onInput={syncComposerHeight}
            />
            <button
              type="submit"
              className="home-chat-send"
              data-role="send"
              aria-label="Send"
              title="Send"
              disabled={locked}
            >
              <span className="home-chat-send-label">Send</span>
              <svg className="home-chat-send-icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
                <path
                  fill="currentColor"
                  d="M5.2 11.1 18.6 4.4a.8.8 0 0 1 1.1.9l-3.7 13.5a.8.8 0 0 1-1.4.3l-3.6-4.7-4.8-1.6a.8.8 0 0 1 0-1.5z"
                />
              </svg>
            </button>
          </div>
        </form>
      </section>
    </div>
  );
}

/**
 * Test / leftover helper. Production Home is a child of ShellPages, not this root.
 */
export function mountHomeHub(
  container: HTMLElement,
  { navigate, openReadLater }: HomePageChrome = {},
) {
  stopHomeHub();
  resetHomeCommands();
  resetHomeState();
  let root: Root | null = createRoot(container);
  flushSync(() => {
    root!.render(<HomePage navigate={navigate} openReadLater={openReadLater} />);
  });
  return () => {
    stopHomeHub();
    resetHomeCommands();
    flushSync(() => {
      root?.unmount();
    });
    root = null;
    resetHomeState();
    container.innerHTML = '';
  };
}
