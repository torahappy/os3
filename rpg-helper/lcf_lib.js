/*  lcf_lib.js  */
export async function call_lcf_lib(function_name, args) {
  // -----------------------------------------------------------------------
  // 1️⃣  Keep a single worker instance
  // -----------------------------------------------------------------------
  if (!call_lcf_lib.worker) {
    call_lcf_lib.worker = new Worker(new URL('./lcf_worker.js', import.meta.url), { type: 'module' });
    call_lcf_lib.pending = new Map();   // transaction_id → {resolve, reject}
  }

  const worker = call_lcf_lib.worker;

  // -----------------------------------------------------------------------
  // 2️⃣  Generate a unique transaction id
  // -----------------------------------------------------------------------
  const transaction_id = ++call_lcf_lib.lastId;

  // -----------------------------------------------------------------------
  // 3️⃣  Return a Promise that will be resolved/rejected by the worker
  // -----------------------------------------------------------------------
  const promise = new Promise((resolve, reject) => {
    call_lcf_lib.pending.set(transaction_id, { resolve, reject });
  });

  // -----------------------------------------------------------------------
  // 4️⃣  Send the request to the worker
  // -----------------------------------------------------------------------
  worker.postMessage({ type: function_name, args, transaction_id });

  // -----------------------------------------------------------------------
  // 5️⃣  Listen for the reply – only once per worker to avoid leaks
  // -----------------------------------------------------------------------
  if (!call_lcf_lib.listenerSet) {
    worker.addEventListener('message', (e) => {
      const { type, transaction_id, data, error } = e.data;
      if (type !== 'return') return;

      const handlers = call_lcf_lib.pending.get(transaction_id);
      if (!handlers) return;  // unknown transaction id – ignore

      call_lcf_lib.pending.delete(transaction_id);

      if (error) {
        handlers.reject(new Error(error));
      } else {
        handlers.resolve(data);
      }
    });
    call_lcf_lib.listenerSet = true;
  }

  return promise;
}

// -----------------------------------------------------------------------
// 6️⃣  Internal helpers
// -----------------------------------------------------------------------
call_lcf_lib.lastId = 0;
call_lcf_lib.pending = null;
call_lcf_lib.worker = null;
call_lcf_lib.listenerSet = false;

