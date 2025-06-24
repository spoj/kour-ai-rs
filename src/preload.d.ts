import { IChatCompletionOptions, ISettings } from './main';

export {};

declare global {
  interface Window {
    electron: {
      get: (key: keyof ISettings) => string;
      set: (key: keyof ISettings, val: string) => void;
      getSettings: () => ISettings;
      setSettings: (val: ISettings) => void;
      chatCompletion: (options: IChatCompletionOptions) => Promise<{ success: boolean; message: string; }>;
    };
  }
}
