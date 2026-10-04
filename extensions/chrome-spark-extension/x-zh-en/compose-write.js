self.Xzh = self.Xzh || {};

Xzh.selectAll = function selectAll(editor) {
  const selection = window.getSelection();
  if (!selection) return;
  const range = document.createRange();
  range.selectNodeContents(editor);
  selection.removeAllRanges();
  selection.addRange(range);
};

Xzh.tryInsertText = async function tryInsertText(editor, text) {
  editor.focus();
  Xzh.selectAll(editor);
  const ok = document.execCommand("insertText", false, text);
  await Xzh.waitMs(80);
  return Boolean(ok) && Xzh.readEditor(editor) === Xzh.normalizeText(text);
};

Xzh.tryInputEvent = async function tryInputEvent(editor, text) {
  editor.focus();
  Xzh.selectAll(editor);
  editor.textContent = text;
  editor.dispatchEvent(
    new InputEvent("beforeinput", {
      inputType: "insertText",
      data: text,
      bubbles: true,
      cancelable: true,
    }),
  );
  editor.dispatchEvent(
    new InputEvent("input", {
      inputType: "insertText",
      data: text,
      bubbles: true,
      cancelable: false,
    }),
  );
  await Xzh.waitMs(80);
  return Xzh.readEditor(editor) === Xzh.normalizeText(text);
};

Xzh.tryPaste = async function tryPaste(editor, text) {
  editor.focus();
  Xzh.selectAll(editor);
  const data = new DataTransfer();
  data.setData("text/plain", text);
  editor.dispatchEvent(
    new ClipboardEvent("paste", {
      clipboardData: data,
      bubbles: true,
      cancelable: true,
    }),
  );
  await Xzh.waitMs(80);
  return Xzh.readEditor(editor) === Xzh.normalizeText(text);
};

Xzh.writeNamed = async function writeNamed(name, editor, text) {
  if (name === "insertText") return Xzh.tryInsertText(editor, text);
  if (name === "keyboard") return Xzh.tryKeyboard(editor, text);
  if (name === "inputEvent") return Xzh.tryInputEvent(editor, text);
  return Xzh.tryPaste(editor, text);
};

Xzh.replaceText = async function replaceText(editor, text, postFinder) {
  const order = ["insertText", "keyboard", "inputEvent", "paste"];
  let fallback = "";
  for (const name of order) {
    const wrote = await Xzh.writeNamed(name, editor, text);
    if (!wrote) continue;
    await Xzh.waitMs(200);
    const post = postFinder ? postFinder() : null;
    if (post && Xzh.isPostEnabled(post)) return name;
    if (!fallback) fallback = name;
  }
  return fallback;
};

Xzh.tryKeyboard = async function tryKeyboard(editor, text) {
  editor.focus();
  Xzh.selectAll(editor);
  document.execCommand("delete", false, null);
  for (const char of text) {
    editor.dispatchEvent(
      new InputEvent("beforeinput", {
        inputType: "insertText",
        data: char,
        bubbles: true,
        cancelable: true,
      }),
    );
    document.execCommand("insertText", false, char);
  }
  await Xzh.waitMs(80);
  return Xzh.readEditor(editor) === Xzh.normalizeText(text);
};
