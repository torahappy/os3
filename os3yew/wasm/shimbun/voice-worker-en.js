import espeak from "./espeak-ng-slim.js";

import * as ort from "./node_modules/onnxruntime-web/dist/ort.min.mjs";

espeak().then((emscripten_functions) => {
  console.log("espeak initialized");

  const HEAPU8 = emscripten_functions.HEAPU8;
  const HEAPU32 = emscripten_functions.HEAPU32;
  const UTF8ToString = emscripten_functions.UTF8ToString;
  const _malloc = emscripten_functions._malloc;
  const _espeak_Initialize = emscripten_functions._espeak_Initialize;
  const _espeak_SetVoiceByName = emscripten_functions._espeak_SetVoiceByName;
  const _free = emscripten_functions._free;
  const _espeak_TextToPhonemesWithTerminator =
    emscripten_functions._espeak_TextToPhonemesWithTerminator;
  const WORKER_DATA = {};

  self.WORKER_DATA = WORKER_DATA;
  self.WORKER_DATA.emscripten_functions = emscripten_functions;

  self.postMessage({
    type: "init_espeak_1",
    data: null,
  });

  init();

  self.onmessage = async function (e) {
    if (e.data.type === "runInference") {
      let result = await runInference(e.data.data);
      self.postMessage({
        type: "runInference_result",
        data: result.output.cpuData,
	relation: e.data.relation
      });
    }
  };

  function init_consts() {
    let c = {};
    c["CLAUSE_PAUSE"] = 0x00000FFF; // pause (x 10mS)
    c["CLAUSE_INTONATION_TYPE"] = 0x00007000; // intonation type
    c["CLAUSE_OPTIONAL_SPACE_AFTER"] = 0x00008000; // don't need space after the punctuation
    c["CLAUSE_TYPE"] = 0x000F0000; // phrase type
    c["CLAUSE_PUNCTUATION_IN_WORD"] = 0x00100000; // punctuation character can be inside a word (Armenian)
    c["CLAUSE_SPEAK_PUNCTUATION_NAME"] = 0x00200000; // speak the name of the punctuation character
    c["CLAUSE_DOT_AFTER_LAST_WORD"] = 0x00400000; // dot after the last word
    c["CLAUSE_PAUSE_LONG"] = 0x00800000; // x 320mS to the CLAUSE_PAUSE value

    c["CLAUSE_INTONATION_FULL_STOP"] = 0x00000000;
    c["CLAUSE_INTONATION_COMMA"] = 0x00001000;
    c["CLAUSE_INTONATION_QUESTION"] = 0x00002000;
    c["CLAUSE_INTONATION_EXCLAMATION"] = 0x00003000;
    c["CLAUSE_INTONATION_NONE"] = 0x00004000;

    c["CLAUSE_TYPE_NONE"] = 0x00000000;
    c["CLAUSE_TYPE_EOF"] = 0x00010000;
    c["CLAUSE_TYPE_VOICE_CHANGE"] = 0x00020000;
    c["CLAUSE_TYPE_CLAUSE"] = 0x00040000;
    c["CLAUSE_TYPE_SENTENCE"] = 0x00080000;

    c["CLAUSE_NONE"] = 0 | c["CLAUSE_INTONATION_NONE"] | c["CLAUSE_TYPE_NONE"];
    c["CLAUSE_PARAGRAPH"] = 70 | c["CLAUSE_INTONATION_FULL_STOP"] |
      c["CLAUSE_TYPE_SENTENCE"];
    c["CLAUSE_EOF"] = 40 | c["CLAUSE_INTONATION_FULL_STOP"] |
      c["CLAUSE_TYPE_SENTENCE"] | c["CLAUSE_TYPE_EOF"];
    c["CLAUSE_VOICE"] = 0 | c["CLAUSE_INTONATION_NONE"] |
      c["CLAUSE_TYPE_VOICE_CHANGE"];
    c["CLAUSE_PERIOD"] = 40 | c["CLAUSE_INTONATION_FULL_STOP"] |
      c["CLAUSE_TYPE_SENTENCE"];
    c["CLAUSE_COMMA"] = 20 | c["CLAUSE_INTONATION_COMMA"] |
      c["CLAUSE_TYPE_CLAUSE"];
    c["CLAUSE_SHORTCOMMA"] = 4 | c["CLAUSE_INTONATION_COMMA"] |
      c["CLAUSE_TYPE_CLAUSE"];
    c["CLAUSE_SHORTFALL"] = 4 | c["CLAUSE_INTONATION_FULL_STOP"] |
      c["CLAUSE_TYPE_CLAUSE"];
    c["CLAUSE_QUESTION"] = 40 | c["CLAUSE_INTONATION_QUESTION"] |
      c["CLAUSE_TYPE_SENTENCE"];
    c["CLAUSE_EXCLAMATION"] = 45 | c["CLAUSE_INTONATION_EXCLAMATION"] |
      c["CLAUSE_TYPE_SENTENCE"];
    c["CLAUSE_COLON"] = 30 | c["CLAUSE_INTONATION_FULL_STOP"] |
      c["CLAUSE_TYPE_CLAUSE"];
    c["CLAUSE_SEMICOLON"] = 30 | c["CLAUSE_INTONATION_COMMA"] |
      c["CLAUSE_TYPE_CLAUSE"];

    WORKER_DATA.constants = c;
  }

  function make_str(s) {
    let buf = new TextEncoder().encode(s);
    let arr = new Uint8Array(buf.length + 1);
    arr.set(buf, 0);
    let pos = _malloc(arr.length);
    HEAPU8.set(arr, pos);
    return pos;
  }

  async function init() {
    init_consts();
    // init espeak-ng
    let datadir = make_str(
      "/usr/share/espeak-ng-data",
    );
    console.log("init:", _espeak_Initialize(2, 0, datadir, 0));
    let voice = make_str("en-us");
    console.log("set voice:", _espeak_SetVoiceByName(voice));
    _free(datadir);
    _free(voice);

    self.postMessage({
      type: "init_espeak_2",
      data: null,
    });

    // init ort
    WORKER_DATA.inference_session = await ort.InferenceSession.create(
      "en_US-libritts_r-medium.onnx",
      {
        executionProviders: ["wasm"],
        graphOptimizationLevel: "extended",
        enableMemPattern: true,
      },
    );
    WORKER_DATA.inference_config =
      await (await fetch("en_US-libritts_r-medium.onnx.json")).json();

    self.postMessage({ type: "init_ort", data: null });
  }

  function getPhonemes_pre(s_in) {
    let text = make_str(s_in);
    let term = _malloc(4);
    let p_text = _malloc(4);
    HEAPU32.set(new Uint32Array([text]), p_text >> 2);
    let ret_js = [];
    while (true) {
      HEAPU32.set(new Uint32Array([0]), term >> 2);
      let ret = _espeak_TextToPhonemesWithTerminator(p_text, 8, 2, term);
      ret_js.push([UTF8ToString(ret, 1000000), HEAPU32[term >> 2]]);
      if (HEAPU32[p_text >> 2] === 0) {
        break;
      }
      _free(ret);
    }
    _free(text);
    _free(term);
    _free(p_text);
    return ret_js;
  }

  function getPhonemes(s_in) {
    let pre = getPhonemes_pre(s_in);
    let ret = "";
    for (let i = 0; i < pre.length; i++) {
      let segment = "";
      let phonemes_str = pre[i][0];
      let punctuation = pre[i][1] & 0x000FFFFF;
      let terminator_str = "";
      switch (punctuation) {
        case WORKER_DATA.constants.CLAUSE_PERIOD:
          terminator_str = ".";
          break;
        case WORKER_DATA.constants.CLAUSE_QUESTION:
          terminator_str = "?";
          break;
        case WORKER_DATA.constants.CLAUSE_EXCLAMATION:
          terminator_str = "!";
          break;
        case WORKER_DATA.constants.CLAUSE_COMMA:
          terminator_str = ",";
          break;
        case WORKER_DATA.constants.CLAUSE_COLON:
          terminator_str = ":";
          break;
        case WORKER_DATA.constants.CLAUSE_SEMICOLON:
          terminator_str = ";";
          break;
      }
      if (
        terminator_str === "," || terminator_str === ":" ||
        terminator_str === ";"
      ) {
        segment = phonemes_str + terminator_str + " ";
      } else {
        segment = phonemes_str + terminator_str;
      }
      ret += Array.from(segment.normalize("NFD")).join("_") + "_"
    }
    console.log(ret);
    return "^_" + ret + "$";
  }

  function runInference(s_in) {
    let phonemes = getPhonemes(s_in);
    let phonemeIds = Array.from(phonemes).map((x) =>
      WORKER_DATA.inference_config.phoneme_id_map[x][0]
    );

    let inputTensor = new ort.Tensor(
      "int64",
      new BigInt64Array(phonemeIds.map((id) => BigInt(id))),
      [1, phonemeIds.length],
    );

    let lengthTensor = new ort.Tensor(
      "int64",
      new BigInt64Array([BigInt(phonemeIds.length)]),
      [1],
    );
    let scalesTensor = new ort.Tensor(
      "float32",
      new Float32Array([
        WORKER_DATA.inference_config.inference.noise_scale || 0.667,
        WORKER_DATA.inference_config.inference.length_scale || 1.0,
        WORKER_DATA.inference_config.inference.noise_w || 0.8,
      ]),
      [3],
    );
    let sidTensor = new ort.Tensor("int64", new BigInt64Array([BigInt(0)]), [
      1,
    ]);

    const feeds = {
      "input": inputTensor,
      "input_lengths": lengthTensor,
      "scales": scalesTensor,
      "sid": sidTensor,
    };
    let result = WORKER_DATA.inference_session.run(
      feeds,
    );
    return result;
  }
});
