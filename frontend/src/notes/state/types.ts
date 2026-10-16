export type {
  HostNoteEntry as NoteEntry,
  HostNoteGroup as NoteGroup,
  HostNoteTag as NoteTag,
  HostViewer as NotesViewer,
} from '../../host/snapshot-types.ts';

export type AssistantNote = {
  title?: string;
  _id?: string;
  id?: string;
  source_type?: string;
  created_at?: string;
};
