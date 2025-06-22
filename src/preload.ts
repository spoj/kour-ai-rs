// See the Electron documentation for details on how to use preload scripts:
// https://www.electronjs.org/docs/latest/tutorial/process-model#preload-scripts
import { ipcRenderer, contextBridge } from "electron";

contextBridge.exposeInMainWorld("electron", {
  get: (key: string) => {
    return ipcRenderer.sendSync("settings-get", key);
  },
  set: (key: string, val: any) => {
    return ipcRenderer.send("settings-set", key, val);
  },
  getSettings: () => {
    return ipcRenderer.sendSync("all-settings-get");
  },
  setSettings: (val: any) => ipcRenderer.send("all-settings-set", val),
});
