/** Snapshot shapes for `host/state.ts`. Runtime fields stay as they are. */

export type HostNoteTag = {
  unknown?: boolean;
  value?: string;
  key?: string;
};

export type HostNoteEntry = {
  _id?: string;
  common_path: string;
  created_at?: string;
  layers?: string[];
  translations?: { zh?: string };
  tag_keys?: string[];
  tags?: HostNoteTag[];
  done?: boolean;
  importance?: string;
  links?: { url: string }[];
  source_type?: string;
  title?: string;
  _comment_counts?: Record<string, number>;
};

export type HostNoteGroup = {
  date: string;
  entries: { id: string; entry: HostNoteEntry }[];
};

export type HostIndexAnnotation = {
  done?: boolean;
  importance?: string;
  links?: { url: string }[];
  comment_counts?: Record<string, number>;
  tags?: HostNoteTag[];
  tag_keys?: string[];
};

export type HostTagMeta = { value?: string; refs?: number };

export type HostTagsRegistry = {
  keys: Record<string, HostTagMeta>;
};

export type HostDiffStatus = 'modified' | 'conflict';

export type HostIndex = {
  data: Record<string, HostNoteEntry> | null;
  groupedByDate: HostNoteGroup[];
  filteredGroups: HostNoteGroup[];
  titleCache: Map<string, Map<string, string>>;
  diffStatus: Map<string, HostDiffStatus>;
  annotations: Record<string, HostIndexAnnotation>;
  tagsRegistry: HostTagsRegistry;
  titleFetchCache: Map<string, string>;
  topicDescriptions: Record<string, string>;
  topicTitles: Record<string, string>;
  topicRepos: Record<string, string>;
};

export type HostUi = {
  activeDate: string | null;
  workbenchRoot: string;
  knowledgeRoot: string;
  githubUserUrl: string;
  activeTopic: string | null;
  activeTagKey: string | null;
  loadError: string | null;
};

/** Layer blobs plus `links`. Kept loose this pass. */
export type HostViewerAnnotation = Record<string, unknown>;

export type HostCreateSession = {
  tempId: string;
  status: 'creating' | 'saving';
};

export type HostOutletMode = '' | 'open' | 'create' | 'safe-empty';

export type HostViewer = {
  entry: HostNoteEntry | null;
  layer: string;
  lang: string | null;
  rawText: string;
  annotation: HostViewerAnnotation;
  commentEditCtx: unknown | null;
  scrollCache: Record<string, number>;
  isKb: boolean;
  kbRepo: string | null;
  kbPath: string | null;
  createSession: HostCreateSession | null;
  outletMode: HostOutletMode;
  outletMessage: string;
  fileSize: string;
  loadError: string;
  loading: boolean;
  editing: boolean;
  saving: boolean;
  pendingCommit: boolean;
  bodyPaintKey: number;
  panelTitle: string;
};

export type HostState = {
  index: HostIndex;
  ui: HostUi;
  viewer: HostViewer;
};
