export type ParsedRoute = {
  name: string;
  params: Record<string, string>;
};

export type RouteHandlers = Record<string, (ctx: ParsedRoute) => void>;

export type NoteNavParams = {
  date?: string;
  note?: string;
  layer?: string;
};

export type ListNavOptions = {
  date?: string;
  onClearCreate?: () => void;
};
