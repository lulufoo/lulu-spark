/** @deprecated */
export const REPO = ''

export const NOTES_DIR = 'notes'

export function notesFileRelPath(layer: string, activePath: string): string {
  return `${NOTES_DIR}/${layer}/${activePath}`
}

export const TAG_VALUE_MAX_LEN = 64
export const TAG_SUGGEST_MIN_SCORE = 0.6

export const LAYERS = ['raw', 'digest']
export const IMPORTANCE_CYCLE = [undefined, 'high', 'medium', 'low']
