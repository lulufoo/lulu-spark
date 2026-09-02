import { save } from './lib/readLaterApi.js';
import { badgeFeedbackForResult } from './lib/feedback.js';

const FEEDBACK_CLEAR_MS = 3000;

async function applyFeedback(result) {
  const { badgeText, title, badgeColor } = badgeFeedbackForResult(result);
  await chrome.action.setBadgeText({ text: badgeText });
  await chrome.action.setBadgeBackgroundColor({ color: badgeColor });
  await chrome.action.setTitle({ title });

  setTimeout(() => {
    chrome.action.setBadgeText({ text: '' });
    chrome.action.setTitle({ title: 'Save to Workbench Read Later' });
  }, FEEDBACK_CLEAR_MS);
}

chrome.action.onClicked.addListener(async (tab) => {
  const url = tab?.url;
  const title = tab?.title || '';

  if (!url) {
    await applyFeedback({
      ok: false,
      status: 0,
      error: 'No active tab URL',
    });
    return;
  }

  const result = await save({ url, title });
  await applyFeedback(result);
});
