/*  lcf_worker.js  */
import LcfModule from './dist-wasm/rpg_lsd_io.js';
// ---------------------------------------------------------------------------
//  Instantiate the module
// ---------------------------------------------------------------------------
let moduleInstance = null;
/// initialize the main module
async function ensureModule() {
    if (moduleInstance !== null) {
        return moduleInstance;
    }
    else {
        const Module = await LcfModule({
            noInitialRun: true,
            noInitialMemory: true,
            print: () => { },
            printErr: () => { }
        });
        moduleInstance = Module;
        return Module;
    }
}
// ---------------------------------------------------------------------------
//  Helper: write a JavaScript string into wasm memory
// ---------------------------------------------------------------------------
function writeString(Module, str) {
    const encoded = (new TextEncoder()).encode(str);
    const ptr = Module._malloc(encoded.length + 1); // +1 for the terminating NUL
    for (let i = 0; i < encoded.length; i++) {
        Module.HEAPU8[ptr + i] = encoded[i];
    }
    Module.HEAPU8[ptr + encoded.length] = 0; // NUL‑terminator
    return ptr;
}
// ---------------------------------------------------------------------------
//  Helper: read an Int32Array from wasm memory
// ---------------------------------------------------------------------------
function readInt32(Module, ptr, len) {
    return Module.HEAP32.slice(ptr >> 2, (ptr >> 2) + len);
}
// ---------------------------------------------------------------------------
//  Helper: read an Int8Array from wasm memory
// ---------------------------------------------------------------------------
function readInt8(Module, ptr, len) {
    return Module.HEAP8.slice(ptr, ptr + len);
}
function read_rpg_var_generic(args, call_func, Module) {
    const { filename, offset, count } = args;
    const ptrName = writeString(Module, filename);
    const retPtr = Module._malloc(count * 4); // 4 bytes per int32
    const retCode = call_func(ptrName, offset, count, retPtr);
    const return_data = readInt32(Module, retPtr, count);
    Module._free(ptrName);
    Module._free(retPtr);
    if (retCode !== 0)
        throw new Error(`read_rpg_var failed: ${retCode}`);
    return return_data;
}
function write_rpg_var_generic(args, call_func, Module) {
    const { in_filename, out_filename, offset, count, variables } = args;
    if (!((Array.isArray(variables) && variables.every(Number.isInteger)) ||
        variables instanceof Int32Array)) {
        throw new Error(`The argument "Variables" is not an integer array!`);
    }
    if (variables.length !== count) {
        throw new Error(`Variable Length is Invalid`);
    }
    const ptrIn = writeString(Module, in_filename);
    const ptrOut = writeString(Module, out_filename);
    const ptrVar = Module._malloc(count * 4);
    Module.HEAP32.set(variables, ptrVar >> 2);
    const retCode = call_func(ptrIn, ptrOut, offset, count, ptrVar);
    Module._free(ptrIn);
    Module._free(ptrOut);
    Module._free(ptrVar);
    if (retCode !== 0)
        throw new Error(`write_rpg_var failed: ${retCode}`);
    return null; // nothing to return
}
function read_rpg_switch_generic(args, call_func, Module) {
    const { filename, offset, count } = args;
    const ptrName = writeString(Module, filename);
    const retPtr = Module._malloc(count); // 1 byte per int8
    const retCode = call_func(ptrName, offset, count, retPtr);
    const return_data = readInt8(Module, retPtr, count);
    Module._free(ptrName);
    Module._free(retPtr);
    if (retCode !== 0)
        throw new Error(`read_rpg_switch failed: ${retCode}`);
    return Array(...return_data).map((x) => (Boolean(x)));
}
function write_rpg_switch_generic(args, call_func, Module) {
    const { in_filename, out_filename, offset, count, switches } = args;
    if (!((Array.isArray(switches) &&
        switches.every(x => (typeof x === 'boolean'))))) {
        throw new Error(`The argument "Switches" is not an switches array!`);
    }
    if (switches.length !== count) {
        throw new Error(`Switches Array Length is Invalid`);
    }
    const ptrIn = writeString(Module, in_filename);
    const ptrOut = writeString(Module, out_filename);
    const ptrSw = Module._malloc(count);
    if (!switches.every((x) => (typeof x === 'boolean')))
        throw new Error(`write_rpg_switch failed: Input vector must be a list of booleans`);
    Module.HEAPU8.set(switches.map((x) => (Number(x))), ptrSw);
    const retCode = call_func(ptrIn, ptrOut, offset, count, ptrSw);
    Module._free(ptrIn);
    Module._free(ptrOut);
    Module._free(ptrSw);
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
            return read_rpg_var_generic(args, Module._read_rpg_var_lgs, Module);
        }
        case 'write_rpg_var_lgs': {
            return write_rpg_var_generic(args, Module._write_rpg_var_lgs, Module);
        }
        case 'read_rpg_switch_lgs': {
            return read_rpg_switch_generic(args, Module._read_rpg_switch_lgs, Module);
        }
        case 'write_rpg_switch_lgs': {
            return write_rpg_switch_generic(args, Module._write_rpg_switch_lgs, Module);
        }
        case 'read_rpg_var': {
            return read_rpg_var_generic(args, Module._read_rpg_var, Module);
        }
        case 'write_rpg_var': {
            return write_rpg_var_generic(args, Module._write_rpg_var, Module);
        }
        case 'read_rpg_switch': {
            return read_rpg_switch_generic(args, Module._read_rpg_switch, Module);
        }
        case 'write_rpg_switch': {
            return write_rpg_switch_generic(args, Module._write_rpg_switch, Module);
        }
        // ---------------------------------------------------------------------
        // File helpers – not part of the C API, but useful for the worker
        // ---------------------------------------------------------------------
        case 'write_file': {
            const { filename, data } = args;
            Module.FS.writeFile(filename, data);
            return null;
        }
        case 'read_file': {
            const { filename } = args;
            const data = Module.FS.readFile(filename);
            return data;
        }
        default:
            throw new Error(`Unknown function name ${name}`);
    }
}
// ---------------------------------------------------------------------------
// Worker message handler
// ---------------------------------------------------------------------------
self.onmessage = async function (e) {
    const { type, args, transaction_id } = e.data;
    try {
        const Module = await ensureModule();
        const result = await callExported(Module, type, args);
        self.postMessage({ type: 'return', transaction_id, data: result });
    }
    catch (err) {
        self.postMessage({ type: 'return', transaction_id, data: null, error: String(err.message || err.errno) });
    }
};
