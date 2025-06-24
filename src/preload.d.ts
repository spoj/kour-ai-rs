import { IChatCompletionOptions } from './main';

export {};

declare global {
  interface Window {
    electron: {
      get: (key: string) => any;
      set: (key: string, val: any) => void;
      getSettings: () => any;
      setSettings: (val: any) => void;
      chatCompletion: (options: IChatCompletionOptions) => Promise<{ success: boolean; message: string; }>;
    };
  }
}
