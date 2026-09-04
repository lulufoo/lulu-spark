(() => {
  let translator = null;
  let running = false;

  function writeResult(report) {
    document.documentElement.setAttribute("data-xzh-result", JSON.stringify(report));
  }

  async function getTranslator(report) {
    if (translator) return translator;
    if (typeof Translator === "undefined") {
      throw new Error("Translator missing");
    }
    const availability = await Translator.availability({
      sourceLanguage: "zh",
      targetLanguage: "en",
    });
    report.availability = availability;
    if (availability === "unavailable") {
      throw new Error("unavailable");
    }
    translator = await Translator.create({
      sourceLanguage: "zh",
      targetLanguage: "en",
      monitor(monitor) {
        monitor.addEventListener("downloadprogress", (event) => {
          report.status = "downloading";
          report.progress = String(event.loaded);
          writeResult(report);
        });
      },
    });
    return translator;
  }

  async function run() {
    if (running) return;
    running = true;
    const source = document.documentElement.getAttribute("data-xzh-src") || "";
    const token = document.documentElement.getAttribute("data-xzh-cmd-token") || "";
    const report = {
      token,
      hasApi: typeof Translator !== "undefined",
      availability: "",
      status: "start",
      progress: "",
      output: "",
      error: "",
    };
    writeResult(report);
    try {
      report.status = "creating";
      writeResult(report);
      const engine = await getTranslator(report);
      report.status = "translating";
      writeResult(report);
      report.output = await engine.translate(source);
      report.status = "done";
      writeResult(report);
    } catch (err) {
      translator = null;
      report.status = "error";
      report.error = String(err && err.message ? err.message : err);
      writeResult(report);
    } finally {
      running = false;
    }
  }

  const observer = new MutationObserver(() => {
    if (!document.documentElement.getAttribute("data-xzh-cmd")) return;
    document.documentElement.removeAttribute("data-xzh-cmd");
    run();
  });
  observer.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["data-xzh-cmd"],
  });
})();
