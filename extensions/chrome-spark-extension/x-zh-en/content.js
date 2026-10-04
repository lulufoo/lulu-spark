(() => {
  let scanTimer = 0;

  function createIcon() {
    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("viewBox", "0 0 24 24");
    svg.setAttribute("aria-hidden", "true");
    svg.setAttribute("focusable", "false");
    const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
    path.setAttribute("d", "M4 7 L8 17 L12 9 L16 17 L20 7");
    svg.appendChild(path);
    return svg;
  }

  function findInsertAnchor(host) {
    const buttons = [...host.querySelectorAll('[role="button"], button')].filter((el) => {
      return !el.hasAttribute(Xzh.BTN_ATTR) && Xzh.isVisible(el);
    });
    if (buttons.length === 0) return { parent: host, before: host.firstChild };
    const first = buttons[0];
    let node = first.parentElement;
    while (node && node !== host) {
      const iconKids = [...node.children].filter((child) =>
        child.querySelector('[role="button"], button'),
      );
      if (iconKids.length >= 2) return { parent: node, before: iconKids[0] };
      node = node.parentElement;
    }
    return { parent: first.parentElement || host, before: first };
  }

  function readResult() {
    const raw = document.documentElement.getAttribute("data-xzh-result");
    if (!raw) return null;
    try {
      return JSON.parse(raw);
    } catch {
      return null;
    }
  }

  async function waitTranslate(source) {
    const token = String(Date.now());
    document.documentElement.removeAttribute("data-xzh-result");
    document.documentElement.setAttribute("data-xzh-src", source);
    document.documentElement.setAttribute("data-xzh-cmd-token", token);
    document.documentElement.setAttribute("data-xzh-cmd", token);
    const deadline = Date.now() + 90000;
    while (Date.now() < deadline) {
      const report = readResult();
      if (report && report.token === token) {
        if (report.status === "done" || report.status === "error") return report;
      }
      await Xzh.waitMs(300);
    }
    return { status: "error", error: "timeout 90s", output: "" };
  }

  function setBusy(btn, busy) {
    btn.disabled = busy;
    if (busy) btn.setAttribute("aria-busy", "true");
    else btn.removeAttribute("aria-busy");
  }

  async function runTranslate(editor, btn) {
    const source = Xzh.readEditor(editor);
    if (!source) {
      btn.title = "先输入中文";
      return;
    }
    setBusy(btn, true);
    try {
      const report = await waitTranslate(source);
      if (report.status !== "done" || !Xzh.normalizeText(report.output)) {
        btn.title = report.error || "翻译失败";
        return;
      }
      const english = Xzh.normalizeText(report.output);
      const targets = [...new Set([Xzh.findWriteTarget(editor), editor])];
      let strategy = "";
      let used = targets[0];
      for (const target of targets) {
        const name = await Xzh.replaceText(target, english, () => Xzh.findPostButton(editor));
        if (!name) continue;
        used = target;
        strategy = name;
        if (Xzh.isPostEnabled(Xzh.findPostButton(editor))) break;
      }
      await Xzh.waitMs(250);
      const readBack =
        Xzh.readEditor(editor).includes(english) || Xzh.readEditor(used).includes(english);
      const post = Xzh.findPostButton(editor);
      const enabled = Xzh.isPostEnabled(post);
      if (strategy && readBack && enabled) {
        btn.title = "已译成英文";
        return;
      }
      btn.title = strategy ? "已写入但发送键未亮" : "写入失败";
    } finally {
      setBusy(btn, false);
    }
  }

  function inject(editor) {
    const mount = Xzh.findMount(editor);
    if (!mount || mount.host.querySelector(`[${Xzh.BTN_ATTR}]`)) return;
    const btn = document.createElement("button");
    btn.type = "button";
    btn.setAttribute(Xzh.BTN_ATTR, "1");
    btn.className = "xzh-btn";
    btn.setAttribute("aria-label", "译成英文");
    btn.title = "译成英文";
    btn.appendChild(createIcon());
    btn.addEventListener("click", (event) => {
      event.preventDefault();
      event.stopPropagation();
      runTranslate(editor, btn);
    });
    const anchor = findInsertAnchor(mount.host);
    anchor.parent.insertBefore(btn, anchor.before);
  }

  function scan() {
    for (const item of Xzh.pickComposeEditors(document)) {
      inject(item.editor);
    }
  }

  function scheduleScan() {
    if (scanTimer) return;
    scanTimer = window.setTimeout(() => {
      scanTimer = 0;
      scan();
    }, 400);
  }

  function isOurNode(node) {
    if (!(node instanceof Element)) return false;
    return Boolean(node.closest(".xzh-btn") || node.classList.contains("xzh-btn"));
  }

  const observer = new MutationObserver((records) => {
    for (const record of records) {
      if (isOurNode(record.target)) continue;
      scheduleScan();
      return;
    }
  });
  observer.observe(document.documentElement, { childList: true, subtree: true });
  scan();
})();
