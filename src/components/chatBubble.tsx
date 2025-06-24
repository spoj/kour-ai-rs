import { IMessage } from "./components";

export const ChatBubble = ({ role, content }: IMessage) => {
  return (
    <div className={`chat-bubble ${role}`}>
      <p>{content}</p>
    </div>
  );
};
