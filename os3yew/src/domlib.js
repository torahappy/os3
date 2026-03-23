/**
 * Return a flat array of all text nodes that belong to the element
 * identified by the given `id`. The traversal is recursive, but the
 * result is a single array.
 *
 * @param {string} id - The id of the element to start from.
 * @returns {Text[]}  - An array of Text node objects.
 */
function getAllTextNodes(id) {
  // Grab the element; return an empty array if it doesn't exist
  const root = document.getElementById(id);
  if (!root) {
    return [];
  }

  // Create a TreeWalker that only shows text nodes
  const walker = document.createTreeWalker(
      root,
      NodeFilter.SHOW_TEXT,
      null,
      false,
  );

  // Walk the tree and push each encountered text node into the result array
  const textNodes = [];
  while (walker.nextNode()) {
    textNodes.push(walker.currentNode);
  }

  return textNodes; // a flat array of Text nodes
}

/**
 * Wrap every character in the element with the given id in a <span>.
 *
 * @param {string} id - The id of the element to process.
 */
function wrapCharactersInSpans(id) {
  // Grab all the text nodes inside the element
  const textNodes = getAllTextNodes(id);

  // For each text node … create a <span> for each character
  textNodes.forEach((textNode) => {
    const parent = textNode.parentNode;
    if (!parent) {
      return; // safety check
    }

    // Build a fragment that will replace the original text node
    const frag = document.createDocumentFragment();

    // Walk through the text and create a <span> for each character
    for (let i = 0; i < textNode.textContent.length; i++) {
      const span = document.createElement("span");
      // Preserve the exact character (including whitespace)
      span.textContent = textNode.textContent[i];
      span.setAttribute("data-fetch-metrics", "1");
      frag.appendChild(span);
    }

    // Insert the fragment before the original node, then remove it
    parent.insertBefore(frag, textNode);
    parent.removeChild(textNode);
  });
}

/**
 * Run `wrapCharactersInSpans` for the supplied element and return a
 * flat array of objects describing each generated `<span>`.
 * Reverses the effect wrapCharactersInSpans have done.
 *
 * @param {string} id  - The id of the element to process.
 * @returns {Array<{x:number, y:number, width:number, height:number,
 *          top:number, right:number, bottom:number, left:number,
 *          character:string}>}
 */
export function getSpanMetrics(id) {
  const root = document.getElementById(id);

  const tmp_elem = document.createElement(root.tagName.toLowerCase());
  tmp_elem.id = "span-metrics-tmp-" + id;
  tmp_elem.style = root.style.cssText;
  tmp_elem.className = root.className;
  tmp_elem.innerHTML = root.innerHTML;
  document.body.appendChild(tmp_elem);

  wrapCharactersInSpans(tmp_elem.id);

  const metrics = [];

  const spans = tmp_elem.querySelectorAll("span[data-fetch-metrics]");

  spans.forEach((span) => {
    const rect = span.getBoundingClientRect();

    // DOMRect has the same numeric properties we need
    metrics.push({
      x : rect.x,
      y : rect.y,
      width : rect.width,
      height : rect.height,
      top : rect.top + window.scrollY,
      left : rect.left + window.scrollX,
      character : span.textContent, // the single character in that <span>
    });
  });

  tmp_elem.remove();

  return JSON.stringify(metrics);
}

/**
 * Speak some text, using the browser’s SpeechSynthesis API by default.
 * If the current page URL contains the query string `tts=api` then
 *   1.  Query `/api/voices` to obtain the list of voice IDs that the
 *       backend knows about.
 *   2.  Pick the first ID that matches one of the candidates for the
 *       supplied language.
 *   3.  Store that ID string in `window.VOICE_CACHE` instead of a
 *       SpeechSynthesisVoice object.
 *   4.  Send a POST to `/api/say` with `{ voice: <id>, text: <text> }`
 *       instead of using the browser’s synth.
 *
 * This function assumes that lang parameter does not change across the
 * session.
 *
 * @param {string} text  The text to speak
 * @param {string} lang  Language code (`"en"`, `"ja"`, …)
 */
export async function doSpeech(text, lang) {
  console.log(text);

  // ------------------------------------------------------------------
  // Detect whether we should use the API path
  // ------------------------------------------------------------------
  const search = new URLSearchParams(window.location.search);
  const useApi = search.has("tts") && search.get("tts") === "api";
  const useWasm = search.has("tts") && search.get("tts") === "wasm";

  // ------------------------------------------------------------------
  // Build the list of candidate names for the requested language
  // ------------------------------------------------------------------
  let speechCandidates = [];
  if (lang === "en") {
    speechCandidates = [ "libritts_r-medium", "Bad News" ];
  }
  if (lang === "ja") {
    speechCandidates = [ "takumi_happy", "Kyoko" ];
  }

  // ------------------------------------------------------------------
  // API path – fetch the list of voices from the backend
  // ------------------------------------------------------------------
  if (useApi) {
    try {
      if (window.VOICE_CACHE === undefined) {
        const resp = await fetch("/api/voices");
        if (!resp.ok) {
          throw new Error(`GET /api/voices failed (${resp.status})`);
        }
        const voices =
            await resp
                .json(); // expect an array of strings [id1, id2, id3, ...]

        // Find the first voice whose id contains one of our candidates
        const matched =
            voices.find((v) => speechCandidates.some((c) => v.includes(c)));

        if (!matched) {
          console.warn("No matching voice found on /api/voices");
          return;
        }

        // Cache the id string
        window.VOICE_CACHE = matched;
      }

      // ------------------------------------------------------------------
      // POST to /api/say with the chosen voice id & the text
      // ------------------------------------------------------------------
      if (window.VOICE_CACHE !== undefined) {
        const payload = {voice : window.VOICE_CACHE, text};
        const r = await fetch("/api/say", {
          method : "POST",
          headers : {"Content-Type" : "application/json"},
          body : JSON.stringify(payload),
        });

        if (!r.ok) {
          console.error(`POST /api/say failed (${r.status})`);
        }
      }

      return;
    } catch (e) {
      console.error("doSpeech (API mode) error:", e);
    }
  } else if (useWasm) {
    // ------------------------------------------------------------------
    // WASM mode – use the wasm webworkers, send runInference message
    // ------------------------------------------------------------------
    if (window.VOICE_WORKER_DATA.workers.every((x) => x.is_init_completed)) {
      if ((window.VOICE_WORKER_DATA.lastPlayTime === null ||
           (new Date()) - window.VOICE_WORKER_DATA.lastPlayTime > 300)) {
        let workers = window.VOICE_WORKER_DATA.workers;
        let target_worker =
            workers.reduce((
                               a,
                               b,
                               ) => (a.queue.length > b.queue.length ? b : a));
	console.log(target_worker.i)
        target_worker.worker.postMessage({
          type : "runInference",
          data : text,
          relation : window.VOICE_WORKER_DATA.currentRelation,
        });
        target_worker.queue.push({
          relation : window.VOICE_WORKER_DATA.currentRelation,
          timestamp : new Date(),
        });
        window.VOICE_WORKER_DATA.currentRelation += 1;
        window.VOICE_WORKER_DATA.lastPlayTime = new Date();
      } else {
        console.log("doSpeech: skip reading")
      }
    } else {
      console.error("doSpeech (WASM mode) error: workers not yet initialized");
    }
  } else {
    // ------------------------------------------------------------------
    // Non‑API mode – use the browser’s SpeechSynthesis API
    // ------------------------------------------------------------------
    const utter = new SpeechSynthesisUtterance(text);

    // Lazily initialise the voice cache (the voice object itself)
    if (window.VOICE_CACHE === undefined) {
      const candidates = speechCandidates;
      const voices = window.speechSynthesis.getVoices();

      const matchedVoice =
          voices.find((v) => candidates.some((c) => v.name.includes(c)));
      if (matchedVoice) {
        window.VOICE_CACHE = matchedVoice;
      }
    }

    if (window.VOICE_CACHE !== undefined) {
      utter.voice = window.VOICE_CACHE;
      // Prevent the queued speech
      window.speechSynthesis.cancel();
      window.speechSynthesis.speak(utter);
    }
  }
}

/**
 * Prepare Speech Synthesis. Do nothing except in wasm mode.
 *
 * @param {string} lang - The speech language. Should not be changed during the
 *     app run.
 */
export async function prepareSpeech(lang) {
  const search = new URLSearchParams(window.location.search);
  const useWasm = search.has("tts") && search.get("tts") === "wasm";
  let volume = 1.0;

  switch (lang) {
  case "en":
    volume = 0.5;
    break;
  case "ja":
    volume = 0.85;
    break;
  }

  if (useWasm) {
    if (window.VOICE_WORKER_DATA === undefined) {
      window.VOICE_WORKER_DATA = {};
      let cores = 4; // TODO: how to obtain core data???
      let workers = [];
      for (let i = 0; i < cores; i++) {
        let worker = new Worker("voice-worker-" + lang + ".js", {
          type : "module",
        });

        let worker_obj = {
          worker,
          queue : [],
          is_init_completed : false,
	  i
        };

        workers.push(worker_obj);

        worker.addEventListener("message", (e) => {
          if (e.data.type === "init_ort") {
            worker_obj.is_init_completed = true;
          } else if (e.data.type === "runInference_result") {
            playArray(e.data.data, 22050, volume);
            worker_obj.queue = worker_obj.queue.filter(
                (x) => (x.relation !== e.data.relation),
            );
          }
        });
      }

      window.VOICE_WORKER_DATA["workers"] = workers;
      window.VOICE_WORKER_DATA["currentRelation"] = 0;
      window.VOICE_WORKER_DATA["lastPlayTime"] = null;
    }
  }
}

/**
 * Play Float32Array as a sound.
 *
 * @param {Float32Array} myArray - Float32Array of the sound.
 * @param {number} rate - The frequency of the sound in Hz.
 * @param {number} volume - The volume of the sound.
 */
function playArray(myArray, rate, volume) {
  const audioCtx = new (window.AudioContext || window.webkitAudioContext)();
  const frameCount = myArray.length;

  const audioBuffer = audioCtx.createBuffer(1, frameCount, rate);

  const nowBuffering = audioBuffer.getChannelData(0);
  for (let i = 0; i < frameCount; i++) {
    nowBuffering[i] = myArray[i] * volume;
  }

  const source = audioCtx.createBufferSource();
  source.buffer = audioBuffer;
  source.connect(audioCtx.destination);
  source.start();
}
