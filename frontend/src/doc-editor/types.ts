export type HighlightRecord = {
  id: string;
  text: string;
  occurrence?: number;
};

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
};

export type DeleteHighlightArgs = ApplyHighlightsArgs & {
  id?: string;
};

export type HighlightOverlayConfig = {
  getBody?: () => Element | null;
  getEditArea?: () => Element | null;
  getIdentityKey?: (bodyEl?: Element | null) => string;
  excludeBarId?: string;
  buttonId?: string;
};
