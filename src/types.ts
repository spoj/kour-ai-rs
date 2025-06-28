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

export type TMessageImageURL = {
  type: "image_url";
  image_url: {
    url: string;
  };
};

export type TMessageFile = {
  type: "file";
  file: {
    filename: string;
    file_data: string;
  };
};

export type MessageContent = (TextContent | TMessageImageURL | TMessageFile)[];

export interface IToolCall {
  id: string;
  type: "function";
  function: {
    name: string;
    arguments: string;
  };
  result?: string;
}

export interface IChatCompletionMessage {
  tool_call_id?: string;
  role: "user" | "assistant" | "tool";
  content: MessageContent;
  isNotification?: boolean;
  tool_calls?: IToolCall[];
  toolName?: string;
  toolArgs?: string;
  toolResult?: string;
}

export interface IChatCompletionOptions {
  apiKey: string;
  modelName: string;
}

export type IChatCompletionUpdate =
  | { type: "Start" }
  | { type: "End" }
  | { type: "Delta"; delta: string; is_notification: boolean }
  | { type: "Message"; message: IChatCompletionMessage }
  | {
      type: "ToolCall";
      tool_name: string;
      tool_call_id: string;
      tool_args: string;
    }
  | { type: "ToolDone"; tool_call_id: string; tool_result: string };
