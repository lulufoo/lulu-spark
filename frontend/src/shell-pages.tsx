import { useEffect, type CSSProperties } from 'react';
import { applySearchNavChrome } from './app-shell/ui/nav-chrome.ts';
import { useHostState } from './host/state.ts';
import { HomePage } from './home/page.tsx';
import { KnowledgeDocPage } from './knowledge/page.tsx';
import { TodoTasksPage } from './todo-task/page.tsx';
import { NotesMain, NotesSidebar } from './notes/page.tsx';
import { openReadLaterDialog } from './read-later/commands/dialog.ts';
import { navigate } from './router/index.ts';

function slotStyle(visible: boolean, shown: CSSProperties): CSSProperties {
  return visible ? shown : { ...shown, display: 'none' };
}

/** Five hash pages. Always mounted so leftover island code can keep writing into these IDs. */
export function ShellPages({
  routeName,
  routeParams = {},
}: {
  routeName: string;
  routeParams?: Record<string, string>;
}) {
  const host = useHostState();
  const homeOn = routeName === 'home' || routeName === 'read-later';
  const knowledgeOn = routeName === 'knowledge-doc';
  const todoOn = routeName === 'todo-tasks';
  const notesOn = routeName === 'workbench';

  useEffect(() => {
    applySearchNavChrome(routeName);
  }, [routeName]);

  return (
    <>
      <div
        id="home-view"
        style={slotStyle(homeOn, {
          overflow: 'hidden',
          height: 'calc(100vh - 52px)',
          boxSizing: 'border-box',
        })}
      >
        {homeOn ? <HomePage navigate={navigate} openReadLater={openReadLaterDialog} /> : null}
      </div>
      <div
        id="knowledge-doc-view"
        style={slotStyle(knowledgeOn, {
          height: 'calc(100vh - 52px)',
          boxSizing: 'border-box',
        })}
      >
        {knowledgeOn ? (
          <KnowledgeDocPage
            key={routeParams.repo ?? ''}
            repo={routeParams.repo ?? ''}
            path={routeParams.path ?? ''}
            navigate={navigate}
          />
        ) : null}
      </div>
      <div
        id="todo-tasks-view"
        style={slotStyle(todoOn, {
          width: '100%',
          minWidth: 0,
          height: 'calc(100vh - 52px)',
          overflow: 'hidden',
          boxSizing: 'border-box',
        })}
      >
        {todoOn ? (
          <TodoTasksPage
            masterId={routeParams.master ?? ''}
            subId={routeParams.sub ?? ''}
            navigate={navigate}
          />
        ) : null}
      </div>

      <div
        className="layout"
        data-active-date={host.ui.activeDate || ''}
        style={notesOn ? undefined : { display: 'none' }}
      >
        <aside id="sidebar">
          <NotesSidebar />
        </aside>
        <main id="main">
          <NotesMain routeParams={routeParams} />
          <div
            id="feed-view"
            style={{ display: 'none', padding: '20px', overflowY: 'auto', height: '100%', boxSizing: 'border-box' }}
          />
        </main>
      </div>
    </>
  );
}
