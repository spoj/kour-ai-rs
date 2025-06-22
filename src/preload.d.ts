export {};

declare global {
  interface Window {
    electron: {
      get: (key: string) => any;
      set: (key: string, val: any) => void;
      // any other methods you've defined...
    };
  }
}
