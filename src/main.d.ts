export interface ISettings {
  apiKey: string;
  modelName: string;
  rootDir: string;
  systemPrompt: string;
  sofficePath: string;
  providerOrder: string;
}

export interface IChatCompletionMessage {
  role: 'user' | 'assistant';
  content: string;
}

export interface IChatCompletionOptions {
  apiKey: string;
  modelName: string;
  messages: IChatCompletionMessage[];
}

export interface IChatCompletionUpdate {
  type: 'start' | 'update' | 'end';
  success: boolean;
  message?: string;
  isNotification?: boolean;
}
