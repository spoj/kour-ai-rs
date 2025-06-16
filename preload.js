const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('electronAPI', {
  sendMessage: (message) => ipcRenderer.invoke('send-message', message),
  getSettings: () => ipcRenderer.invoke('get-settings'),
  setSettings: (settings) => ipcRenderer.invoke('set-settings', settings),
  processAttachment: (fileData) => ipcRenderer.invoke('process-attachment', fileData),
  onDebugLog: (callback) => ipcRenderer.on('debug-log', (_event, value) => callback(value)),
  onUpdateHistory: (callback) => ipcRenderer.on('update-history', (_event, value) => callback(value)),
  openExternalUrl: (url) => ipcRenderer.invoke('open-external-url', url),
});