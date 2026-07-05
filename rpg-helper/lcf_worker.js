/*  lcf_worker.js  */
import {LcfModule} from './dist/rpg_lsd_io.js'; // the exported factory

// ---------------------------------------------------------------------------
// 1️⃣  Instantiate the module the first time the worker receives a message
// ---------------------------------------------------------------------------
let moduleInstance = null; // will hold Promise resolving to Module
let malloc, free;          // helpers that we’ll need

async function ensureModule() {
  if (moduleInstance !== null) {
    return moduleInstance;
  } else {
    const Module = await LcfModule({
      // Tell the runtime we want the FS helpers exported
      noInitialRun : true,
      noInitialMemory : true,
    });

    // Grab the exported helpers for convenience
    malloc = Module.cwrap('_malloc', 'number', [ 'number' ]);
    free = Module.cwrap('_free', 'void', [ 'number' ]);
    moduleInstance = Module;

    // The FS helpers are on the Module object itself
    return Module;
  }
}

// ---------------------------------------------------------------------------
// 2️⃣  Helper: write a JavaScript string into wasm memory
// ---------------------------------------------------------------------------
function writeString(Module, str) {
  const ptr = malloc(str.length + 1); // +1 for the terminating NUL
  const view = new Uint8Array(Module.memory.buffer, ptr, str.length + 1);
  for (let i = 0; i < str.length; ++i)
    view[i] = str.charCodeAt(i);
  view[str.length] = 0; // NUL‑terminator
  return ptr;
}

// ---------------------------------------------------------------------------
// 3️⃣  Helper: read an Int32Array from wasm memory
// ---------------------------------------------------------------------------
function readInt32(Module, ptr, len) {
  const arr = Array.from(new Int32Array(Module.memory.buffer, ptr, len));
  return Array.from(arr);
}

// ---------------------------------------------------------------------------
// 4️⃣  Helper: read an Int8Array from wasm memory
// ---------------------------------------------------------------------------
function readInt8(Module, ptr, len) {
  const arr = Array.from(new Int8Array(Module.memory.buffer, ptr, len));
  return Array.from(arr);
}

// ---------------------------------------------------------------------------
// 5️⃣  Core: call a specific exported function
// ---------------------------------------------------------------------------
async function callExported(Module, name, args) {
  switch (name) {
  case 'read_rpg_var': {
    const {filename, offset, count} = args;
    const ptrName = writeString(Module, filename);
    const retPtr = malloc(count * 4); // 4 bytes per int32

    const retCode = Module._read_rpg_var(ptrName, offset, count, retPtr);
    const return_data = readInt32(Module, retPtr, count);
    free(ptrName);
    free(retPtr);

    if (retCode !== 0)
      throw new Error(`read_rpg_var failed: ${retCode}`);
    return return_data;
  }

  case 'write_rpg_var': {
    const {in_filename, out_filename, offset, count, variables} = args;
    const ptrIn = writeString(Module, in_filename);
    const ptrOut = writeString(Module, out_filename);
    const ptrVar = malloc(count * 4);
    new Int32Array(Module.memory.buffer, ptrVar, count).set(variables);

    const retCode = Module._write_rpg_var(ptrIn, ptrOut, offset, count, ptrVar);
    free(ptrIn);
    free(ptrOut);
    free(ptrVar);

    if (retCode !== 0)
      throw new Error(`write_rpg_var failed: ${retCode}`);
    return null; // nothing to return
  }

  case 'read_rpg_switch': {
    const {filename, offset, count} = args;
    const ptrName = writeString(Module, filename);
    const retPtr = malloc(count); // 1 byte per int8

    const retCode = Module._read_rpg_switch(ptrName, offset, count, retPtr);
    const return_data = readInt8(Module, retPtr, count);
    free(ptrName);
    free(retPtr);

    if (retCode !== 0)
      throw new Error(`read_rpg_switch failed: ${retCode}`);
    return return_data.map((x) => (Boolean(x)));
  }

  case 'write_rpg_switch': {
    const {in_filename, out_filename, offset, count, switches} = args;
    const ptrIn = writeString(Module, in_filename);
    const ptrOut = writeString(Module, out_filename);
    const ptrSw = malloc(count);
    if (!switches.every((x) => (typeof x === 'boolean')))
      throw new Error(
          `write_rpg_switch failed: Input vector must be a list of booleans`);
    new Int8Array(Module.memory.buffer, ptrSw, count)
        .set(switches.map((x) => (Number(x))));

    const retCode =
        Module._write_rpg_switch(ptrIn, ptrOut, offset, count, ptrSw);
    free(ptrIn);
    free(ptrOut);
    free(ptrSw);

    if (retCode !== 0)
      throw new Error(`write_rpg_switch failed: ${retCode}`);
    return null;
  }

  // ---------------------------------------------------------------------
  // File helpers – not part of the C API, but useful for the worker
  // ---------------------------------------------------------------------
  case 'write_file': {
    const {filename, data} = args;
    // `FS_createDataFile` expects a Uint8Array – we store the JSON string
    // verbatim
    Module.FS_createDataFile('/', filename,
                             new Uint8Array(new TextEncoder().encode(data)),
                             true, true);
    return null;
  }

  case 'read_file': {
    const {filename} = args;
    const data = Module.FS_readFile(
        '/', filename, {encoding : 'utf8'}); // returns a Uint8Array
    return new TextDecoder().decode(data);
  }

  default:
    throw new Error(`Unknown function name ${name}`);
  }
}

// ---------------------------------------------------------------------------
// 6️⃣  Worker message handler
// ---------------------------------------------------------------------------
self.onmessage = async function(e) {
  const {type, args, transaction_id} = e.data;

  try {
    const Module = await ensureModule();
    const result = await callExported(Module, type, args);

    self.postMessage({type : 'return', transaction_id, data : result});
  } catch (err) {
    self.postMessage(
        {type : 'return', transaction_id, data : null, error : err.message});
  }
};
