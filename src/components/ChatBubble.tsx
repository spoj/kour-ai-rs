import { useState } from "react";
import { IChatCompletionMessage, MessageContent } from "../types";
import ReactMarkdown from "react-markdown";
import { FaCopy, FaAngleDown, FaAngleUp, FaDownload } from "react-icons/fa";
import "./components.css";
import { base64toBlob } from "../helpers";

const renderContent = (content: MessageContent) => {
  if (!content || !content.length) {
    return null;
  }

  return content.map((item, index) => {
    if (item.type === "text") {
      return (
        <ReactMarkdown
          key={index}
          components={{
            a: ({ node, ...props }) => (
              <a {...props} target="_blank" rel="noopener noreferrer" />
            ),
          }}
        >
          {item.text}
        </ReactMarkdown>
      );
    } else if (item.type === "image_url") {
      return (
        <img
          key={index}
          src={item.image_url.url}
          alt="attachment"
          className="chat-image"
        />
      );
    } else if (item.type === "file") {
      if (item.file.file_data.split(";base64,")[1]) {
        const blobVal = base64toBlob(
          item.file.file_data.split(";base64,")[1],
          item.file.file_data.split(";base54,")[0].split(":")[1]
        );
        return (
          <a
            key={index}
            download={item.file.filename}
            href={URL.createObjectURL(blobVal)}
            target="_blank"
            rel="noopener noreferrer"
            className="file-attachment-link"
          >
            <button>
              <FaDownload style={{ marginRight: "5px" }} />
              {item.file.filename}
            </button>
          </a>
        );
      }
    }
    return null;
  });
};

export const ChatBubble = ({
  role,
  content,
  isNotification,
  onCopy,
  toolName,
  toolArgs,
  toolResult,
}: IChatCompletionMessage & {
  onCopy?: () => void;
}) => {
  const [isExpanded, setIsExpanded] = useState(false);

  // A bubble is a tool bubble if it has a tool name, or if it already has args/results from history
  const isTool = !!(toolName || toolArgs || toolResult);

  const mainContent = toolName
    ? `Calling ${toolName}${toolResult ? " done." : ""}`
    : renderContent(content);

  return (
    <div className={`chat-bubble-container ${role}`}>
      {role === "user" && !isNotification && (
        <div className="message-actions">
          <button onClick={onCopy} title="Copy">
            <FaCopy />
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
          <div className="chat-content">{mainContent}</div>
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
        </div>
      )}
    </div>
  );
};
