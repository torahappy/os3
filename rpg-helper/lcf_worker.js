/*  lcf_worker.js  */
import LcfModule from './dist/rpg_lsd_io.js'; // the exported factory

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
    malloc = Module._malloc;
    free = Module._free;
    moduleInstance = Module;

    // The FS helpers are on the Module object itself
    return Module;
  }
}

// ---------------------------------------------------------------------------
// 2️⃣  Helper: write a JavaScript string into wasm memory
// ---------------------------------------------------------------------------
function writeString(Module, str) {
  const encoded = (new TextEncoder()).encode(str)
  const ptr = malloc(encoded.length + 1); // +1 for the terminating NUL
  for (let i = 0; i < encoded.length; i++) {
    Module.HEAPU8[ptr + i] = encoded[i];
  }
  Module.HEAPU8[ptr + encoded.length] = 0; // NUL‑terminator
  return ptr;
}

// ---------------------------------------------------------------------------
// 3️⃣  Helper: read an Int32Array from wasm memory
// ---------------------------------------------------------------------------
function readInt32(Module, ptr, len) {
  return Module.HEAP32.slice(ptr >> 2, (ptr >> 2) + len);
}

// ---------------------------------------------------------------------------
// 4️⃣  Helper: read an Int8Array from wasm memory
// ---------------------------------------------------------------------------
function readInt8(Module, ptr, len) {
  return Module.HEAP8.slice(ptr, ptr + len);
}

function read_rpg_var_generic(args, call_func, Module) {
  const {filename, offset, count} = args;
  const ptrName = writeString(Module, filename);
  const retPtr = malloc(count * 4); // 4 bytes per int32

  const retCode = call_func(ptrName, offset, count, retPtr);
  const return_data = readInt32(Module, retPtr, count);
  free(ptrName);
  free(retPtr);

  if (retCode !== 0)
    throw new Error(`read_rpg_var failed: ${retCode}`);
  return return_data;
}

function write_rpg_var_generic(args, call_func, Module) {
  const {in_filename, out_filename, offset, count, variables} = args;
  if (!((Array.isArray(variables) && variables.every(Number.isInteger)) ||
        variables instanceof Int32Array)) {
    throw new Error(`The argument "Variables" is not an integer array!`);
  }
  if (variables.length !== count) {
    throw new Error(`Variable Length is Invalid`);
  }
  const ptrIn = writeString(Module, in_filename);
  const ptrOut = writeString(Module, out_filename);
  const ptrVar = malloc(count * 4);
  Module.HEAP32.set(variables, ptrVar >> 2)

  const retCode = call_func(ptrIn, ptrOut, offset, count, ptrVar);
  free(ptrIn);
  free(ptrOut);
  free(ptrVar);

  if (retCode !== 0)
    throw new Error(`write_rpg_var failed: ${retCode}`);
  return null; // nothing to return
}

function read_rpg_switch_generic(args, call_func, Module) {
  const {filename, offset, count} = args;
  const ptrName = writeString(Module, filename);
  const retPtr = malloc(count); // 1 byte per int8

  const retCode = call_func(ptrName, offset, count, retPtr);
  const return_data = readInt8(Module, retPtr, count);
  free(ptrName);
  free(retPtr);

  if (retCode !== 0)
    throw new Error(`read_rpg_switch failed: ${retCode}`);
  return return_data.map((x) => (Boolean(x)));
}

function write_rpg_switch_generic(args, call_func, Module) {

  const {in_filename, out_filename, offset, count, switches} = args;
  if (!((Array.isArray(variables) &&
         switches.every(x => (typeof x === 'boolean'))))) {
    throw new Error(`The argument "Switches" is not an switches array!`);
  }
  if (switches.length !== count) {
    throw new Error(`Switches Array Length is Invalid`);
  }
  const ptrIn = writeString(Module, in_filename);
  const ptrOut = writeString(Module, out_filename);
  const ptrSw = malloc(count);
  if (!switches.every((x) => (typeof x === 'boolean')))
    throw new Error(
        `write_rpg_switch failed: Input vector must be a list of booleans`);

  Module.HEAPU8.set(switches.map((x) => (Number(x))), ptrSw);

  const retCode = call_func(ptrIn, ptrOut, offset, count, ptrSw);
  free(ptrIn);
  free(ptrOut);
  free(ptrSw);

  if (retCode !== 0)
    throw new Error(`write_rpg_switch failed: ${retCode}`);
  return null;
}

// ---------------------------------------------------------------------------
// 5️⃣  Core: call a specific exported function
// ---------------------------------------------------------------------------
async function callExported(Module, name, args) {
  switch (name) {
  case 'read_rpg_var_lgs': {
    return read_rpg_var_generic(args, Module._read_rpg_var_lgs, Module)
  }

  case 'write_rpg_var_lgs': {
    return write_rpg_var_generic(args, Module._write_rpg_var_lgs, Module)
  }

  case 'read_rpg_switch_lgs': {
    return read_rpg_switch_generic(args, Module._read_rpg_switch_lgs, Module)
  }

  case 'write_rpg_switch_lgs': {
    return write_rpg_switch_generic(args, Module._write_rpg_switch_lgs, Module)
  }
  case 'read_rpg_var': {
    return read_rpg_var_generic(args, Module._read_rpg_var, Module)
  }

  case 'write_rpg_var': {
    return write_rpg_var_generic(args, Module._write_rpg_var, Module)
  }

  case 'read_rpg_switch': {
    return read_rpg_switch_generic(args, Module._read_rpg_switch, Module)
  }

  case 'write_rpg_switch': {
    return write_rpg_switch_generic(args, Module._write_rpg_switch, Module)
  }
  // ---------------------------------------------------------------------
  // File helpers – not part of the C API, but useful for the worker
  // ---------------------------------------------------------------------
  case 'write_file': {
    const {filename, data} = args;
    // `FS_createDataFile` expects a Uint8Array – we store the JSON string
    // verbatim
    Module.FS.writeFile(filename, data);
    return null;
  }

  case 'read_file': {
    const {filename} = args;
    const data = Module.FS.readFile(filename); // returns a Uint8Array
    return data;
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
