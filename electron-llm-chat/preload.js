const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('electronAPI', {
  sendMessage: (message) => ipcRenderer.invoke('send-message', message),
  getApiKey: () => ipcRenderer.invoke('get-api-key'),
  setApiKey: (apiKey) => ipcRenderer.invoke('set-api-key', apiKey),
  getModelName: () => ipcRenderer.invoke('get-model-name'),
  setModelName: (modelName) => ipcRenderer.invoke('set-model-name', modelName),
  getRootDir: () => ipcRenderer.invoke('get-root-dir'),
  setRootDir: (rootDir) => ipcRenderer.invoke('set-root-dir', rootDir),
  getSystemPrompt: () => ipcRenderer.invoke('get-system-prompt'),
  setSystemPrompt: (systemPrompt) => ipcRenderer.invoke('set-system-prompt', systemPrompt),
  onDebugLog: (callback) => ipcRenderer.on('debug-log', (_event, value) => callback(value)),
});