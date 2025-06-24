import { IChatCompletionMessage, MessageContent } from "../types";
import ReactMarkdown from "react-markdown";
import { FaCopy, FaTrash } from "react-icons/fa";
import "./components.css";

const renderContent = (content: MessageContent) => {
  return content.map((item, index) => {
    if (item.type === "text") {
      return <ReactMarkdown key={index}>{item.text}</ReactMarkdown>;
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
  onCopy,
  onDelete,
}: IChatCompletionMessage & {
  onCopy: () => void;
  onDelete: () => void;
}) => {
  return (
    <div className={`chat-bubble-container ${role}`}>
      {role === "user" && !isNotification && (
        <div className="message-actions">
          <button onClick={onCopy} title="Copy">
            <FaCopy />
          </button>
          <button onClick={onDelete} title="Delete">
            <FaTrash />
          </button>
        </div>
      )}
      <div
        className={`chat-bubble ${role} ${isNotification ? "notification" : ""}`}
      >
        {renderContent(content)}
      </div>
      {role === "assistant" && !isNotification && (
        <div className="message-actions">
          <button onClick={onCopy} title="Copy">
            <FaCopy />
          </button>
          <button onClick={onDelete} title="Delete">
            <FaTrash />
          </button>
        </div>
      )}
    </div>
  );
};