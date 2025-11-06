/**
 * =======================================================================
 * HarfBuzz WASM Module
 * =======================================================================
 */

const HB_DIRECTION_LTR = 4;
const HB_BUFFER_CONTENT_TYPE_UNICODE = 1;

// --- HarfBuzz Helper Functions ---

/**
 * Gets the glyph ID for a single character.
 * @param {string} char - A single character.
 * @returns {number} The glyph ID.
 */
function getGlyph(char) {
  // handle surrogate pair
  if (Array.from(char).length > 2 || char === "") {
    throw new Error("String length must be 1 logical character");
  }

  // Use codePointAt for full Unicode support
  const codepoint = char.codePointAt(0);

  const ret_addr = Module._malloc(4);

  const success = Module._hb_font_get_glyph(
    MyData.font,
    codepoint,
    0,
    ret_addr,
  );

  const ret = Module.getValue(ret_addr, "i32");

  Module._free(ret_addr);

  if (!success) {
    throw new Error(`Bad Character: ${char}`);
  }

  return ret;
}

/**
 * Gets the extents (metrics) for a single character's glyph.
 * @param {string} char - A single character.
 * @returns {Object} An object like { width, height, x_bearing, y_bearing }
 */
function getExtents(char) {
  const extentsPtr = Module._malloc(16);
  const success = Module._hb_font_get_glyph_extents(
    MyData.font,
    getGlyph(char),
    extentsPtr,
  );

  if (!success) {
    Module._free(extentsPtr);
    throw new Error(`Could not get extents for char: ${char}`);
  }

  const ret = {
    width: Module.HEAP32[extentsPtr / 4],
    height: Module.HEAP32[extentsPtr / 4 + 1],
    x_bearing: Module.HEAP32[extentsPtr / 4 + 2],
    y_bearing: -Module.HEAP32[extentsPtr / 4 + 3],
  };

  Module._free(extentsPtr);

  return ret;
}

/**
 * Gets the advance (how much the "pen" moves) for a single character.
 *
 * @param {string} char - A single character.
 * @param {number} [direction] - HarfBuzz direction constant.
 * @returns {Object} An object like { x, y }
 */
function getAdvance(char, direction = HB_DIRECTION_LTR) {
  // 1. Allocate memory for the two out-parameters (x and y)
  const xPtr = Module._malloc(4); // 4 bytes for one int32_t
  const yPtr = Module._malloc(4); // 4 bytes for one int32_t

  let advance;
  try {
    // 2. Call the function
    Module._hb_font_get_glyph_advance_for_direction(
      MyData.font,
      getGlyph(char),
      direction,
      xPtr,
      yPtr,
    );

    // 3. Read the values from the heap
    advance = {
      x: Module.HEAP32[xPtr / 4],
      y: Module.HEAP32[yPtr / 4],
    };
  } finally {
    // 4. Free both pointers
    Module._free(xPtr);
    Module._free(yPtr);
  }
  return advance;
}

/**
 * Shapes a string of characters, returning positioning info for each glyph.
 * @param {string} chars - The string to shape.
 * @param {number} [direction] - HarfBuzz direction constant.
 * @returns {Array<Object>} Array of shape objects: { x_advance, y_advance, x_offset, y_offset }
 */
function getShape(chars, direction = HB_DIRECTION_LTR) {
  const b = Module._hb_buffer_create();

  // 1. Allocate memory for the length out-parameter
  const lengthPtr = Module._malloc(4); // 4 bytes for one unsigned int
  const codepoints = Array.from(chars).map((c) => c.codePointAt(0));
  const codepointsPtr = Module._malloc(codepoints.length * 4);

  const results = [];

  try {
    Module._hb_buffer_set_content_type(b, HB_BUFFER_CONTENT_TYPE_UNICODE);
    Module._hb_buffer_set_direction(b, direction);

    for (let i = 0; i < codepoints.length; i++) {
      Module.HEAP32[codepointsPtr / 4 + i] = codepoints[i]
    }

    Module._hb_buffer_add_codepoints(b, codepointsPtr, codepoints.length, 0, codepoints.length);
    Module._hb_shape(MyData.font, b);

    // 2. Call the function. It returns a pointer to the *start* of the array
    const posArrayPtr = Module._hb_buffer_get_glyph_positions(b, lengthPtr);
    if (posArrayPtr === 0) {
      throw new Error(`Shape Fail: ${codepoints}`);
    }

    // 3. Read the length from the lengthPtr
    const length = Module.HEAPU32[lengthPtr / 4];

    const structSize = 20; // 20 bytes per hb_glyph_position_t

    // 3b. Loop through the array and read each struct
    for (let i = 0; i < length; i++) {
      // Calculate the memory address of the current struct in the array
      const currentStructPtr = posArrayPtr + i * structSize;

      // Get the HEAP32 index for the start of this struct
      const heapIndex = currentStructPtr / 4;

      results.push({
        x_advance: Module.HEAP32[heapIndex + 0],
        y_advance: Module.HEAP32[heapIndex + 1],
        x_offset: Module.HEAP32[heapIndex + 2],
        y_offset: Module.HEAP32[heapIndex + 3],
        // We ignore heapIndex + 4 (the 'var' field)
      });
    }
  } finally {
    // 4. Free the memory *we* allocated (lengthPtr)
    Module._free(lengthPtr);
    Module._free(codepointsPtr);

    // 5. destroying buffer also frees the posArrayPtr
    Module._hb_buffer_destroy(b);
  }

  return results;
}

/**
 * Combines getExtents and getShape for a string.
 * @param {string} chars - The string to process.
 * @param {number} [direction] - HarfBuzz direction constant.
 * @returns {Array<Array<Object>>} An array of [extents, shape] pairs.
 */
function getShapeAndExtents(chars, direction = Module.HB_DIRECTION_LTR) {
  const extents = Array.from(chars).map((c) => getExtents(c));
  const shapes = getShape(chars, direction);

  // Zip the two arrays together
  return extents.map((e, i) => [e, shapes[i]]);
}

// --- Main Logic Function ---

/**
 * Gets page layout information by combining OCR and font metrics.
 * @param {Object} [imageInfo] - Object with image properties {width, height, dpi}.
 * @param {number} layoutPlan - Layout plan (1 or 2). Default is 1.
 * @returns {Object} the Page object.
 */
function getPageInfo(imageInfo, layoutPlan = 1) {
  const { width: imageWidth, height: imageHeight, dpi } = imageInfo;

  let actualWidth, actualHeight, unit;
  if (dpi) {
    unit = "in";
    actualWidth = imageWidth / dpi;
  } else {
    unit = "cm";
    actualWidth = 21; // Default to A4 width
  }
  actualHeight = actualWidth * (imageHeight / imageWidth);

  const textData = [];
  const ocrResultsLines = MyData.ocr_result.split('\n');
  const ocrResults = [];

  for (let i = 1; i < ocrResultsLines.length; i++) {
    const data = ocrResultsLines[i].split('\t')
    if (data.length === 12 && data[11] !== '') {
    ocrResults.push({
      left: parseFloat(data[6]),
      top: parseFloat(data[7]),
      width: parseFloat(data[8]),
      height: parseFloat(data[9]),
      confidence: parseFloat(data[10]),
      text: data[11]
    })
    }
  }

  for (const result of ocrResults) {
    const target_height = result.height;
    const target_width = result.width;

    // Skip empty text results
    if (!result.text || result.text.trim() === "") continue;

    const se = getShapeAndExtents(result.text); // [ [extents, shape], ... ]

    // total text width (sum of advances)
    const hbWidth = se.reduce((sum, [ext, shape]) => sum + shape.x_advance, 0);
    // total text height (max of bearings)
    const hbHeight = Math.max(...se.map(([ext, shape]) => ext.y_bearing));

    if (hbHeight === 0 || hbWidth === 0) {
      continue; // Skip glyphs with no dimensions
    }

    let font_size, mult_x, mult_y;
    if (layoutPlan === 1) {
      font_size = target_height;
      mult_y = 1024 / hbHeight;
      mult_x = target_width / ((hbWidth / 1024) * font_size);
    } else if (layout_plan === 2) {
      font_size = target_height * (1024 / hb_height);
      mult_y = 1.0;
      mult_x = target_width / ((hbWidth / 1024) * font_size);
    } else {
      throw new Error("invalid layout_plan value");
    }

    textData.push({
      font_size,
      mult_x,
      mult_y,
      x: result.left,
      y: result.top,
      target_width,
      target_height,
      text: result.text,
    });
  }

  // Return Page object
  return {
    width: `${actualWidth}${unit}`,
    height: `${actualHeight}${unit}`,
    viewbox: [0, 0, imageWidth, imageHeight],
    texts: textData,
  };
}

Module = {
  preRun: () => {
    self.postMessage({ message: "info", data: "preRun" });
    try {
      FS.writeFile(MyData.config.font, MyData.font_data)
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
    self.postMessage({ message: "info", data: "postRun" });
    let fontname = Module.intArrayFromString(MyData.config.font, false);
    let fontname_addr = Module._malloc(fontname.length);
    Module.writeArrayToMemory(fontname, fontname_addr);
    MyData.blob = Module._hb_blob_create_from_file(fontname_addr);
    MyData.face = Module._hb_face_create(MyData.blob, 0);
    MyData.font = Module._hb_font_create(MyData.face);
    Module._free(fontname_addr);
    self.postMessage({ message: "info", data: "font loaded" });
    self.postMessage({ message: "done", data: getPageInfo(MyData.config.metrics, MyData.config.layout_plan) });
  },
};

MyData = {
  ocr_result: undefined,
  config: undefined,
  blob: undefined,
  face: undefined,
  font: undefined,
  font_data: undefined
};

self.postMessage({ message: "info", data: "worker start" });
self.addEventListener("message", (e) => {
  if (e.data.message === "init") {
    MyData.ocr_result = e.data.ocr_result;
    MyData.config = e.data.config;
    MyData.font_data = e.data.font_data;
    importScripts("./harfbuzz.js");
  }
});
