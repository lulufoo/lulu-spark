import { memo, useEffect, useLayoutEffect, useRef, useState, type FormEvent, type KeyboardEvent } from 'react';
import { attachHomeSidebarResize, detachHomeSidebarResize } from './ui/sidebar-resize.ts';
import { createRoot, type Root } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { getAuthUser, signOutAuth, startAuthLogin, type AuthUserView } from '../auth/oauth.ts';
import { hydrateHomeChatMarkdown, renderHomeChatMarkdown } from './ui/chat-render.ts';
import { AccountBar } from './ui/account-bar.tsx';
import { HomeNavIcon } from './ui/nav-icons.tsx';
import { SessionList } from './ui/session-list.tsx';
import { SessionMenu } from './ui/session-menu.tsx';
import { ContextPercent } from './ui/context-percent.tsx';
import { StagedList } from './ui/staged-list.tsx';
import { copyCurrentSessionId } from './commands/copy-session-id.ts';
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
import { createImeEnterGuard } from './ime-enter.ts';
import {
  composerInputLocked,
  composerLocked,
  messagePaintKey,
  progressHint,
  resetHomeState,
  useHomeState,
  type HubMessage,
} from './state/store.ts';
import { openCreateNote } from '../notes/commands/viewer/create.ts';
import { openBindDialog } from '../app-shell/ui/bind-dialog.tsx';
import { openSettingsDialog } from '../app-shell/ui/settings/dialog.tsx';
import { WindowDragStrip } from '../shared/window-drag-strip.tsx';

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
  if (dest === 'spark') navigateFn?.('#/spark');
  else if (dest === 'read-later') {
    if (typeof openReadLater === 'function') openReadLater();
    else navigateFn?.('#/read-later');
  } else if (dest === 'knowledge') navigateFn?.('#/knowledge');
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
  const [authUser, setAuthUser] = useState<AuthUserView | null>(null);
  const locked = composerLocked(state);
  const inputLocked = composerInputLocked(state);
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
    let cancelled = false;
    void getAuthUser()
      .then((user) => {
        if (!cancelled) setAuthUser(user);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
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
        <WindowDragStrip />
        <nav className="home-chat-nav" aria-label="Lulu Spark">
          <div className="home-chat-nav-group" data-home-nav-group="workspace">
            <div className="home-chat-nav-label">Workspace</div>
            <div className="home-chat-nav-notes">
              <button
                type="button"
                className="home-chat-nav-item home-desktop-shortcut"
                data-home-entry="spark"
                {...unreadProps(state.channelUnread.notes)}
                onClick={() => {
                  void markHomeEntryRead('spark');
                  goHomeEntry('spark', navigateFn, openReadLater);
                }}
              >
                <HomeNavIcon name="notes" />
                <span className="home-desktop-shortcut-label">Notes</span>
              </button>
              <button
                type="button"
                className="home-chat-new"
                data-role="create-note"
                aria-label="New note"
                title="New note"
                onClick={() => {
                  const temp_id =
                    typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function'
                      ? crypto.randomUUID()
                      : `note-${Date.now()}`;
                  goHomeEntry('spark', navigateFn, openReadLater);
                  void openCreateNote({ temp_id });
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
            <button
              type="button"
              className="home-chat-nav-item home-desktop-shortcut"
              data-home-entry="knowledge"
              onClick={() => goHomeEntry('knowledge', navigateFn, openReadLater)}
            >
              <HomeNavIcon name="knowledge" />
              <span className="home-desktop-shortcut-label">Knowledge</span>
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
              <HomeNavIcon name="read-later" />
              <span className="home-desktop-shortcut-label">Read Later</span>
            </button>
          </div>
          <div className="home-chat-nav-group" data-home-nav-group="settings">
            <div className="home-chat-nav-label">Settings</div>
            <button
              type="button"
              className="home-chat-nav-item home-desktop-shortcut"
              id="btn-bind"
              onClick={() => openBindDialog()}
            >
              <HomeNavIcon name="bind" />
              <span className="home-desktop-shortcut-label">Bind Device</span>
            </button>
            <button
              type="button"
              className="home-chat-nav-item home-desktop-shortcut"
              id="btn-settings"
              title="Settings"
              onClick={() => {
                void openSettingsDialog();
              }}
            >
              <HomeNavIcon name="settings" />
              <span className="home-desktop-shortcut-label">Settings</span>
            </button>
          </div>
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
        <AccountBar
          user={authUser}
          onLogin={(provider) => {
            void startAuthLogin(provider).catch(() => {});
          }}
          onLogout={() => {
            void signOutAuth()
              .then(() => setAuthUser(null))
              .catch(() => {});
          }}
        />
        <div
          className="home-chat-sidebar-resizer sidebar-resizer"
          role="separator"
          aria-orientation="vertical"
          aria-label="Resize sidebar"
          tabIndex={0}
        />
      </aside>
      <section className="home-chat-main">
        <div className="home-chat-main-drag" data-tauri-drag-region="deep">
          <SessionMenu sessionId={state.currentSessionId} onCopy={copyCurrentSessionId} />
        </div>
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
              disabled={inputLocked}
              onKeyDown={onComposerKey}
              onCompositionStart={() => imeEnterRef.current.onCompositionStart()}
              onCompositionEnd={() => imeEnterRef.current.onCompositionEnd()}
              onInput={syncComposerHeight}
            />
            <div className="home-chat-composer-corner">
              <ContextPercent percent={state.contextPercent} usage={state.contextUsage} />
              <button
                type="submit"
                className="home-chat-send"
                data-role="send"
                aria-label="Send"
                title="Send"
                disabled={locked}
              >
                <span className="home-chat-send-label">Send</span>
                <svg className="home-chat-send-icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false" fill="none">
                  <path d="M12 19V5M5 12l7-7 7 7" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" />
                </svg>
              </button>
            </div>
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
