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
      x: rect.x,
      y: rect.y,
      width: rect.width,
      height: rect.height,
      top: rect.top + window.scrollY,
      left: rect.left + window.scrollX,
      character: span.textContent, // the single character in that <span>
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
  // 1 Detect whether we should use the API path
  // ------------------------------------------------------------------
  const search = new URLSearchParams(window.location.search);
  const useApi = search.has("tts") && search.get("tts") === "api";

  // ------------------------------------------------------------------
  // 2 Build the list of candidate names for the requested language
  // ------------------------------------------------------------------
  let speechCandidates = [];
  if (lang === "en") {
    speechCandidates = ["libritts_r-medium", "Bad News"];
  }
  if (lang === "ja") {
    speechCandidates = ["takumi_happy", "Kyoko"];
  }

  // ------------------------------------------------------------------
  // 3 API path – fetch the list of voices from the backend
  // ------------------------------------------------------------------
  if (useApi) {
    try {
      if (window.VOICE_CACHE === undefined) {
        const resp = await fetch("/api/voices");
        if (!resp.ok) {
          throw new Error(`GET /api/voices failed (${resp.status})`);
        }
        const voices = await resp
          .json(); // expect an array of strings [id1, id2, id3, ...]

        // Find the first voice whose id contains one of our candidates
        const matched = voices.find((v) =>
          speechCandidates.some((c) => v.includes(c))
        );

        if (!matched) {
          console.warn("No matching voice found on /api/voices");
          return;
        }

        // Cache the id string
        window.VOICE_CACHE = matched;
      }

      // ------------------------------------------------------------------
      // 4 POST to /api/say with the chosen voice id & the text
      // ------------------------------------------------------------------
      if (window.VOICE_CACHE !== undefined) {
        const payload = { voice: window.VOICE_CACHE, text };
        const r = await fetch("/api/say", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(payload),
        });

        if (!r.ok) {
          console.error(`POST /api/say failed (${r.status})`);
        }
      }

      return;
    } catch (e) {
      console.error("doSpeech (API mode) error:", e);
    }
  }

  // ------------------------------------------------------------------
  // 5 Non‑API mode – use the browser’s SpeechSynthesis API
  // ------------------------------------------------------------------
  const utter = new SpeechSynthesisUtterance(text);

  // Lazily initialise the voice cache (the voice object itself)
  if (window.VOICE_CACHE === undefined) {
    const candidates = speechCandidates;
    const voices = window.speechSynthesis.getVoices();

    const matchedVoice = voices.find((v) =>
      candidates.some((c) => v.name.includes(c))
    );
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
