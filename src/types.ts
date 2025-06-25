export interface ISettings {
  apiKey: string;
  modelName: string;
  rootDir: string;
  sofficePath: string;
  providerOrder: string;
}

export type TextContent = {
  type: "text";
  text: string;
};

export type ImageContent = {
  type: "image_url";
  image_url: {
    url: string;
  };
};

export type MessageContent = (TextContent | ImageContent)[];

export interface IChatCompletionMessage {
  tool_call_id?: string;
  role: "user" | "assistant";
  content: MessageContent;
  isNotification?: boolean;
  toolName?: string;
  toolArgs?: string;
  toolResult?: string;
}

export interface IChatCompletionOptions {
  apiKey: string;
  modelName: string;
  messages: IChatCompletionMessage[];
}

export type IChatCompletionUpdate =
  | { type: "Start" }
  | { type: "End" }
  | { type: "Update"; message: string; is_notification: boolean }
  | {
      type: "ToolCall";
      tool_name: string;
      tool_call_id: string;
      tool_args: string;
    }
  | { type: "ToolDone"; tool_call_id: string; tool_result: string };