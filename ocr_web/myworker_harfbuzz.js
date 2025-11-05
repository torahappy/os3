Module = {
  preRun: () => {
    self.postMessage({ message: "info", data: "preRun" });
    try {
      FS.createLazyFile("/", MyData.config.font, MyData.config.font, true, false);
    } catch (e) {
      self.postMessage({ message: "err", data: String(e) });
    }
  },
  print: (m) => {
    self.postMessage({ message: "stdout", data: m });
  },
  printErr: (m) => {
    self.postMessage({ message: "stderr", data: m });
  },
  postRun: () => {
    try {
      let data = UTF8Decoder.decode(FS.readFile("out.tsv"));
      self.postMessage({ message: "done", data });
    } catch (e) {
      self.postMessage({ message: "err", data: String(e) });
    }
  },
};

MyData = { ocr_result: undefined, config: undefined };

self.postMessage({ message: "info", data: "worker start" });
self.addEventListener("message", (e) => {
  if (e.data.message === "init") {
    MyData.ocr_result = e.data.ocr_result;
    MyData.config = e.data.config;
    importScripts("./harfbuzz.js");
  }
});
