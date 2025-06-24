import { IChatCompletionOptions, ISettings, IChatCompletionUpdate } from './main';

export {};

declare global {
  interface Window {
    electron: {
      get: (key: keyof ISettings) => string;
      set: (key: keyof ISettings, val: string) => void;
      getSettings: () => ISettings;
      setSettings: (val: ISettings) => void;
      chatCompletion: (options: IChatCompletionOptions, callback: (update: IChatCompletionUpdate) => void) => void;
    };
  }
}
