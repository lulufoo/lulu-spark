// @vitest-environment node
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const SETTINGS_SHADOW =
  '0 8px 32px var(--shadow-rgba-010409-18), 0 0 0 1px var(--shadow-rgba-1f2328-08)';

function readRel(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
}

function escapeRe(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

describe('fullscreen overlay chrome', () => {
  it('points every listed mask at the Settings overlay token', () => {
    const css = readRel('frontend/app.css');
    const overlays = [
      '#comment-dialog',
      '#kb-comment-dialog',
      '#comment-delete-dialog',
      '#notes-cat-edit-dialog',
      '#todo-task-dialog',
      '#read-later-dialog',
      '#knowledge-tree-delete-dialog',
    ];
    for (const sel of overlays) {
      expect(css).toMatch(
        new RegExp(`${escapeRe(sel)}\\s*\\{[^}]*background:\\s*var\\(--bg-rgba-000000-55\\)`),
      );
    }
    expect(css).toMatch(
      /\.home-chat-delete-confirm-backdrop\s*\{[^}]*background:\s*var\(--bg-rgba-000000-55\)/,
    );
    expect(css).toMatch(
      /\.todo-task-comment-delete-confirm-backdrop\s*\{[^}]*background:\s*var\(--bg-rgba-000000-55\)/,
    );
    expect(css).toMatch(
      /\.todo-task-attachment-delete-confirm-backdrop\s*\{[^}]*background:\s*var\(--bg-rgba-000000-55\)/,
    );
    expect(css).not.toMatch(/--bg-rgba-000000-45/);
    expect(css).not.toMatch(/--bg-rgba-1f2328-45/);
  });

  it('lifts fullscreen cards with the Settings ring', () => {
    const css = readRel('frontend/app.css');
    const cards = [
      '#settings-dialog-box',
      '#file-popup .file-popup-box',
      '#comment-dialog-box',
      '#read-later-dialog-box',
      '.home-chat-delete-confirm-panel',
    ];
    for (const sel of cards) {
      expect(css).toMatch(
        new RegExp(
          `${escapeRe(sel)}\\s*\\{[^}]*border-radius:\\s*8px[^}]*box-shadow:\\s*${escapeRe(SETTINGS_SHADOW)}`,
        ),
      );
    }
  });

  it('does not rewrite Light or Dark overlay token values', () => {
    const light = readRel('frontend/theme-light.css');
    const dark = readRel('frontend/theme-dark.css');
    expect(light).toMatch(/--bg-rgba-000000-55:\s*rgba\(0, 0, 0, 0\.55\)/);
    expect(dark).toMatch(/--bg-rgba-000000-55:\s*rgba\(0, 0, 0, 0\.63\)/);
  });
});
