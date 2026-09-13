import type {CallLcfLibData, LcfMessage, LcfMessageReturn} from './lcf_lib_defines.d.ts';

export const call_lcf_lib_data: CallLcfLibData = {
  worker: null,
  pending: new Map(),
  lastId: 0
}

export async function call_lcf_lib(function_name: string, args: LcfMessage): Promise<LcfMessageReturn> {
  // -----------------------------------------------------------------------
  // 1️⃣  Keep a single worker instance
  // -----------------------------------------------------------------------
  if (!call_lcf_lib_data.worker) {
    call_lcf_lib_data.worker = new Worker(
        './lcf_worker.js', {type : 'module'});
    call_lcf_lib_data.pending = new Map(); 
    call_lcf_lib_data.worker.addEventListener('message', (e) => {
      const {type, transaction_id, data, error} = e.data;
      if (type !== 'return')
        return;

      const handlers = call_lcf_lib_data.pending.get(transaction_id);
      if (!handlers)
        return; // unknown transaction id – ignore

      call_lcf_lib_data.pending.delete(transaction_id);

      if (error) {
        handlers.reject(new Error(error));
      } else {
        handlers.resolve(data);
      }
    });
  }

  const worker = call_lcf_lib_data.worker;

  // -----------------------------------------------------------------------
  // 2️⃣  Generate a unique transaction id
  // ----------------------------------------js check if boolean-------------------------------
  const transaction_id = ++call_lcf_lib_data.lastId;

  // -----------------------------------------------------------------------
  // 3️⃣  Return a Promise that will be resolved/rejected by the worker
  // -----------------------------------------------------------------------
  const promise: Promise<LcfMessageReturn> = new Promise((resolve, reject) => {
    call_lcf_lib_data.pending.set(transaction_id, {resolve, reject});
  });

  // -----------------------------------------------------------------------
  // 4️⃣  Send the request to the worker
  // -----------------------------------------------------------------------
  worker.postMessage({type : function_name, args, transaction_id});

  return promise;
}

// -----------------------------------------------------------------------
// 6️⃣  Internal helpers
// -----------------------------------------------------------------------

window.call_lcf_lib = call_lcf_lib;
window.call_lcf_lib_data = call_lcf_lib_data;
