import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { IChatCompletionUpdate, ISettings, MessageContent } from "./types";

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
export const cancelOutstandingRequest = async (): Promise<void> => {
	await invoke("cancel_outstanding_request");
}

export const ensureLibreoffice = async (): Promise<void> => {
	await invoke("ensure_libreoffice");
}

export const delete_message = async (id: number): Promise<void> => {
	await invoke("delete_message", { id });
}

export const delete_tool_interaction = async (llm_interaction_id: number, tool_call_id: string): Promise<void> => {
	await invoke("delete_tool_interaction", { llmInteractionId: llm_interaction_id, toolCallId: tool_call_id });
}

export const onLibreofficeUpdate = async (
	callback: (update: any) => void
) => {
	return await listen("libreoffice_update", (event) => {
		callback(event.payload);
	});
}

export const chat = async (
	content: MessageContent,
): Promise<void> => {
	await invoke("chat", { content: content });
};

export const onChatCompletionUpdate = async (
	callback: (update: IChatCompletionUpdate) => void
) => {
	return await listen("chat_completion_update", (event) => {
		callback(event.payload as IChatCompletionUpdate);
	});
}