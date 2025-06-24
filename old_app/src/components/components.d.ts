export interface IMessage {
  role: "assistant" | "user";
  content: string;
  isNotification?: boolean;
}
