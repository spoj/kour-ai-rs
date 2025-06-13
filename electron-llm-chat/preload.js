const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('electronAPI', {
  sendMessage: (message) => ipcRenderer.invoke('send-message', message),
  getApiKey: () => ipcRenderer.invoke('get-api-key'),
  setApiKey: (apiKey) => ipcRenderer.invoke('set-api-key', apiKey),
  getModelName: () => ipcRenderer.invoke('get-model-name'),
  setModelName: (modelName) => ipcRenderer.invoke('set-model-name', modelName),
  onDebugLog: (callback) => ipcRenderer.on('debug-log', (_event, value) => callback(value)),
});