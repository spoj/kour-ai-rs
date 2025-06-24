import { IChatCompletionMessage, MessageContent } from "../types";
import "./components.css";

const renderContent = (content: MessageContent) => {
  return content.map((item, index) => {
    if (item.type === "text") {
      return <p key={index}>{item.text}</p>;
    } else if (item.type === "image_url") {
      return (
        <img
          key={index}
          src={item.image_url.url}
          alt="attachment"
          className="chat-image"
        />
      );
    }
    return null;
  });
};

export const ChatBubble = ({
  role,
  content,
  isNotification,
}: IChatCompletionMessage) => {
  return (
    <div className={`chat-bubble ${role} ${isNotification ? "notification" : ""}`}>
      {renderContent(content)}
    </div>
  );
};