// preload.js
const { contextBridge, ipcRenderer } = require('electron');

// Expose only what you need – keep the rest of the Node API hidden
contextBridge.exposeInMainWorld('api_electron', {
  /**
   * Triggers the Python script with three numeric arguments.
   * @param {number|string} num1
   * @param {number|string} num2
   * @param {number|string} num3
   * @returns {Promise<{stdout:string,stderr:string,code:number}>}
   */
  do_print: (...args) => ipcRenderer.invoke('do_print', ...args)
});
