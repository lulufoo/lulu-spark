export type ReadLaterFilter = 'all' | 'unread';

export type ReadLaterEntry = {
  id: string;
  title?: string;
  url: string;
  saved_at?: string;
  read?: boolean;
};
