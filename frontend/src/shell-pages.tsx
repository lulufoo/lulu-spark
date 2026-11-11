import type { CSSProperties } from 'react';
import { useHostState } from './host/state.ts';
import { HomePage } from './home/page.tsx';
import { NotesFrame } from './notes/ui/notes-frame.tsx';
import { NotesMain } from './notes/page.tsx';
import { openReadLaterDialog } from './read-later/commands/dialog.ts';
import { navigate } from './router/index.ts';

function slotStyle(visible: boolean, shown: CSSProperties): CSSProperties {
  return visible ? shown : { ...shown, display: 'none' };
}

/** Hash pages. Always mounted so leftover island code can keep writing into these IDs. */
export function ShellPages({
  routeName,
  routeParams = {},
}: {
  routeName: string;
  routeParams?: Record<string, string>;
}) {
  const host = useHostState();
  const homeOn = routeName === 'home' || routeName === 'read-later';
  const notesOn = routeName === 'spark';

  return (
    <div className="app-pages">
      <div
        id="home-view"
        style={slotStyle(homeOn, {
          overflow: 'hidden',
          height: '100%',
          boxSizing: 'border-box',
        })}
      >
        {homeOn ? <HomePage navigate={navigate} openReadLater={openReadLaterDialog} /> : null}
      </div>
      <div className="notes-view" hidden={!notesOn}>
        <NotesFrame activeDate={host.ui.activeDate || ''}>
          <NotesMain routeParams={routeParams} active={notesOn} />
        </NotesFrame>
      </div>
    </div>
  );
}
