import openjtalk from "./openjtalk-slim.js";

openjtalk({
  print: (txt) => {
    WORKER_DATA.output_buffer += txt;
  }
}).then((emscripten_functions) => {
  console.log("openjtalk initialized");

  const UTF8ToString = emscripten_functions.UTF8ToString;
  const _malloc = emscripten_functions._malloc;
  const _free = emscripten_functions._free;
  const _fopen = emscripten_functions._fopen;
  const _fclose = emscripten_functions._fclose;
  const _Open_JTalk_initialize = emscripten_functions._Open_JTalk_initialize;
  const _Open_JTalk_clear = emscripten_functions._Open_JTalk_clear;
  const _Open_JTalk_load = emscripten_functions._Open_JTalk_load;
  const _Open_JTalk_set_sampling_frequency =
      emscripten_functions._Open_JTalk_set_sampling_frequency;
  const _Open_JTalk_set_fperiod = emscripten_functions._Open_JTalk_set_fperiod;
  const _Open_JTalk_set_alpha = emscripten_functions._Open_JTalk_set_alpha;
  const _Open_JTalk_set_beta = emscripten_functions._Open_JTalk_set_beta;
  const _Open_JTalk_set_speed = emscripten_functions._Open_JTalk_set_speed;
  const _Open_JTalk_add_half_tone =
      emscripten_functions._Open_JTalk_add_half_tone;
  const _Open_JTalk_set_msd_threshold =
      emscripten_functions._Open_JTalk_set_msd_threshold;
  const _Open_JTalk_set_gv_weight =
      emscripten_functions._Open_JTalk_set_gv_weight;
  const _Open_JTalk_set_volume = emscripten_functions._Open_JTalk_set_volume;
  const _Open_JTalk_set_audio_buff_size =
      emscripten_functions._Open_JTalk_set_audio_buff_size;
  const _Open_JTalk_synthesis = emscripten_functions._Open_JTalk_synthesis;
  const FS = emscripten_functions.FS;

  const WORKER_DATA = {};

  self.WORKER_DATA = WORKER_DATA;
  self.WORKER_DATA.emscripten_functions = emscripten_functions;

  self.postMessage({
    type : "init_openjtalk_1",
    data : null,
  });

  init();

  self.onmessage = async function(e) {
    if (e.data.type === "runInference") {
      let result = runInference(e.data.data);
      self.postMessage({
          type : "runInference_result",
          data : "data:audio/wav;base64," + result.toBase64(),
	  dataType : "dataurl",
          relation : e.data.relation
      });
    }
  };

  function init_consts() {
    let c = {};
    WORKER_DATA.constants = c;
  }

  function make_str(s) {
    let buf = new TextEncoder().encode(s);
    let arr = new Uint8Array(buf.length + 1);
    arr.set(buf, 0);
    let pos = _malloc(arr.length);
    emscripten_functions.HEAPU8.set(arr, pos);
    return pos;
  }

  function make_str_with_size(s, size) {
    let buf = new TextEncoder().encode(s);
    if (s.length + 1 > size) { throw Error("make str with size failed: string exceeds max length"); }
    let arr = new Uint8Array(size);
    arr.set(buf, 0);
    let pos = _malloc(size);
    emscripten_functions.HEAPU8.set(arr, pos);
    return pos;
  }

  async function init() {
    init_consts();
    WORKER_DATA.open_jtalk = _malloc(280);
    _Open_JTalk_initialize(WORKER_DATA.open_jtalk);
    const str_dict = make_str("/dic");
    const str_voice = make_str("/takumi_happy.htsvoice");
    let bool_result =
        _Open_JTalk_load(WORKER_DATA.open_jtalk, str_dict, str_voice);
    if (!bool_result) {
      console.error("init failed");
      self.postMessage(
          {type : "init_failed", data : {reason : "Open JTalk load failed"}});
      return;
    }

    self.postMessage({type : "init_all_finished", data : null});
  }

  function make_u32(num_in) {
    const pointer = _malloc(4);
    emscripten_functions.HEAPU32[align_u32(pointer)] = num_in;
    return pointer;
  }

  function align_u32(x) {
    return Math.floor(x / 4) // TODO
  }

  function runInference(s_in) {
    WORKER_DATA.output_buffer = "";

    const str_text = make_str_with_size(s_in, 1024);

    const str_out_wav = make_str("/out.wav");
    const str_wb = make_str("wb");
    const fopen_out_wav = _fopen(str_out_wav, str_wb);
    _free(str_out_wav);
    _free(str_wb);

    _Open_JTalk_set_speed(WORKER_DATA.open_jtalk, 1.8);
    _Open_JTalk_add_half_tone(WORKER_DATA.open_jtalk, 1.0);
    _Open_JTalk_synthesis(WORKER_DATA.open_jtalk, str_text, fopen_out_wav, 0);
    _free(str_text);
    _fclose(fopen_out_wav);

    const a = _malloc(1024);
    _free(a);
    console.log("mem leak test: " + a)

    return FS.readFile("/out.wav");
  }
  self.WORKER_DATA.runInference = runInference;
  self.WORKER_DATA.make_str = make_str;
});
