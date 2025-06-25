import { useState, useEffect, useRef } from "react";
import { FaCog, FaPaperPlane, FaTrash } from "react-icons/fa";
import "./App.css";
import { chatCompletion, getSettings, saveSettings, replayHistory, clearHistory, onChatCompletionUpdate } from "./commands";
import { IChatCompletionMessage, ISettings } from "./types";
import { ChatBubble } from "./components/ChatBubble";
import { SettingsModal } from "./components/SettingsModal";

function App() {
  const [messages, setMessages] = useState<IChatCompletionMessage[]>([]);
  const [input, setInput] = useState("");
  const [attachments, setAttachments] = useState<string[]>([]);
  const chatContainerRef = useRef<HTMLDivElement>(null);
  const rootDirInputRef = useRef<HTMLInputElement>(null);
  const messageInputRef = useRef<HTMLTextAreaElement>(null);
  const hasReplayed = useRef(false);
  const [openSettingsModal, setOpenSettingsModal] = useState(false);
  const [isTyping, setIsTyping] = useState(false);
  const [settings, setSettings] = useState<ISettings>({
    apiKey: "",
    modelName: "",
    rootDir: "",
    sofficePath: "",
    providerOrder: ""
  });

  useEffect(() => {
    getSettings().then(setSettings);
    messageInputRef.current?.focus();

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "l" && e.ctrlKey) {
        e.preventDefault();
        rootDirInputRef.current?.focus();
      } else if (e.key === "k" && e.ctrlKey) {
        e.preventDefault();
        clearHistory().then(() => setMessages([]));
      }
    };

    window.addEventListener("keydown", handleKeyDown);

    const unlisten = onChatCompletionUpdate((update) => {
      switch (update.type) {
        case "Start":
          setIsTyping(true);
          break;
        case "End":
          setIsTyping(false);
          break;
        case "Message":
          setMessages((prev) => [...prev, update.message]);
          break;
        case "ToolCall":
          const toolCallMessage: IChatCompletionMessage = {
            tool_call_id: update.tool_call_id,
            role: "assistant",
            content: [],
            isNotification: true,
            toolName: update.tool_name,
            toolArgs: update.tool_args,
          };
          setMessages((prev) => [...prev, toolCallMessage]);
          break;
        case "ToolDone":
          setMessages((prev) => {
            // Find the most recent tool call with matching ID that doesn't have a result yet
            let foundIndex = -1;
            for (let i = prev.length - 1; i >= 0; i--) {
              if (prev[i].tool_call_id === update.tool_call_id && !prev[i].toolResult) {
                foundIndex = i;
                break;
              }
            }

            if (foundIndex !== -1) {
              return prev.map((m, index) =>
                index === foundIndex
                  ? {
                    ...m,
                    toolResult: update.tool_result,
                  }
                  : m
              );
            }

            // Fallback to original behavior if no match found
            return prev.map((m) =>
              m.tool_call_id === update.tool_call_id
                ? {
                  ...m,
                  toolResult: update.tool_result,
                }
                : m
            );
          });
          break;
      }
    });

    if (!hasReplayed.current) {
      // Clear messages before replaying history to prevent duplication
      setMessages([]);
      replayHistory();
      hasReplayed.current = true;
    }

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      unlisten.then((f) => f());
    };
  }, []);

  useEffect(() => {
    setTimeout(() => {
      if (chatContainerRef.current) {
        chatContainerRef.current.scrollTop = chatContainerRef.current.scrollHeight;
      }
    }, 100);
  }, [messages]);

  const handleSettingsChange = (newSettings: Partial<ISettings>) => {
    const updatedSettings = { ...settings, ...newSettings };
    setSettings(updatedSettings);
    saveSettings(updatedSettings);
  };

  const handleSend = async () => {
    if (input.trim() === "") return;
    let content: any = [{ type: "text", text: input }];
    if (attachments.length > 0) {
      content = [
        ...content,
        ...attachments.map((a) => ({
          type: "image_url",
          image_url: { url: a },
        })),
      ];
    }

    const userMessage: IChatCompletionMessage = { role: "user", content };

    setMessages((prev) => [...prev, userMessage]);
    setInput("");
    setAttachments([]);

    chatCompletion(userMessage);
  };

  const handleKeyDown = (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      handleSend();
    }
  };

  const handlePaste = (event: React.ClipboardEvent<HTMLTextAreaElement>) => {
    const items = event.clipboardData.items;
    for (let i = 0; i < items.length; i++) {
      if (items[i].type.indexOf("image") !== -1) {
        const file = items[i].getAsFile();
        if (file) {
          const reader = new FileReader();
          reader.onload = (e) => {
            if (e.target?.result) {
              setAttachments((prev) => [...prev, e.target?.result as string]);
            }
          };
          reader.readAsDataURL(file);
        }
      }
    }
  };

  const handleCopy = (content: IChatCompletionMessage["content"]) => {
    const textToCopy = content
      .filter((item) => item.type === "text")
      .map((item: any) => item.text)
      .join("\n");
    navigator.clipboard.writeText(textToCopy);
  };


  return (
    <div className="container">
      <header>
        <a href="/" style={{ color: "white", textDecoration: "none" }}>
          <h1>Kour-AI</h1>
        </a>
        <div id="path-container">
          <input
            ref={rootDirInputRef}
            type="text"
            id="path-input"
            placeholder="Enter root directory..."
            value={settings.rootDir}
            onChange={(e) => handleSettingsChange({ rootDir: e.target.value })}
            onFocus={(e) => e.target.select()}
          />
        </div>
        <div style={{ paddingLeft: "10px" }}>
          <button
            id="header-button"
            title="Clear History"
            onClick={() => clearHistory().then(() => setMessages([]))}
          >
            <FaTrash />
          </button>
          <button
            id="header-button"
            title="Settings"
            onClick={() => setOpenSettingsModal(true)}
          >
            <FaCog />
          </button>
        </div>
      </header>
      <div id="chat-container" ref={chatContainerRef}>
        {messages.map((m, i) => (
          <ChatBubble
            key={i}
            role={m.role}
            content={m.content}
            isNotification={m.isNotification}
            onCopy={() => handleCopy(m.content)}
            toolName={m.toolName}
            toolArgs={m.toolArgs}
            toolResult={m.toolResult}
          />
        ))}
        {isTyping && (
          <ChatBubble
            role="assistant"
            content={[{ type: "text", text: "Thinking..." }]}
            isNotification
            onCopy={() => { }}
          />
        )}
      </div>
      <div id="input-container">
        {attachments.map((a, i) => (
          <img
            key={i}
            src={a}
            alt="attachment"
            className="attachment-thumbnail"
            onClick={() => setAttachments((prev) => prev.filter((_, j) => i !== j))}
          />
        ))}
        <textarea
          ref={messageInputRef}
          id="message-input"
          placeholder="Type a message..."
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          onPaste={handlePaste}
        ></textarea>
        <button id="send-button" onClick={handleSend}>
          <FaPaperPlane />
        </button>
      </div>
      {openSettingsModal && (
        <SettingsModal
          settings={settings}
          onClose={() => setOpenSettingsModal(false)}
          onSave={handleSettingsChange}
        />
      )}
    </div>
  );
}

export default App;
