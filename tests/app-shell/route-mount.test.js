// @vitest-environment jsdom
import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect } from 'vitest';
import { readMainSource } from '../helpers/read-frontend-js.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const mainJs = readMainSource();

describe('main.js route mount integration', () => {
  it('has no per-route search chrome: the inline search wrap is gone, the modal follows the route itself', () => {
    expect(existsSync(join(fixtureRoot, 'frontend/src/app-shell/ui/nav-chrome.ts'))).toBe(false);
    expect(mainJs).not.toMatch(/nav-chrome/);
    expect(mainJs).not.toMatch(/applySearchNavChrome|updateNavChrome/);
    expect(mainJs).not.toMatch(/btn-nav-home-title/);
    expect(mainJs).not.toMatch(/classList\.toggle\('is-back', !onHome\)/);
  });

  it('removes initGlobalSearch import and init call', () => {
    expect(mainJs).not.toMatch(/initGlobalSearch/);
  });

  it('keeps wrapRouteMount and home hub default landing', () => {
    expect(mainJs).toMatch(/wrapRouteMount/);
    expect(mainJs).toMatch(/HomePage/);
    expect(mainJs).toMatch(/['"]#\/home['"]/);
  });
});
