Module = {
  preRun: () => {
    self.postMessage({ message: "info", data: "preRun" });
    try {
      FS.createLazyFile("/", "eng.traineddata", "eng.traineddata", true, false);
      FS.writeFile(MyData.filename, new Uint8Array(MyData.buffer));
      ENV.TESSDATA_PREFIX = "/";
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

MyData = { buffer: undefined, extension: undefined };

self.postMessage({ message: "info", data: "worker start" });
self.addEventListener("message", (e) => {
  if (e.data.message === "img in") {
    MyData.buffer = e.data.data;
    if (e.data.filename.match(/\.jpe?g$/i)) {
      MyData.filename = "in.jpg";
    } else if (e.data.filename.match(/\.png$/i)) {
      MyData.filename = "in.png";
    } else {
      self.postMessage({
        message: "err",
        data: "file extension not supported",
      });
    }
    Module.arguments = [MyData.filename, "out", "-c", "tessedit_create_tsv=1"];
    importScripts("./tesseract.js");
  }
});
