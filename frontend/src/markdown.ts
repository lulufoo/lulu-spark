import { marked } from 'marked';
import TurndownService from 'turndown';

marked.setOptions({ gfm: true, async: false });
globalThis.marked = {
  parse(source: string) {
    return String(marked.parse(source, { async: false }));
  },
};
globalThis.TurndownService = TurndownService;
