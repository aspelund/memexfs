import { Worker } from "node:worker_threads";
import { availableParallelism } from "node:os";
import { fileURLToPath } from "node:url";

const workerScript = fileURLToPath(new URL("./worker.mjs", import.meta.url));

/**
 * Create a pool of MemexFS worker threads.
 * @param {string} dir - Directory of .md files to load.
 * @param {{ workers?: number }} [opts]
 * @returns {Promise<{ grep, read, ls, toolDefinitions, documentCount, terminate }>}
 */
export async function createPool(dir, opts = {}) {
  const size = opts.workers ?? availableParallelism();
  let nextId = 0;
  let terminated = false;

  /** @type {{ worker: Worker, pending: Map<number, { resolve, reject }> }[]} */
  const slots = [];

  function spawnWorker() {
    const worker = new Worker(workerScript, { workerData: { dir } });
    const pending = new Map();
    const slot = { worker, pending };

    worker.on("message", (msg) => {
      if (msg.type === "ready") return;
      const { id, result, error } = msg;
      const p = pending.get(id);
      if (!p) return;
      pending.delete(id);
      if (error !== undefined) {
        p.reject(new Error(error));
      } else {
        p.resolve(result);
      }
    });

    worker.on("error", (err) => {
      for (const p of pending.values()) {
        p.reject(err);
      }
      pending.clear();
    });

    worker.on("exit", (code) => {
      if (terminated) return;
      // Reject any pending requests
      for (const p of pending.values()) {
        p.reject(new Error(`worker exited with code ${code}`));
      }
      pending.clear();
      // Replace crashed worker
      const idx = slots.indexOf(slot);
      if (idx !== -1) {
        slots[idx] = spawnWorker();
      }
    });

    return slot;
  }

  // Spawn all workers
  for (let i = 0; i < size; i++) {
    slots.push(spawnWorker());
  }

  // Wait for all workers to be ready
  await Promise.all(
    slots.map(
      (slot) =>
        new Promise((resolve, reject) => {
          function onMessage(msg) {
            if (msg.type === "ready") {
              slot.worker.removeListener("message", onMessage);
              slot.worker.removeListener("error", onError);
              resolve();
            }
          }
          function onError(err) {
            slot.worker.removeListener("message", onMessage);
            slot.worker.removeListener("error", onError);
            reject(err);
          }
          slot.worker.on("message", onMessage);
          slot.worker.on("error", onError);
        })
    )
  );

  function dispatch(method, args) {
    if (terminated) return Promise.reject(new Error("pool is terminated"));

    // Least-pending dispatch
    let best = slots[0];
    for (let i = 1; i < slots.length; i++) {
      if (slots[i].pending.size < best.pending.size) {
        best = slots[i];
      }
    }

    const id = nextId++;
    return new Promise((resolve, reject) => {
      best.pending.set(id, { resolve, reject });
      best.worker.postMessage({ id, method, args });
    });
  }

  return {
    grep: (pattern, glob) =>
      dispatch("grep", glob !== undefined ? [pattern, glob] : [pattern]),
    read: (path, offset, limit) => {
      const args = [path];
      if (offset !== undefined) args.push(offset);
      if (limit !== undefined) args.push(limit);
      return dispatch("read", args);
    },
    ls: (path) => dispatch("ls", [path]),
    toolDefinitions: () => dispatch("toolDefinitions", []),
    documentCount: () => dispatch("documentCount", []),
    terminate: async () => {
      terminated = true;
      await Promise.all(slots.map((s) => s.worker.terminate()));
    },
  };
}
