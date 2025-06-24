import { IChatCompletionMessage } from "../types";
import "./components.css";

export const ChatBubble = ({ role, content, isNotification }: IChatCompletionMessage) => {
  return (
    <div className={`chat-bubble ${role} ${isNotification ? 'notification' : ''}`}>
      <p>{content}</p>
    </div>
  );
};