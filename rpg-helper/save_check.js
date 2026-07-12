/*  main.js  –  example usage of call_lcf_lib + easyrpgPlayer FS  */

import {call_lcf_lib} from './lcf_lib.js';

(async () => {
  // path of the save file inside the game's FS
  const SAVE_PATH = 'Save/Save.lgs';          // ← use .lsd if that is the real extension
  const SAVE_PATH_TMP = 'tmp.lgs';          // ← use .lsd if that is the real extension
  const POLL_MS   = 1000;                     // poll every 1 s

  /* -------------------------------------------------------------
     helper: write a string into the wasm module's FS
     ------------------------------------------------------------- */
  async function copyToWasmFS() {
    const data = await easyrpgPlayer.FS.readFile(SAVE_PATH);
    await call_lcf_lib('write_file', { filename: SAVE_PATH_TMP, data });
  }

  /* -------------------------------------------------------------
     helper: read a single variable from the wasm module
     ------------------------------------------------------------- */
  async function getVar(var_idx) {
    // read_rpg_var returns an array of Int32 values
    const vals = await call_lcf_lib('read_rpg_var_lgs', {
      filename: SAVE_PATH_TMP,
      offset: var_idx - 1,
      count: 1
    });
    return vals[0];              // first (and only) element
  }

  /* -------------------------------------------------------------
     helper: set a single variable in the wasm module
     ------------------------------------------------------------- */
  async function setVar(var_idx, value) {
    await call_lcf_lib('write_rpg_var_lgs', {
      in_filename: SAVE_PATH_TMP,
      out_filename: SAVE_PATH_TMP,
      offset: var_idx - 1,
      count: 1,
      variables: [value]            // plain array is fine – worker copies it
    });
  }


  /* -------------------------------------------------------------
     helper: read multi variables from the wasm module
     ------------------------------------------------------------- */
  async function getVarMulti(var_idx, length) {
    // read_rpg_var returns an array of Int32 values
    const vals = await call_lcf_lib('read_rpg_var_lgs', {
      filename: SAVE_PATH_TMP,
      offset: var_idx - 1,
      count: length
    });
    return vals; 
  }

  /* -------------------------------------------------------------
     helper: set multi variables in the wasm module
     ------------------------------------------------------------- */
  async function setVarMulti(var_idx, value) {
    await call_lcf_lib('write_rpg_var_lgs', {
      in_filename: SAVE_PATH_TMP,
      out_filename: SAVE_PATH_TMP,
      offset: var_idx - 1,
      count: value.length,
      variables: value  
    });
  }

  /* -------------------------------------------------------------
     helper: read the updated file from the wasm FS
     ------------------------------------------------------------- */
  async function readUpdatedFile() {
    return await call_lcf_lib('read_file', { filename: SAVE_PATH_TMP });
  }

  /* -------------------------------------------------------------
     helper: write a string back into the game's FS
     ------------------------------------------------------------- */
  async function syncBackToGameFS(updated) {
    await easyrpgPlayer.FS.writeFile(SAVE_PATH, updated);
  }

  /* -------------------------------------------------------------
     main polling loop
     ------------------------------------------------------------- */
  setInterval(async () => {
    try {
      /* 1️⃣  Copy the current save file into the wasm FS  */
      await copyToWasmFS();

      console.log("write ok")

      /* 2️⃣  Read variable #100 to #109  */
      const cur = await getVarMulti(100, 10);
      console.log("read ok")

      /* 3️⃣  If #100 is 1 → reset it to 0  */
      if (cur[0] === 1) {
        await setVar(100, 0);
        console.log('Variable #100 was 1 – reset to 0.');
	console.log(cur[6], cur[7])

        /* 4️⃣  Pull the modified file from the wasm FS  */
        const updated = await readUpdatedFile();

        /* 5️⃣  Write the updated file back into the game’s FS  */
        await syncBackToGameFS(updated);
      }
    } catch (err) {
      console.error('Error while polling the save file:', err);
    }
  }, POLL_MS);
})();

