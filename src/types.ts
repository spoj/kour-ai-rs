export interface ISettings {
  apiKey: string;
  modelName: string;
  rootDir: string;
  systemPrompt: string;
  sofficePath: string;
  providerOrder: string;
}

export interface IChatCompletionMessage {
  role: "user" | "assistant";
  content: string;
  isNotification?: boolean;
}

export interface IChatCompletionOptions {
  apiKey: string;
  modelName: string;
  messages: IChatCompletionMessage[];
}

export type IChatCompletionUpdate =
  | { type: "Start" }
  | { type: "End" }
  | { type: "Update"; message: string; is_notification: boolean };