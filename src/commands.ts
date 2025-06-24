import { invoke } from "@tauri-apps/api/core";
import { ISettings, IChatCompletionOptions } from "./types";

export const getSettings = async (): Promise<ISettings> => {
  return await invoke("get_settings");
};

export const setSettings = async (settings: ISettings): Promise<void> => {
  await invoke("set_settings", { settings });
};

export const chatCompletion = async (
  options: IChatCompletionOptions
): Promise<string> => {
  return await invoke("chat_completion", { options });
};