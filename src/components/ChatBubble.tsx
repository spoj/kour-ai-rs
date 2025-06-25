import { useState } from "react";
import { IChatCompletionMessage, MessageContent } from "../types";
import ReactMarkdown from "react-markdown";
import { FaCopy, FaTrash, FaAngleDown, FaAngleUp } from "react-icons/fa";
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
  toolArgs,
  toolResult,
}: IChatCompletionMessage & {
  onCopy: () => void;
  onDelete: () => void;
}) => {
  const [isExpanded, setIsExpanded] = useState(false);

  const isTool = !!(toolArgs || toolResult);

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
        className={`chat-bubble ${role} ${
          isNotification ? "notification" : ""
        }`}
      >
        <div className="chat-bubble-content-wrapper">
          {isTool && (
            <button
              className="expand-button"
              onClick={() => setIsExpanded(!isExpanded)}
            >
              {isExpanded ? <FaAngleUp /> : <FaAngleDown />}
            </button>
          )}
          {renderContent(content)}
        </div>
        {isTool && isExpanded && (
          <div className="tool-details-content">
            {toolArgs && (
              <pre>
                <b>Arguments:</b> {toolArgs}
              </pre>
            )}
            {toolResult && (
              <pre>
                <b>Result:</b> {toolResult.substring(0, 300)}
              </pre>
            )}
          </div>
        )}
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