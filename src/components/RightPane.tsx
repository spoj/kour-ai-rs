import { Resizable } from "re-resizable";
import { FaPaperPlane, FaSquare, FaFile } from "react-icons/fa";
import { IChatCompletionMessage } from "../types";
import { ChatBubble } from "./ChatBubble";

type Attachment = {
  type: string; // Mime type e.g. "image/png"
  content: string; // data URL of the content
  filename: string;
};

type RightPaneProps = {
  messages: IChatCompletionMessage[];
  isTyping: boolean;
  onCopy: (content: IChatCompletionMessage["content"]) => void;
  onDelete: (id: number) => void;
  onDeleteTool: (llm_interaction_id: number, tool_call_id: string) => void;
  chatContainerRef: React.RefObject<HTMLDivElement>;
  attachments: Attachment[];
  setAttachments: React.Dispatch<React.SetStateAction<Attachment[]>>;
  input: string;
  setInput: (input: string) => void;
  handleKeyDown: (event: React.KeyboardEvent<HTMLTextAreaElement>) => void;
  handlePaste: (event: React.ClipboardEvent<HTMLTextAreaElement>) => void;
  handleSend: () => void;
  handleCancel: () => void;
  messageInputRef: React.RefObject<HTMLTextAreaElement>;
  rightPaneWidth: number;
  setRightPaneWidth: (width: number) => void;
};

export const RightPane = ({
  messages,
  isTyping,
  onCopy,
  onDelete,
  onDeleteTool,
  chatContainerRef,
  attachments,
  setAttachments,
  input,
  setInput,
  handleKeyDown,
  handlePaste,
  handleSend,
  handleCancel,
  messageInputRef,
  rightPaneWidth,
  setRightPaneWidth,
}: RightPaneProps) => (
  <Resizable
    className="right-pane"
    size={{ width: rightPaneWidth, height: "100%" }}
    onResizeStop={(_e, _direction, _ref, d) => {
      setRightPaneWidth(rightPaneWidth + d.width);
    }}
    minWidth={300}
    maxWidth={600}
    enable={{ left: true }}
  >
    <div id="chat-container" ref={chatContainerRef}>
      {messages
        .sort((a, b) => a.id - b.id)
        .map((m) => (
          <ChatBubble
            key={m.tool_call_id || m.id}
            {...m}
            onCopy={() => onCopy(m.content)}
            onDelete={() => onDelete(m.id)}
            onDeleteTool={(llm_interaction_id, tool_call_id) =>
              onDeleteTool(llm_interaction_id, tool_call_id)
            }
          />
        ))}
      {isTyping && (
        <ChatBubble
          id={0}
          role="assistant"
          content={[{ type: "text", text: "Thinking..." }]}
          isNotification
          onCopy={() => {}}
        />
      )}
    </div>
    <div id="input-container">
      <div id="attachment-container">
        {attachments.map((a, i) =>
          a.type.startsWith("image/") ? (
            <img
              key={i}
              src={a.content}
              alt={a.filename}
              title={a.filename}
              className="attachment-thumbnail"
              onClick={() =>
                setAttachments((prev) => prev.filter((_, j) => i !== j))
              }
            />
          ) : (
            <div
              key={i}
              title={a.filename}
              onClick={() =>
                setAttachments((prev) => prev.filter((_, j) => i !== j))
              }
            >
              <FaFile
                className="attachment-thumbnail"
                id="file-attachment"
              />
            </div>
          )
        )}
      </div>
      <div style={{ width: "100%", display: "flex" }}>
        <textarea
          ref={messageInputRef}
          id="message-input"
          placeholder="Type a message..."
          rows={1}
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          onPaste={handlePaste}
        ></textarea>
        {isTyping ? (
          <button
            className="send-button"
            id="stop-button"
            onClick={handleCancel}
          >
            <FaSquare />
          </button>
        ) : (
          <button className="send-button" onClick={handleSend}>
            <FaPaperPlane />
          </button>
        )}
      </div>
    </div>
  </Resizable>
);