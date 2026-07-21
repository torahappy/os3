const { app, BrowserWindow, ipcMain } = require('electron');
const path = require('path');
const { spawn } = require('child_process');

function createWindow () {
  const win = new BrowserWindow({
    width: 800,
    height: 600,
    fullscreen: true,
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      preload: path.join(__dirname, 'preload.js')
    }
  });

  win.loadURL(`file://${__dirname}/../../os3yew/wasm/doomscroll/index.html?screen`);
}

ipcMain.handle('do_print', async (e, ...args) => {
  // Validate that we have exactly 3 arguments (optional)

  const pythonPath = path.join(__dirname, 'venv', 'bin', 'python');

  // Use spawn; `shell:true` allows a relative script path
  const child = spawn(pythonPath, [path.join(__dirname, 'do_print.py'), ...args], { shell: true, cwd: __dirname });

  let stdout = '';
  let stderr = '';

  child.stdout.on('data', d => stdout += d.toString());
  child.stderr.on('data', d => stderr += d.toString());

  // Return a Promise that resolves/rejects when the child exits
  return new Promise((resolve, reject) => {
    child.on('close', code => {
      if (code === 0) {
        resolve({ stdout, stderr, code });
      } else {
        reject({ stdout, stderr, code });
      }
    });

    child.on('error', err => {
      reject(err);
    });
  });
});

app.whenReady().then(createWindow);

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit();
});

