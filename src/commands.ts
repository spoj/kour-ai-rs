import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { IChatCompletionOptions, IChatCompletionUpdate, ISettings } from "./types";

export const getSettings = async (): Promise<ISettings> => {
  return await invoke("get_settings");
};

export const setSettings = async (settings: ISettings): Promise<void> => {
  await invoke("set_settings", { settings });
};

export const chatCompletion = async (
  options: IChatCompletionOptions,
  callback: (update: IChatCompletionUpdate) => void
): Promise<void> => {
  const unlisten = await listen("chat_completion_update", (event) => {
    callback(event.payload as IChatCompletionUpdate);
  });

  try {
    await invoke("chat_completion", { options });
  } finally {
    unlisten();
  }
};