export type {
  HostNoteEntry as NoteEntry,
  HostNoteGroup as NoteGroup,
  HostNoteTag as NoteTag,
  HostViewer as NotesViewer,
} from '../../host/snapshot-types.ts';

export type AssistantNote = {
  title?: string;
  common_path?: string;
  source_type?: string;
  created_at?: string;
};
