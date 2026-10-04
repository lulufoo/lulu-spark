self.Xzh = self.Xzh || {};

Xzh.BTN_ATTR = "data-xzh-btn";

Xzh.normalizeText = function normalizeText(value) {
  return String(value || "")
    .replace(/[\u200B-\u200D\uFEFF]/g, "")
    .replace(/\s+/g, " ")
    .trim();
};

Xzh.waitMs = function waitMs(ms) {
  return new Promise((resolve) => {
    window.setTimeout(resolve, ms);
  });
};

Xzh.isPlaceholderNode = function isPlaceholderNode(node) {
  const el = node.nodeType === Node.ELEMENT_NODE ? node : node.parentElement;
  if (!el || !(el instanceof Element)) return false;
  if (el.closest(".public-DraftEditorPlaceholder-root")) return true;
  if (el.closest(".public-DraftEditorPlaceholder-inner")) return true;
  const testid = el.closest("[data-testid]")?.getAttribute("data-testid") || "";
  return /placeholder/i.test(testid);
};

Xzh.readUserText = function readUserText(root) {
  if (!root) return "";
  const parts = [];
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
    acceptNode(node) {
      return Xzh.isPlaceholderNode(node) ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT;
    },
  });
  let node = walker.nextNode();
  while (node) {
    parts.push(node.nodeValue || "");
    node = walker.nextNode();
  }
  return Xzh.normalizeText(parts.join(""));
};

Xzh.hintText = function hintText(editor, target) {
  const scope = editor.closest('[data-testid^="tweetTextarea"]') || editor;
  const node =
    scope.querySelector(".public-DraftEditorPlaceholder-inner") ||
    scope.querySelector(".public-DraftEditorPlaceholder-root");
  if (node) return Xzh.normalizeText(node.innerText || node.textContent || "");
  const aria =
    (target && target.getAttribute("aria-placeholder")) ||
    editor.getAttribute("aria-placeholder") ||
    "";
  return Xzh.normalizeText(aria);
};

Xzh.readEditor = function readEditor(editor) {
  const target = Xzh.findWriteTarget(editor);
  const text = Xzh.readUserText(target);
  if (!text) return "";
  const hint = Xzh.hintText(editor, target);
  if (hint && text === hint) return "";
  return text;
};

Xzh.isVisible = function isVisible(el) {
  const box = el.getBoundingClientRect();
  return box.width > 8 && box.height > 8;
};

Xzh.classifySurface = function classifySurface(editor) {
  if (editor.closest('[role="dialog"]')) return "reply-dialog";
  if (editor.closest("article")) return "reply-inline";
  return "tweet-compose";
};

Xzh.findEditors = function findEditors(root) {
  const nodes = [
    ...root.querySelectorAll('[data-testid="tweetTextarea_0"]'),
    ...root.querySelectorAll('[data-testid^="tweetTextarea_"]'),
  ];
  return [...new Set(nodes)].filter(Xzh.isVisible);
};

Xzh.SURFACES = ["tweet-compose", "reply-dialog"];

Xzh.pickComposeEditors = function pickComposeEditors(root) {
  const bySurface = new Map();
  for (const editor of Xzh.findEditors(root)) {
    const surface = Xzh.classifySurface(editor);
    if (!Xzh.SURFACES.includes(surface)) continue;
    const box = editor.getBoundingClientRect();
    const area = box.width * box.height;
    const prev = bySurface.get(surface);
    if (!prev || area > prev.area) bySurface.set(surface, { editor, area });
  }
  return [...bySurface.entries()].map(([surface, item]) => ({
    surface,
    editor: item.editor,
  }));
};

Xzh.findWriteTarget = function findWriteTarget(editor) {
  const inners = [...editor.querySelectorAll('[contenteditable="true"]')].filter(Xzh.isVisible);
  if (inners.length) return inners[inners.length - 1];
  if (editor.getAttribute("contenteditable") === "true") return editor;
  return editor.querySelector('[role="textbox"]') || editor;
};

Xzh.findMount = function findMount(editor) {
  const dialog = editor.closest('[role="dialog"]');
  if (dialog) {
    const toolbar = dialog.querySelector('[data-testid="toolBar"]');
    if (toolbar) return { host: toolbar, via: "toolBar-dialog" };
  }
  let node = editor;
  for (let depth = 0; depth < 16 && node.parentElement; depth += 1) {
    const parent = node.parentElement;
    if (dialog && !dialog.contains(parent)) break;
    for (const child of parent.children) {
      if (child.contains(editor)) continue;
      if (child.getAttribute("data-testid") === "toolBar") {
        return { host: child, via: "toolBar-sibling" };
      }
      const toolbar = child.querySelector('[data-testid="toolBar"]');
      if (toolbar) return { host: toolbar, via: "toolBar-cousin" };
    }
    node = parent;
    if (dialog && node === dialog) break;
  }
  return editor.parentElement
    ? { host: editor.parentElement, via: "editor-parent" }
    : null;
};

Xzh.controlLabel = function controlLabel(el) {
  return Xzh.normalizeText(
    `${el.getAttribute("data-testid") || ""} ${el.getAttribute("aria-label") || ""} ${el.textContent || ""}`,
  );
};

Xzh.composeRoot = function composeRoot(editor) {
  return editor.closest('[role="dialog"]') || document;
};

Xzh.isPostControl = function isPostControl(el, allowReplyLabel) {
  const testid = el.getAttribute("data-testid") || "";
  if (testid === "tweetButton" || testid === "tweetButtonInline") return true;
  const label = Xzh.controlLabel(el);
  if (/可以回复|can reply|who can reply/i.test(label)) return false;
  if (/发帖|发布/.test(label)) return true;
  if (/^post$/i.test(el.getAttribute("aria-label") || "")) return true;
  if (!allowReplyLabel) return false;
  const short = Xzh.normalizeText(el.getAttribute("aria-label") || el.textContent || "");
  return /^(回复|Reply)$/i.test(short);
};

Xzh.findPostButton = function findPostButton(editor) {
  const root = Xzh.composeRoot(editor);
  const allowReplyLabel = root !== document;
  const nodes = [
    ...root.querySelectorAll('[data-testid="tweetButtonInline"]'),
    ...root.querySelectorAll('[data-testid="tweetButton"]'),
    ...root.querySelectorAll('[role="button"]'),
  ];
  const kept = [...new Set(nodes)].filter((el) => Xzh.isPostControl(el, allowReplyLabel));
  if (kept.length === 0) return null;
  const testid = kept.filter((el) => {
    const id = el.getAttribute("data-testid") || "";
    return id === "tweetButton" || id === "tweetButtonInline";
  });
  const pool = testid.length ? testid : kept;
  const editorBottom = editor.getBoundingClientRect().bottom;
  pool.sort((left, right) => {
    const a = Math.abs(left.getBoundingClientRect().top - editorBottom);
    const b = Math.abs(right.getBoundingClientRect().top - editorBottom);
    return a - b;
  });
  return pool[0];
};

Xzh.describePost = function describePost(button) {
  if (!button) return "not-found";
  return (
    button.getAttribute("data-testid") ||
    button.getAttribute("aria-label") ||
    Xzh.normalizeText(button.textContent).slice(0, 24) ||
    "unlabeled"
  );
};

Xzh.isPostEnabled = function isPostEnabled(button) {
  if (!button) return false;
  if (button.getAttribute("aria-disabled") === "true") return false;
  return !button.disabled;
};
