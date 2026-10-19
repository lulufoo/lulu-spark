/** Snapshot shapes for `host/state.ts`. Runtime fields stay as they are. */

export type HostNoteTag = {
  unknown?: boolean;
  value?: string;
  key?: string;
};

export type HostNoteEntry = {
  _id?: string;
  id?: string;
  workspace_id?: string;
  project_id?: string;
  /** Path of the raw file inside the project; read-only, drives the directory tree. */
  raw_path?: string;
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

export type HostIndex = {
  data: Record<string, HostNoteEntry> | null;
  groupedByDate: HostNoteGroup[];
  filteredGroups: HostNoteGroup[];
  titleCache: Map<string, Map<string, string>>;
  annotations: Record<string, HostIndexAnnotation>;
  tagsRegistry: HostTagsRegistry;
  titleFetchCache: Map<string, string>;
  topicDescriptions: Record<string, string>;
  topicTitles: Record<string, string>;
  /** Project ids in the order Settings arranged them (the order the backend lists them). */
  topicOrder: string[];
  /** Which project ids are the workspace default project. */
  topicDefault: Record<string, boolean>;
  topicRepos: Record<string, string>;
};

export type HostUi = {
  activeDate: string | null;
  sparkRoot: string;
  activeTopic: string | null;
  loadError: string | null;
};

/** Layer blobs plus `links`. Kept loose this pass. */
export type HostViewerAnnotation = Record<string, unknown>;

export type HostCreateSession = {
  tempId: string;
  status: 'creating' | 'saving';
  /** Where the document goes: set when the session starts, never guessed at save time. */
  workspace_id?: string;
  project_id?: string;
  /** Folder of the project the document goes into (`/` is the project root); none: the default. */
  folder?: string;
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
  createSession: HostCreateSession | null;
  outletMode: HostOutletMode;
  outletMessage: string;
  fileSize: string;
  loadError: string;
  loading: boolean;
  editing: boolean;
  saving: boolean;
  bodyPaintKey: number;
  panelTitle: string;
};

export type HostWorkspace = {
  workspace_id: string;
  workspace_name: string;
  view_mode: 'directory' | 'timeline';
  is_default: boolean;
  /** Id of the configured icon; empty = the default icon. */
  icon: string;
  /** Position in the order Settings arranged; the list is already in this order. */
  sort_index?: number;
};

export type HostWorkspaces = {
  /** Id of the workspace the page shows. */
  current: string;
  list: HostWorkspace[];
};

export type HostState = {
  workspaces: HostWorkspaces;
  index: HostIndex;
  ui: HostUi;
  viewer: HostViewer;
};
