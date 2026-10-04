// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import * as api from '../../frontend/src/host/api.ts';
import * as scheme from '../../frontend/src/router/scheme.ts';
import { handleNotesOsNotifyEnvelope } from '../../frontend/src/notes/commands/os-notify.ts';
import * as toast from '../../frontend/src/toast.tsx';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

function readRel(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
}

function extractFunctionBody(source, name) {
  const start = source.indexOf(`function ${name}`);
  if (start === -1) return '';
  const braceStart = source.indexOf('{', start);
  let depth = 0;
  for (let i = braceStart; i < source.length; i += 1) {
    if (source[i] === '{') depth += 1;
    if (source[i] === '}') {
      depth -= 1;
      if (depth === 0) return source.slice(braceStart, i + 1);
    }
  }
  return '';
}

const NOTE_CREATE = {
  business: 'notes',
  action: 'create',
  params: { id: 'abc', common_path: 'inbox/x.md' },
};

describe('handleNotesOsNotifyEnvelope', () => {
  /** @type {import('vitest').MockInstance} */
  let notifySpy;
  /** @type {import('vitest').MockInstance} */
  let composeSpy;
  /** @type {import('vitest').MockInstance} */
  let toastSpy;
  /** @type {import('vitest').MockInstance} */
  let alertSpy;
  /** @type {import('vitest').MockInstance} */
  let fetchSpy;

  beforeEach(() => {
    notifySpy = vi.spyOn(api, 'showOsNotification').mockResolvedValue(undefined);
    composeSpy = vi.spyOn(scheme, 'composeSparkScheme');
    toastSpy = vi.spyOn(toast, 'showToast');
    alertSpy = vi.spyOn(window, 'alert').mockImplementation(() => {});
    fetchSpy = vi.spyOn(api, 'fetchIndex').mockResolvedValue({ entries: {} });
  });

  afterEach(() => {
    notifySpy.mockRestore();
    composeSpy.mockRestore();
    toastSpy.mockRestore();
    alertSpy.mockRestore();
    fetchSpy.mockRestore();
  });

  it('reloads Host index then composes notes/open for notes+create', async () => {
    await handleNotesOsNotifyEnvelope(NOTE_CREATE);
    expect(fetchSpy).toHaveBeenCalled();
    expect(fetchSpy.mock.invocationCallOrder[0]).toBeLessThan(
      notifySpy.mock.invocationCallOrder[0],
    );
    expect(composeSpy).toHaveBeenCalledWith(NOTE_CREATE);
    expect(notifySpy).toHaveBeenCalledWith({
      title: 'New note',
      body: 'A note was added',
      scheme: 'spark://notes/open?id=abc&path=inbox%2Fx.md',
    });
  });

  it('reloads Host index on notes+update without a banner', async () => {
    await handleNotesOsNotifyEnvelope({
      business: 'notes',
      action: 'update',
      params: { id: 'abc', common_path: 'inbox/x.md' },
    });
    expect(fetchSpy).toHaveBeenCalled();
    expect(composeSpy).not.toHaveBeenCalled();
    expect(notifySpy).not.toHaveBeenCalled();
  });

  it.each([
    [{ business: 'read_later', action: 'create', params: { id: 'e1' } }],
    [{ business: 'notes', action: 'changed', params: { id: 'abc', common_path: 'inbox/x.md' } }],
    [{ business: 'todos', action: 'create', params: { id: 't1' } }],
  ])('returns without notify for %j', async (envelope) => {
    await handleNotesOsNotifyEnvelope(envelope);
    expect(fetchSpy).not.toHaveBeenCalled();
    expect(composeSpy).not.toHaveBeenCalled();
    expect(notifySpy).not.toHaveBeenCalled();
  });

  it('does not call showOsNotification when compose returns null', async () => {
    await handleNotesOsNotifyEnvelope({
      business: 'notes',
      action: 'create',
      params: { id: 'abc' },
    });
    expect(composeSpy).toHaveBeenCalled();
    expect(composeSpy.mock.results[0]?.value).toBeNull();
    expect(notifySpy).not.toHaveBeenCalled();
  });

  it('swallows showOsNotification rejection without a UI prompt', async () => {
    notifySpy.mockRejectedValue(new Error('not authorized'));
    await expect(handleNotesOsNotifyEnvelope(NOTE_CREATE)).resolves.toBeUndefined();
    expect(notifySpy).toHaveBeenCalled();
    expect(toastSpy).not.toHaveBeenCalled();
    expect(alertSpy).not.toHaveBeenCalled();
    expect(document.getElementById('react-toast-root')).toBeNull();
  });
});

describe('startOsNotifyHub process-level wiring', () => {
  it('boots a process-level hub that only dispatches the envelope payload', () => {
    const bootSrc = readRel('frontend/src/boot.ts');
    expect(bootSrc).toMatch(/startOsNotifyHub\s*\(\s*\)/);
    expect(bootSrc).toMatch(/handleNotesOsNotifyEnvelope/);
    expect(bootSrc).toMatch(/handleReadLaterOsNotifyEnvelope/);
    const body = extractFunctionBody(bootSrc, 'startOsNotifyHub');
    expect(body).toMatch(/message-center:changed/);
    expect(body).toMatch(/event\.payload|event\?\.payload/);
    expect(body).toMatch(/handleNotesOsNotifyEnvelope/);
    expect(body).toMatch(/handleReadLaterOsNotifyEnvelope/);
    expect(body).not.toMatch(/composeSparkScheme/);
    expect(body).not.toMatch(/showOsNotification/);
    expect(bootSrc).not.toMatch(/writeApiInvokeMap/);
    expect(bootSrc).not.toMatch(/from ['"]\.\/home\/commands\/hub/);
  });

  it('does not hang OS notify on the Home page hub lifecycle', () => {
    const hubSrc = readRel('frontend/src/home/commands/hub.ts');
    const pageSrc = readRel('frontend/src/home/page.tsx');
    const hubBody = extractFunctionBody(hubSrc, 'startHomeHub');
    expect(hubBody).toMatch(/refreshChannelUnread/);
    expect(hubBody).not.toMatch(/showOsNotification/);
    expect(hubSrc).not.toMatch(/showOsNotification/);
    expect(hubSrc).not.toMatch(/startOsNotifyHub/);
    expect(pageSrc).not.toMatch(/showOsNotification/);
    expect(pageSrc).not.toMatch(/startOsNotifyHub/);
  });
});

describe('notes os-notify import constraints', () => {
  it('imports showOsNotification from host/api and compose from L2 scheme', () => {
    const src = readRel('frontend/src/notes/commands/os-notify.ts');
    expect(src).toMatch(/from ['"].*host\/api\.ts['"]/);
    expect(src).toMatch(/showOsNotification/);
    expect(src).toMatch(/composeSparkScheme/);
    expect(src).toMatch(/from ['"].*router\/scheme\.ts['"]/);
    expect(src).not.toMatch(/writeApiInvokeMap/);
    expect(src).not.toMatch(/@tauri-apps\//);
  });
});
