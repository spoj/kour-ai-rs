// See the Electron documentation for details on how to use preload scripts:
// https://www.electronjs.org/docs/latest/tutorial/process-model#preload-scripts
import { ipcRenderer, contextBridge } from "electron";
import { IChatCompletionOptions, ISettings, IChatCompletionUpdate } from "./main";

contextBridge.exposeInMainWorld("electron", {
  get: (key: keyof ISettings) => {
    return ipcRenderer.sendSync("settings-get", key);
  },
  set: (key: keyof ISettings, val: string) => {
    return ipcRenderer.send("settings-set", key, val);
  },
  getSettings: (): ISettings => {
    return ipcRenderer.sendSync("all-settings-get");
  },
  setSettings: (val: ISettings) => ipcRenderer.send("all-settings-set", val),
  chatCompletion: (options: IChatCompletionOptions, callback: (update: IChatCompletionUpdate) => void) => {
    ipcRenderer.removeAllListeners('chat:completion-update');
    ipcRenderer.on('chat:completion-update', (_, update) => {
      callback(update);
    });
    ipcRenderer.send('chat:completion', options);
  },
});
