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
    if (!root) return [];

    // Create a TreeWalker that only shows text nodes
    const walker = document.createTreeWalker(
        root,
        NodeFilter.SHOW_TEXT,
        null,
        false
    );

    // Walk the tree and push each encountered text node into the result array
    const textNodes = [];
    while (walker.nextNode()) {
        textNodes.push(walker.currentNode);
    }

    return textNodes;          // a flat array of Text nodes
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
  textNodes.forEach(textNode => {
    const parent = textNode.parentNode;
    if (!parent) return; // safety check

    // Build a fragment that will replace the original text node
    const frag = document.createDocumentFragment();

    // Walk through the text and create a <span> for each character
    for (let i = 0; i < textNode.textContent.length; i++) {
      const span = document.createElement('span');
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

  const orig_innerHTML = root.innerHTML;

  wrapCharactersInSpans(id);

  const metrics = [];

  const spans = root.querySelectorAll('span[data-fetch-metrics]');

  spans.forEach(span => {
    const rect = span.getBoundingClientRect();

    // DOMRect has the same numeric properties we need
    metrics.push({
      x:        rect.x,
      y:        rect.y,
      width:    rect.width,
      height:   rect.height,
      top:      rect.top,
      right:    rect.right,
      bottom:   rect.bottom,
      left:     rect.left,
      character: span.textContent   // the single character in that <span>
    });
  });

  root.innerHTML = orig_innerHTML;

  return JSON.stringify(metrics);
}

