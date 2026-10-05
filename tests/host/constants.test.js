import { describe, expect, it } from 'vitest'
import { notesFileRelPath } from '../../frontend/src/host/constants.ts'

describe('notesFileRelPath', () => {
  it('joins notes layer and common path', () => {
    expect(notesFileRelPath('raw', 'ai/note.md')).toBe('notes/raw/ai/note.md')
  })
})
