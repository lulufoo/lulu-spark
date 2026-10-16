// @vitest-environment jsdom
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

const invokeMock = vi.hoisted(() => vi.fn());

vi.mock('../../frontend/src/host/api.ts', () => ({
  invoke: (...args) => invokeMock(...args),
}));

function readRel(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
}

const payloadA = {
  request_id: 'apr-1',
  type: 'fs',
  path: '/tmp/a.md',
  session_id: 'spark_chat_a',
};

const payloadB = {
  request_id: 'apr-2',
  type: 'fs',
  path: '/tmp/b.md',
  session_id: 'spark_chat_a',
};

describe('approval · source', () => {
  it('Home page paints the approval dialog', () => {
    const page = readRel('frontend/src/home/page.tsx');
    expect(page).toMatch(/from ['"]\.\/ui\/approval-dialog\.tsx['"]/);
    expect(page).toMatch(/<ApprovalDialog/);
    expect(existsSync(join(repoRoot, 'frontend/src/home/ui/approval-dialog.tsx'))).toBe(true);
  });

  it('boot listens at startup', () => {
    const boot = readRel('frontend/src/boot.ts');
    expect(boot).toMatch(/startApprovalHub/);
    const cmd = readRel('frontend/src/home/commands/approval.ts');
    expect(cmd).toMatch(/ai-assistant:approval-requested/);
    expect(cmd).not.toMatch(/始终允许|always/);
  });
});

describe('approval queue', () => {
  beforeEach(async () => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue({ ok: true });
    const { resetApprovalForTests } = await import(
      '../../frontend/src/home/commands/approval.ts'
    );
    const { resetHomeState, setHomeState, getHomeState } = await import(
      '../../frontend/src/home/state/store.ts'
    );
    resetApprovalForTests();
    resetHomeState();
    setHomeState({ ...getHomeState(), currentSessionId: 'spark_chat_a' });
  });

  it('does not open without an event', async () => {
    const { pendingApproval } = await import(
      '../../frontend/src/home/commands/approval.ts'
    );
    expect(pendingApproval()).toBeNull();
  });

  it('shows the first request after an event', async () => {
    const { enqueueApproval, pendingApproval } = await import(
      '../../frontend/src/home/commands/approval.ts'
    );
    enqueueApproval(payloadA);
    expect(pendingApproval()).toEqual({
      requestId: 'apr-1',
      type: 'fs',
      path: '/tmp/a.md',
      sessionId: 'spark_chat_a',
    });
  });

  it('three buttons invoke the same id', async () => {
    const { enqueueApproval, resolveVisibleApproval } = await import(
      '../../frontend/src/home/commands/approval.ts'
    );
    enqueueApproval(payloadA);
    await resolveVisibleApproval('allow_once');
    expect(invokeMock).toHaveBeenCalledWith('resolve_approval', {
      requestId: 'apr-1',
      choice: 'allow_once',
    });
    enqueueApproval(payloadA);
    await resolveVisibleApproval('allow_session');
    expect(invokeMock).toHaveBeenLastCalledWith('resolve_approval', {
      requestId: 'apr-1',
      choice: 'allow_session',
    });
    enqueueApproval(payloadA);
    await resolveVisibleApproval('deny');
    expect(invokeMock).toHaveBeenLastCalledWith('resolve_approval', {
      requestId: 'apr-1',
      choice: 'deny',
    });
  });

  it('close is deny', async () => {
    const { enqueueApproval, dismissVisibleApproval, pendingApproval } = await import(
      '../../frontend/src/home/commands/approval.ts'
    );
    enqueueApproval(payloadA);
    await dismissVisibleApproval();
    expect(invokeMock).toHaveBeenCalledWith('resolve_approval', {
      requestId: 'apr-1',
      choice: 'deny',
    });
    expect(pendingApproval()).toBeNull();
  });

  it('switching chat hides the dialog and coming back keeps it without invoke', async () => {
    const { enqueueApproval, pendingApproval } = await import(
      '../../frontend/src/home/commands/approval.ts'
    );
    const { setHomeState, getHomeState } = await import(
      '../../frontend/src/home/state/store.ts'
    );
    enqueueApproval(payloadA);
    expect(pendingApproval()?.requestId).toBe('apr-1');
    setHomeState({ ...getHomeState(), currentSessionId: 'spark_chat_b' });
    expect(pendingApproval()).toBeNull();
    expect(invokeMock).not.toHaveBeenCalled();
    setHomeState({ ...getHomeState(), currentSessionId: 'spark_chat_a' });
    expect(pendingApproval()?.requestId).toBe('apr-1');
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it('queues a second request until the first is resolved', async () => {
    const { enqueueApproval, pendingApproval, resolveVisibleApproval } = await import(
      '../../frontend/src/home/commands/approval.ts'
    );
    enqueueApproval(payloadA);
    enqueueApproval(payloadB);
    expect(pendingApproval()?.requestId).toBe('apr-1');
    await resolveVisibleApproval('allow_once');
    expect(pendingApproval()?.requestId).toBe('apr-2');
  });
});

describe('Approval dialog paint', () => {
  let container;
  let root;

  beforeEach(async () => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue({ ok: true });
    const { resetApprovalForTests } = await import(
      '../../frontend/src/home/commands/approval.ts'
    );
    const { resetHomeState, setHomeState, getHomeState } = await import(
      '../../frontend/src/home/state/store.ts'
    );
    resetApprovalForTests();
    resetHomeState();
    setHomeState({ ...getHomeState(), currentSessionId: 'spark_chat_a' });
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
  });

  afterEach(() => {
    act(() => {
      root.unmount();
    });
    container.remove();
  });

  it('paints path and the three Chinese actions after an event', async () => {
    const { enqueueApproval } = await import(
      '../../frontend/src/home/commands/approval.ts'
    );
    const { ApprovalDialog } = await import(
      '../../frontend/src/home/ui/approval-dialog.tsx'
    );
    await act(async () => {
      root.render(createElement(ApprovalDialog));
    });
    expect(container.querySelector('[data-open="true"]')).toBeNull();
    await act(async () => {
      enqueueApproval(payloadA);
    });
    expect(container.querySelector('[data-role="approval-path"]')?.textContent).toBe(
      '/tmp/a.md',
    );
    expect(container.querySelector('[data-role="approval-allow-once"]')?.textContent).toBe(
      '允许一次',
    );
    expect(container.querySelector('[data-role="approval-allow-session"]')?.textContent).toBe(
      '本会话都允许',
    );
    expect(container.querySelector('[data-role="approval-deny"]')?.textContent).toBe(
      '拒绝',
    );
    expect(container.textContent).not.toMatch(/始终允许/);
  });
});
