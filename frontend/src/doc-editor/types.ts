export type HighlightRecord = {
  id: string;
  text: string;
  occurrence?: number;
  comments?: { id: string; text: string }[];
};

export type HighlightPoint = { x: number; y: number };
export type OpenHighlightComment = (
  key: string,
  highlight: HighlightRecord,
  anchor: HTMLElement,
  point?: HighlightPoint,
) => void;

export type HighlightsPayload = {
  highlights?: HighlightRecord[];
  ok?: boolean;
  error?: string;
  id?: string;
};

export type ApplyHighlightsArgs = {
  bodyEl?: Element | null;
  identityKey?: string;
  excludeBarId?: string;
  onOpenComment?: OpenHighlightComment;
  isCurrent?: () => boolean;
};

export type HighlightOverlayConfig = {
  getBody?: () => Element | null;
  getEditArea?: () => Element | null;
  getIdentityKey?: (bodyEl?: Element | null) => string;
  excludeBarId?: string;
  buttonId?: string;
  onOpenComment?: OpenHighlightComment;
};
