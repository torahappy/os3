const binaryData = {};

function forgetBinary(path){
  delete binaryData[path]
}

async function getBinary(path) {
  if (binaryData[path] === undefined) {
    const f = await fetch(path);
    const b = await f.arrayBuffer();
    const a = new Uint8Array(b);
    binaryData[path] = a;
    return a;
  } else {
    return binaryData[path];
  }
}

function stringToBase64(s) {
  return new TextEncoder().encode(s).toBase64();
}

/**
 * Helper function to escape text for inclusion in XML/SVG.
 * This replaces the Jinja2 '|e' filter.
 * @param {string} str The string to escape.
 * @returns {string} The escaped string.
 */
function escapeXML(str) {
  if (typeof str !== "string") return "";
  return str.replace(/[<>&"']/g, function (match) {
    switch (match) {
      case "<":
        return "&lt;";
      case ">":
        return "&gt;";
      case "&":
        return "&amp;";
      case '"':
        return "&quot;";
      case "'":
        return "&#39;"; // Use &#39; as &apos; is not universally supported
      default:
        return match;
    }
  });
}

/**
 * Generates an SVG string from a page object.
 * @param {Object} page - The page data object (with width, height, viewbox, texts).
 * @param {Uint8Array} fontData - The font data
 * @returns {string} An SVG string.
 */
function genSVG(page, fontData) {
  const textElements = page.texts
    .map((t) => {
      // Calculate the transform matrix components
      const inter_x = -t.mult_x * t.x + t.x;
      const inter_y = t.target_height - t.y * t.mult_y + t.y;

      return `
    <text transform="matrix(${t.mult_x} 0 0 ${t.mult_y} ${inter_x} ${inter_y})" x="${t.x}" y="${t.y}" dominant-baseline="text-bottom" font-size="${t.font_size}" >${escapeXML(t.text)}</text>
    <rect x="${t.x}" y="${t.y}" width="${t.target_width}" height="${t.target_height}" ></rect>
  `;
    })
    .join("\n");

  const svgString = `
<svg width="${page.width}" height="${page.height}" viewBox="${page.viewbox.join(" ")}" xmlns="http://www.w3.org/2000/svg">
  <style>
    @font-face {
      font-family: "hogehoge";
      src:
	url("data:application/x-font-ttf;base64,${fontData.toBase64()}") format("truetype");
    }
    text {
      font-family: "hogehoge", "Noto Serif JP", serif;
    }
    rect {
      stroke: red;
      stroke-width: 1;
      fill: none;
    }
  </style>
  ${textElements}
</svg>
`;

  return stringToBase64(svgString.trim()); // Trim whitespace from start/end
}
const layoutConfig = {
  method: "harfbuzz",
  font: "CoralPixels-Regular.ttf",
  layout_plan: 1,
  metrics: undefined
};

function debugLog(data) {
  outputBox.innerHTML += data;
}

function getFileContents(file) {
  return new Promise((res, rej) => {
    const reader = new FileReader();
    reader.onload = (e) => {
      const buf = reader.result;
      res(buf);
    };
    reader.onerror = (e) => {
      rej(e);
    };
    reader.readAsArrayBuffer(file);
  });
}

function getImageMetrics(buf) {
  const meta = ExifReader.load(buf);
  if (
    meta["Image Width"] === undefined ||
    !Number.isFinite(meta["Image Width"].value) ||
    meta["Image Height"] === undefined ||
    !Number.isFinite(meta["Image Height"].value)
  ) {
    debugLog({ message: "error", data: "Invalid Image Data" });
    return undefined;
  }
  const width = meta["Image Width"].value;
  const height = meta["Image Height"].value;
  let dpi;
  if (
    meta["Pixel Units"] === undefined ||
    !Number.isFinite(meta["Pixel Units"].value) ||
    meta["Pixels Per Unit X"] === undefined ||
    !Number.isFinite(meta["Pixels Per Unit X"].value)
  ) {
    if (
      !(
        meta["ResolutionUnit"] === undefined ||
        meta["XResolution"] === undefined
      ) &&
      !isNaN(parseInt(meta["XResolution"].description))
    ) {
      if (meta["ResolutionUnit"].description === "inches") {
        dpi = parseInt(meta["XResolution"].description);
      } else if (meta["ResolutionUnit"].description === "cm") {
        dpi = parseInt(meta["XResolution"].description) * 2.54;
      }
    }
  } else if (meta["Pixel Units"].description === "meters") {
    dpi =
      meta["Pixels Per Unit X"].value / (meta["Pixel Units"].value * 39.37008);
  }
  if (dpi === undefined) {
    debugLog({
      message: "info",
      data: "Image DPI calculation failed. Fallback to 72dpi.",
    });
    dpi = 72;
  }
  return { width, height, dpi };
}

document.getElementById("ocr_input").onchange = async (e) => {
  const fileInput = document.getElementById("ocr_input");
  const file = fileInput.files[0];
  if (!file) {
    return;
  }
  const buf = await getFileContents(file);
  const metrics = getImageMetrics(buf);
  if (metrics === undefined) {
    return;
  }
  layoutConfig.metrics = metrics;
  document.getElementById("inputBox").remove();
  const myWorkerTesseract = new Worker("./myworker_tesseract.js");
  myWorkerTesseract.addEventListener("message", (e) => {
    if (e.data.message === "done") {
      debugLog(e.data.data);
      myWorkerTesseract.terminate();
      LayoutStage(e.data.data);
    } else {
      debugLog(JSON.stringify(e.data) + "\n");
    }
  });
  myWorkerTesseract.postMessage({
    message: "img in",
    data: buf,
    filename: file.name,
  });
};

async function LayoutStage(ocr_result) {
  const fontData = await getBinary(layoutConfig.font);
  if (layoutConfig.method === "harfbuzz") {
    const myWorkerHarfbuzz = new Worker("./myworker_harfbuzz.js");
    myWorkerHarfbuzz.postMessage({
      message: "init",
      ocr_result,
      config: layoutConfig,
      font_data: fontData,
    });
    forgetBinary(layoutConfig.font);
    myWorkerHarfbuzz.addEventListener("message", (e) => {
      if (e.data.message === "done") {
        const elem = document.createElement("img");
        elem.src = "data:image/svg+xml;base64," + genSVG(e.data.data, fontData);
        document.body.appendChild(elem);
        myWorkerHarfbuzz.terminate();
      } else {
        debugLog(JSON.stringify(e.data) + "\n");
      }
    });
  }
}
