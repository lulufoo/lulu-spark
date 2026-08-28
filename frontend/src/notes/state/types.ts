export type NoteEntry = {
  _id?: string;
  common_path: string;
  created_at?: string;
  layers?: string[];
  translations?: { zh?: string };
  tag_keys?: string[];
  tags?: { unknown?: boolean; value?: string; key?: string }[];
  done?: boolean;
  importance?: string;
  links?: { url: string }[];
  source_type?: string;
  title?: string;
};

export type NoteGroup = { date: string; entries: { id: string; entry: NoteEntry }[] };

export type NotesViewer = {
  entry: NoteEntry | null;
  layer: string;
  lang: string | null;
  rawText: string;
  annotation: Record<string, { comments?: unknown[] } | undefined>;
  createSession: unknown;
  outletMode: string;
  outletMessage: string;
  fileSize: string;
  loadError: string;
  loading: boolean;
  editing: boolean;
  saving: boolean;
  pendingCommit: boolean;
  bodyPaintKey: number;
};

export type AssistantNote = {
  title?: string;
  common_path?: string;
  source_type?: string;
  created_at?: string;
};
