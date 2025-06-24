import { IMessage } from "./components";

export const ChatBubble = ({ role, content, isNotification }: IMessage) => {
  return (
    <div className={`chat-bubble ${role} ${isNotification ? 'notification' : ''}`}>
      <p>{content}</p>
    </div>
  );
};
