import { createContext, useContext } from 'react';

/** Where the list toolbar and the article header paint. Null keeps them in the page body. */
export const NotesHeaderSlot = createContext<HTMLElement | null>(null);

export function useNotesHeaderSlot(): HTMLElement | null {
  return useContext(NotesHeaderSlot);
}
