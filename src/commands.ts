import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { IChatCompletionMessage, IChatCompletionOptions, IChatCompletionUpdate, ISettings } from "./types";

export const getSettings = async (): Promise<ISettings> => {
	return await invoke("get_settings");
};

export const saveSettings = async (settings: ISettings): Promise<void> => {
	await invoke("set_settings", { settings });
};

export const getHistory = async (): Promise<IChatCompletionMessage[]> => {
	return await invoke("get_history");
}

export const clearHistory = async (): Promise<void> => {
	await invoke("clear_history");
}

export const chatCompletion = async (
	message: IChatCompletionMessage,
	callback: (update: IChatCompletionUpdate) => void
): Promise<void> => {
	const unlisten = await listen("chat_completion_update", (event) => {
		callback(event.payload as IChatCompletionUpdate);
	});

	try {
		await invoke("chat_completion", { message });
	} finally {
		unlisten();
	}
};