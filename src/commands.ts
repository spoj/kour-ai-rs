import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { IChatCompletionMessage, IChatCompletionUpdate, ISettings } from "./types";

export const getSettings = async (): Promise<ISettings> => {
	return await invoke("get_settings");
};

export const saveSettings = async (settings: ISettings): Promise<void> => {
	await invoke("set_settings", { settings });
};

export const replayHistory = async (): Promise<void> => {
	await invoke("replay_history");
}

export const clearHistory = async (): Promise<void> => {
	await invoke("clear_history");
}

export const chatCompletion = async (
	message: IChatCompletionMessage,
): Promise<void> => {
	await invoke("chat_completion", { message });
};

export const onChatCompletionUpdate = async (
	callback: (update: IChatCompletionUpdate) => void
) => {
	return await listen("chat_completion_update", (event) => {
		callback(event.payload as IChatCompletionUpdate);
	});
}