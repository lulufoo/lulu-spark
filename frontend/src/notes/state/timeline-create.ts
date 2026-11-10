import { createModuleStore } from '../../shared/module-store.ts';

/** The timeline card that asks for a title before a document exists. */
export type TimelineCreateSnap = {
  /** Project the document will belong to. Empty while the card is hidden. */
  project: string;
  title: string;
  saving: boolean;
  /** Bumped each time "+" opens the card, so the title field takes focus again. */
  tick: number;
};

export const timelineCreateStore = createModuleStore<TimelineCreateSnap>({
  project: '',
  title: '',
  saving: false,
  tick: 0,
});
