import { useSyncExternalStore } from 'react';
import { OverlayDismissButton } from '../../shared/overlay-dismiss-button.tsx';
import {
  dismissVisibleApproval,
  resolveVisibleApproval,
} from '../commands/approval.ts';
import { approvalStore, visibleApproval } from '../state/approval.ts';
import { getHomeState, subscribeHome } from '../state/store.ts';

function subscribeBoth(onChange: () => void) {
  const unApproval = approvalStore.subscribe(onChange);
  const unHome = subscribeHome(onChange);
  return () => {
    unApproval();
    unHome();
  };
}

function snapshot() {
  return visibleApproval(approvalStore.getSnapshot().queue, getHomeState().currentSessionId);
}

export function ApprovalDialog() {
  const req = useSyncExternalStore(subscribeBoth, snapshot, snapshot);
  const open = Boolean(req);

  return (
    <div
      id="approval-dialog"
      className={open ? 'open' : undefined}
      data-role="approval-dialog"
      data-open={open ? 'true' : 'false'}
      onClick={(e) => {
        if (e.target === e.currentTarget) void dismissVisibleApproval();
      }}
    >
      {req ? (
        <div
          id="approval-dialog-box"
          role="dialog"
          aria-modal="true"
          aria-labelledby="approval-dialog-title"
        >
          <div id="approval-dialog-header">
            <h3 id="approval-dialog-title">需要文件权限</h3>
            <OverlayDismissButton
              id="btn-approval-close"
              label="拒绝"
              onClick={() => {
                void dismissVisibleApproval();
              }}
            />
          </div>
          <p data-role="approval-kind">文件写入或加入 Stage（fs）</p>
          <p data-role="approval-path">{req.path}</p>
          <div id="approval-dialog-actions">
            <button
              type="button"
              data-role="approval-allow-once"
              onClick={() => {
                void resolveVisibleApproval('allow_once');
              }}
            >
              允许一次
            </button>
            <button
              type="button"
              data-role="approval-allow-session"
              onClick={() => {
                void resolveVisibleApproval('allow_session');
              }}
            >
              本会话都允许
            </button>
            <button
              type="button"
              data-role="approval-deny"
              onClick={() => {
                void resolveVisibleApproval('deny');
              }}
            >
              拒绝
            </button>
          </div>
        </div>
      ) : null}
    </div>
  );
}
