/**
 * rpg_lsd_io controller
 *
 * This module does the following:
 *
 * 1. Creates a Dexie (IndexedDB) database with the required tables.
 * 2. Communicates with the RPG process via Web Worker (call_lcf_lib).
 *        - every 50ms (SYNC_WINDOW), reads var #98 (RPG Maker internal VarID: #99)
 *        - if we have ping, puts 0 to #98.
 *        - 500ms after ping, we read or write actual data.
 * 3. (as concurrent async context) Performs the "user login" flow:
 *        - launches a QR code scanner and parses a line like
 *              QR-Code:105 <base64-signature>
 *        - reads the signing key from credentials.ts
 *        - validates the signature
 *        - inserts/updates the user and login tables
 *        - runs the write command to communicate with RPG executable.
 * 4. (as concurrent async context) Handles the optional data-input stage
 *        - launches QR code scanner and parses a line like
 *              QR-Code:<data1> <data2> … <dataN> <signature>
 *        - validates the signature
 *        - runs the write command to communicate with RPG executable.
 * 5. (main async context) Process progression / data input / logout signal from the Game.
 *        - reads #99 from RPG (RPG Maker internal VarID: #100).
 *        - if the first number is 2, updates the progression table
 *        - if the first number is 3, triggers the data-input stage
 *        - if the first number is 4, triggers logout
 *        - Values >= 10000 will result in error.
 *
 * This module is designed to run indefinitely, so that even if we have some error,
 * we can restart and reset all ephemeral states.
 *
 * For browser, restarting means just reloading the web page by `location.reload()`.
 */

// @ts-ignore TS7016: Could not find a declaration file for module

import type {EasyRPGModule} from './lcf_lib_defines.d.ts';
import { Dexie, type EntityTable } from "dexie";

import QrScanner from "qr-scanner";

import { call_lcf_lib } from './lcf_lib.js';

// ---------------------------------------------------------------------------
//  Constants
// ---------------------------------------------------------------------------

const SYNC_WINDOW = 50; // ms
const WRITE_WINDOW = 500; // ms
const RPG_TIMEOUT_WINDOW = 10_000; // ms
const QR_APP_WINDOW = 2_000; // ms
const MAX_DATA = 10;

const SAVE_PATH = "/easyrpg/Save/Save.lgs";

// ---------------------------------------------------------------------------
//  Types
// ---------------------------------------------------------------------------

type QrState = "login" | "data-input" | "";

interface LoginResult {
  userId: number | null;
  errorCode?: number;
  errorMessage?: string;
}

interface DataInputResult {
  data: number[] | null;
  errorCode?: number;
  errorMessage?: string;
}

type LoginQueueItem =
  | { userId: number }
  | { errorCode: number; errorMessage: string };
type DataInputQueueItem =
  | { data: number[] }
  | { errorCode: number; errorMessage: string };

// ---------------------------------------------------------------------------
//  Database (Dexie)
// ---------------------------------------------------------------------------

class DB extends Dexie {
  users!: EntityTable<UserRow, "user_id">;
  logins!: EntityTable<LoginRow, "rowId">;
  logouts!: EntityTable<LogoutRow, "rowId">;
  progressions!: EntityTable<ProgressionRow, "rowId">
  choices!: EntityTable<ChoiceRow, "rowId">

  constructor() {
    super("TaraimawashiDB")
    this.version(1).stores({
      users: "user_id, creation_date, current_progression",
      logins: "++rowId, user_id, login_date, progression",
      logouts: "++rowId, user_id, logout_date, progression",
      progressions: "++rowId, user_id, progression_date, progression",
      choices: "++rowId, user_id, choice_date, progression, details",
    })
  }
}

interface UserRow {
  user_id: number;
  creation_date: string;
  current_progression: number;
}

interface LoginRow {
  rowId: number;
  user_id: number;
  login_date: string;
  progression: number;
}

interface LogoutRow {
  rowId: number;
  user_id: number;
  logout_date: string;
  progression: number;
}

interface ProgressionRow {
  rowId: number;
  user_id: number;
  progression_date: string;
  progression: number;
}

interface ChoiceRow {
  rowId: number;
  user_id: number;
  choice_date: string;
  progression: number;
  details: string;
}

// ------------------------------------------
//  Simple in-memory queue with max size = 1
// ------------------------------------------

class QueueError extends Error {}

class QueueFullError extends QueueError {
  constructor() {
    super("Queue Error: Queue is Full");
  }
}

/// simple in-memory queue with max size = 1, only supports non-wait I/O.
class SimpleQueue<T> {
  private item: T | null = null;

  get_nowait(): T | null {
    if (this.item === null) {
      return null;
    }
    return this.item;
  }

  put_nowait(item: T): void {
    if (this.item !== null) {
      throw new QueueFullError();
    }
    this.item = item;
  }

  flush(): void {
    this.item = null;
  }
}

// ---------------------------------------------------------------------------
//  Global state (shared between concurrent async contexts)
// ---------------------------------------------------------------------------

let currentQrState: QrState = "login";

const loginQueue = new SimpleQueue<LoginQueueItem | null>();
const dataInputQueue = new SimpleQueue<DataInputQueueItem | null>();


// ---------------------------------------------------------------------------
//  Helpers
// ---------------------------------------------------------------------------

function debug(msg: string): void {
  console.debug(`[DEBUG] ${msg}`);
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function nowIso(): string {
  return new Date().toISOString();
}

// ---------------------------------------------------------------------------
//  Signature & QR parsing
// ---------------------------------------------------------------------------

/**
 * Async signature verification using Web Crypto (SHA-256).
 */
async function verifySignatureAsync(
  data: string,
  signatureB64: string,
  signingKey: string,
  purpose: string,
): Promise<boolean> {
  const text = `${signingKey}/${purpose}/${data}`;
  const encoder = new TextEncoder();
  const bytes = encoder.encode(text);

  const digest = await crypto.subtle.digest("SHA-256", bytes);
  const expectedB64 = btoa(String.fromCharCode(...new Uint8Array(digest)));

  debug(`Signature check: ${expectedB64 === signatureB64}`);
  return expectedB64 === signatureB64;
}

function parseQrCodeLine(line: string): [number, string] {
  const m = line.match(/(\d+)\s+([A-Za-z0-9+/=]+)/);
  if (!m) {
    throw new Error(`Line does not match QR-Code pattern: ${line}`);
  }
  return [parseInt(m[1], 10), m[2]];
}

// ---------------------------------------------------------------------------
//  RPG I/O wrappers (Web Worker calls)
// ---------------------------------------------------------------------------

/**
 * Read `count` variables starting at `offset` from the save file.
 * Returns an array of numbers (the variable values).
 */
async function rpgReadVars(
  count: number,
  offset: number = 99,
): Promise<Int32Array> {
  // 1. Read save data from main process
  console.log(Object.keys(window))
  const saveData: Uint8Array = window.easyrpgPlayer.FS.readFile(SAVE_PATH);

  // 2. Copy to Web Worker
  await call_lcf_lib("write_file", { filename: SAVE_PATH, data: saveData });

  // 3. Read the variables in the Web Worker
  const result = await call_lcf_lib("read_rpg_var_lgs", {
    filename: SAVE_PATH,
    offset,
    count,
  });

  // result is expected to be the array of numbers read from the file
  return result as Int32Array;
}

/**
 * Write an array of numbers starting at `offset` into the save file.
 */
async function rpgWriteVars(
  variables: number[],
  offset: number = 199,
): Promise<void> {
  // 1. Read save data from main process
  const saveData: Uint8Array = window.easyrpgPlayer.FS.readFile(SAVE_PATH);

  // 2. Copy to Web Worker
  await call_lcf_lib("write_file", { filename: SAVE_PATH, data: saveData });

  // 3. Write the variables in the Web Worker
  await call_lcf_lib("write_rpg_var_lgs", {
    in_filename: SAVE_PATH,
    out_filename: SAVE_PATH,
    offset,
    count: variables.length,
    variables,
  });

  // 4. Read the modified file back from Web Worker
  const updated: Uint8Array = await call_lcf_lib("read_file", {
    filename: SAVE_PATH,
  }) as Uint8Array;

  // 5. Write back to main process
  window.easyrpgPlayer.FS.writeFile(SAVE_PATH, updated);
}

/**
 * Write an error code to the RPG process.
 * e.g. 199 1 [ 10001 ]
 */
async function rpgWriteError(code: number): Promise<void> {
  await rpgWriteVars([10000 + code]);
}

// ---------------------------------------------------------------------------
//  QR code scanning (browser)
// ---------------------------------------------------------------------------

async function scanQrCode(): Promise<string> {
  return new Promise<string>((resolve, reject) => {
    const videoElem = document.createElement("video");
    videoElem.style = "display: block !important; opacity: 1; position: absolute; top:0; left:0; width: calc((100vw - 100vh * 1.3333333333) / 2); height: auto;";
    document.body.appendChild(videoElem);

    const qrScanner = new QrScanner(videoElem, (result) => {
      // Got a QR code result
      cleanup();
      resolve(result);
    });

    async function cleanup(): Promise<void> {
      try {
        qrScanner.stop();
      } catch {}
      if (videoElem.parentNode) {
        videoElem.remove();
      }
    }

    qrScanner.start().catch((err) => {
      cleanup();
      reject(err);
    });

    // Store scanner for cleanup
    (window as unknown as Record<string, unknown>).__qrScanner = qrScanner;
  });
}

// ---------------------------------------------------------------------------
//  QR Reader (replaces the QR thread)
// ---------------------------------------------------------------------------

async function qrReader(signingKey: string): Promise<void> {
  while (true) {
    await delay(QR_APP_WINDOW);


    try {
      const qrData = await scanQrCode();

      if (currentQrState === "login") {
        processQrLogin(qrData, signingKey);
      } else if (currentQrState === "data-input") {
        processQrDataInput(qrData, signingKey);
      } else {
        debug("QR code provided but no use");
      }
    } catch (e) {
      debug(`QR scan error: ${e}`);
    }
  }
}

async function processQrLogin(
  qrData: string,
  signingKey: string,
): Promise<void> {
  // Find a matching QR-code line
  let userId: number | null = null;
  let signatureB64: string | null = null;

  for (const line of splitlines(qrData)) {
    try {
      [userId, signatureB64] = parseQrCodeLine(line);
      break;
    } catch {
      continue;
    }
  }

  if (userId === null || signatureB64 === null) { throw Error("no user id and signature found"); }

  // Validate
  const valid = await verifySignatureAsync(
    String(userId),
    signatureB64,
    signingKey,
    "ikiteikou_os_v0.0002_aone_cards",
  );
  if (!valid) {
    loginQueue.put_nowait({
      errorCode: 1,
      errorMessage: `Invalid QR-code signature for user ${userId}`,
    });
    return;
  }

  loginQueue.put_nowait({ userId });
}

async function processQrDataInput(
  qrData: string,
  signingKey: string,
): Promise<void> {
  // Expected format: QR-Code:2 5 3 7 <signature>
  let data: number[] = [];
  let signatureB64: string | null = null;

  for (const line of splitlines(qrData)) {
    const m = line.match(/((\d+ )+)([A-Za-z0-9+/=]+)/);
    if (!m) continue;

    const dataPart = m[1];
    data = dataPart.split(/\s+/).map((x) => parseInt(x, 10));
    signatureB64 = m[2];
    break;
  }
  if (signatureB64 === null) { throw Error("no user id and signature found"); }

  if (data.length === 0) {
    dataInputQueue.put_nowait({
      errorCode: 2,
      errorMessage: "No QR-code data line found while data-input",
    });
    return;
  }

  if (data.length > MAX_DATA) {
    dataInputQueue.put_nowait({
      errorCode: 2,
      errorMessage: "Too many data provided",
    });
    return;
  }

  // Verify the signature
  const path = data.join("/");
  const valid = await verifySignatureAsync(
    path,
    signatureB64,
    signingKey,
    "ikiteikou_os_v0.0002_data_input",
  );

  if (!valid) {
    dataInputQueue.put_nowait({
      errorCode: 2,
      errorMessage: "Invalid data-input QR-code signature",
    });
    return;
  }

  dataInputQueue.put_nowait({ data });
}

// ---------------------------------------------------------------------------
//  Queue helpers
// ---------------------------------------------------------------------------

function sanitizeQueues(): void {
  if (currentQrState === "") {
    loginQueue.flush();
    dataInputQueue.flush();
  } else if (currentQrState === "login") {
    dataInputQueue.flush();
  } else if (currentQrState === "data-input") {
    loginQueue.flush();
  }
}

// ---------------------------------------------------------------------------
//  Progression loop (main async context)
// ---------------------------------------------------------------------------

async function progressionLoop(db: DB, signingKey: string): Promise<void> {
  let userId: number | null = null;
  let currentProgression: number | null = null;
  let pingStart: number | null = null;
  let processed = true;

  while (true) {
    // 1. Read the ping
    const pingData = await rpgReadVars(1, 98);

    if (pingData.length === 0) {
      await delay(SYNC_WINDOW);
      continue;
    }

    // We have a ping
    if (pingData[0] === 1 && processed === true) {
      pingStart = Date.now();
      await rpgWriteVars([0], 98);
      processed = false;
    }

    if (pingStart !== null && Date.now() - pingStart >= RPG_TIMEOUT_WINDOW) {
      throw new Error("Lost connection from RPG!");
    }

    // If we have a ping and we have waited enough
    if (
      pingStart !== null &&
      processed === false &&
      Date.now() - pingStart >= WRITE_WINDOW
    ) {
      processed = true;
      sanitizeQueues();

      if (userId === null) {
        // ===========================================
        // BEFORE LOGIN
        // ===========================================

        currentQrState = "login";
        sanitizeQueues();

        const loginResult = loginQueue.get_nowait();

        if (loginResult === null) {
          // empty / timeout — keep looping
        } else if ("userId" in loginResult) {
          // Successful login
          userId = loginResult.userId;

          // create / update DB
          let user = await db.users.get(userId);
          if (!user) {
            await db.users.add({
              user_id: userId,
              creation_date: nowIso(),
              current_progression: 1,
            });
            currentProgression = 1;
          } else {
            currentProgression = user.current_progression;
          }

          if (currentProgression === null || currentProgression === undefined) {
            throw new Error(
              "SQL data type error; perhaps Database structure is malformed",
            );
          }

          // log the login
          await db.logins.add({
            user_id: userId,
            login_date: nowIso(),
            progression: currentProgression,
          });

          // write the required command
          await rpgWriteVars([1, userId, currentProgression]);

          // On success, disable QR input
          currentQrState = "";
          sanitizeQueues();
          debug(`Login Ok: ${userId} ${currentProgression}`);
        } else {
          // Login error
          if (loginResult.errorCode !== undefined) {
            await rpgWriteError(loginResult.errorCode);
            await db.choices.add({
              user_id: 0,
              choice_date: nowIso(),
              progression: 0,
              details: `(before login) Login Error: ${loginResult.errorMessage}`,
            });
            debug(`Login Error: ${loginResult.errorMessage}`);
          }
        }

        // Check for error from RPG
        const check = await rpgReadVars(1, 99);
        if (check.length > 0 && check[0] >= 10000) {
          throw new Error(
            `Desync from RPG with Error Code (while not logged in): ${check[0]}`,
          );
        }
      } else {
        // ===========================================
        // AFTER LOGIN
        // ===========================================

        if (currentQrState === "data-input") {
          const dataResult = dataInputQueue.get_nowait();

          if (dataResult === null) {
            // timeout — do nothing
          } else if ("data" in dataResult) {
            // Tell the input data to the RPG process
            if (currentProgression !== null) {
              await db.choices.add({
                user_id: userId ?? 0,
                choice_date: nowIso(),
                progression: currentProgression,
                details: `(data input) [${dataResult.data.join(", ")}]`,
              });
              await rpgWriteVars([
                2,
                userId ?? 0,
                currentProgression,
                ...dataResult.data,
              ]);
            } else {
              throw new Error("something wrong happened");
            }
            currentQrState = "";
            sanitizeQueues();
          } else {
            // Error
            if (dataResult.errorCode !== undefined) {
              await rpgWriteError(dataResult.errorCode);
              await db.choices.add({
                user_id: userId ?? 0,
                choice_date: nowIso(),
                progression: currentProgression ?? 0,
                details: `(before login) Login Error: ${dataResult.errorMessage}`,
              });
              debug(`Data-input Error: ${dataResult.errorMessage}`);
            }
          }
        }

        // read vars #100 - #198 (0-indexed: #99 - #198 → offset 99, count 100)
        const out = await rpgReadVars(100, 99);

        if (out.length === 0) {
          await delay(SYNC_WINDOW);
          continue;
        }

        // The first number is the *command* indicator
        const cmd = out[0];
        debug(`Command ID from RPG: ${cmd}`);

        if (cmd === 2) {
          // Progression command
          if (userId !== out[1]) {
            throw new Error("User ID desync from RPG!!");
          }
          const nextProg = out[2];
          debug(`${userId} Progression to ${nextProg}`);
          // Update DB
          await db.progressions.add({
            user_id: userId ?? 0,
            progression_date: nowIso(),
            progression: nextProg,
          });
          await db.users
            .where("user_id")
            .equals(userId!)
            .modify({ current_progression: nextProg } as any);
          currentProgression = nextProg;
        } else if (cmd === 3) {
          // Data-Input command
          debug(`User ${userId} Data Input Request`);
          if (userId !== out[1]) {
            throw new Error("User ID desync from RPG!!");
          }
          // change QR read state
          currentQrState = "data-input";
          sanitizeQueues();
        } else if (cmd === 4) {
          // Logout command
          if (userId !== out[1]) {
            throw new Error("User ID desync from RPG!!");
          }
          debug(`User ${userId} Logout`);
          await db.logouts.add({
            user_id: userId ?? 0,
            logout_date: nowIso(),
            progression: currentProgression ?? 0,
          });
          userId = null;
          currentProgression = null;
          // change QR read state
          currentQrState = "login";
          sanitizeQueues();
        } else if (cmd >= 10000) {
          throw new Error(`Desync from RPG with Error Code: ${cmd}`);
        }
      }

      // loop and re-run
      await delay(SYNC_WINDOW);
    }
  }
}

// ---------------------------------------------------------------------------
//  Main entry point
// ---------------------------------------------------------------------------

async function main(): Promise<void> {
  while (window.easyrpgPlayer === undefined) {
    await delay(1000);
  }
  const db = new DB();
  await db.open();

  // Read the signing key
  const signingKey = await loadSigningKey();

  // Launch the QR reader (concurrent async context)
  const qrReaderPromise = qrReader(signingKey);

  // Start the progression loop
  const progressionPromise = progressionLoop(db, signingKey).catch((e) => {console.error(e)});

}

/**
 * Load the signing key from credentials.
 * In the browser, this could be imported from a config module.
 */
let signing_key_cache: string | null = null;
async function loadSigningKey(): Promise<string> {
  if (signing_key_cache !== null) {
    return signing_key_cache
  } else{
    signing_key_cache = (await (await fetch('credentials.py')).text())
	    .replace(/^SIGNING_KEY="/, '')
	    .replace(/"$/, '')
	    .replace('\n', '');
    return signing_key_cache;
  }
}

// ---------------------------------------------------------------------------
//  Splitlines polyfill (Python's str.splitlines())
// ---------------------------------------------------------------------------

const splitlines = function (data: string): string[] {
  return data.split(/\r\n|\r|\n/).map((l) => l.replace(/[\r\n]+$/, ""));
};

// ---------------------------------------------------------------------------
//  Entry
// ---------------------------------------------------------------------------

main().catch((e) => {
  console.error(e);
});
