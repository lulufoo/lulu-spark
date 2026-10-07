// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { encodePassBag, PASS_QUERY } from '../../frontend/src/auth/pass.ts';
import * as api from '../../frontend/src/host/api.ts';
import * as scheme from '../../frontend/src/router/scheme.ts';
import { handleReadLaterOsNotifyEnvelope } from '../../frontend/src/read-later/commands/os-notify.ts';
import * as toast from '../../frontend/src/toast.tsx';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

function readRel(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
}

const HOP_ID = 'trace_12345678';
const READ_LATER_CREATE = {
  business: 'read_later',
  action: 'create',
  params: { id: HOP_ID, entry_id: 'e1' },
};
const READ_LATER_LIST = `spark://read-later/list?${PASS_QUERY}=${encodePassBag(HOP_ID)}`;

describe('handleReadLaterOsNotifyEnvelope', () => {
  /** @type {import('vitest').MockInstance} */
  let notifySpy;
  /** @type {import('vitest').MockInstance} */
  let composeSpy;
  /** @type {import('vitest').MockInstance} */
  let toastSpy;
  /** @type {import('vitest').MockInstance} */
  let alertSpy;

  beforeEach(() => {
    notifySpy = vi.spyOn(api, 'showOsNotification').mockResolvedValue(undefined);
    composeSpy = vi.spyOn(scheme, 'composeSparkScheme');
    toastSpy = vi.spyOn(toast, 'showToast');
    alertSpy = vi.spyOn(window, 'alert').mockImplementation(() => {});
  });

  afterEach(() => {
    notifySpy.mockRestore();
    composeSpy.mockRestore();
    toastSpy.mockRestore();
    alertSpy.mockRestore();
  });

  it('composes the list scheme and shows Read Later for read_later+create', async () => {
    await handleReadLaterOsNotifyEnvelope(READ_LATER_CREATE);
    expect(composeSpy).toHaveBeenCalledWith(READ_LATER_CREATE);
    expect(notifySpy).toHaveBeenCalledWith({
      title: 'Read Later',
      body: 'A link was saved',
      scheme: READ_LATER_LIST,
    });
  });

  it.each([
    [{ business: 'read_later', action: 'update', params: { id: 'e1' } }],
    [{ business: 'notes', action: 'create', params: { id: 'abc', common_path: 'inbox/x.md' } }],
    [{ business: 'todos', action: 'create', params: { id: 't1' } }],
  ])('returns without notify for %j', async (envelope) => {
    await handleReadLaterOsNotifyEnvelope(envelope);
    expect(composeSpy).not.toHaveBeenCalled();
    expect(notifySpy).not.toHaveBeenCalled();
  });

  it('does not call showOsNotification when compose returns null', async () => {
    composeSpy.mockReturnValue(null);
    await handleReadLaterOsNotifyEnvelope(READ_LATER_CREATE);
    expect(composeSpy).toHaveBeenCalledWith(READ_LATER_CREATE);
    expect(notifySpy).not.toHaveBeenCalled();
  });

  it('swallows showOsNotification rejection without a UI prompt', async () => {
    notifySpy.mockRejectedValue(new Error('not authorized'));
    await expect(handleReadLaterOsNotifyEnvelope(READ_LATER_CREATE)).resolves.toBeUndefined();
    expect(notifySpy).toHaveBeenCalled();
    expect(toastSpy).not.toHaveBeenCalled();
    expect(alertSpy).not.toHaveBeenCalled();
    expect(document.getElementById('react-toast-root')).toBeNull();
  });
});

describe('read-later os-notify import constraints', () => {
  it('imports showOsNotification from host/api and compose from L2 scheme', () => {
    const src = readRel('frontend/src/read-later/commands/os-notify.ts');
    expect(src).toMatch(/from ['"].*host\/api\.ts['"]/);
    expect(src).toMatch(/showOsNotification/);
    expect(src).toMatch(/composeSparkScheme/);
    expect(src).toMatch(/from ['"].*router\/scheme\.ts['"]/);
    expect(src).not.toMatch(/writeApiInvokeMap/);
    expect(src).not.toMatch(/@tauri-apps\//);
  });

  it('boot hub dispatches read-later envelopes without composing in boot.ts', () => {
    const bootSrc = readRel('frontend/src/boot.ts');
    expect(bootSrc).toMatch(/handleReadLaterOsNotifyEnvelope/);
    expect(bootSrc).toMatch(/startOsNotifyHub/);
    expect(bootSrc).not.toMatch(/composeSparkScheme/);
    expect(bootSrc).not.toMatch(/showOsNotification/);
  });
});
