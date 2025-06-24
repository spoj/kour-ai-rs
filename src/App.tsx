import { useState, useEffect, useRef } from "react";
import { FaCog, FaPaperPlane, FaRedo } from "react-icons/fa";
import "./App.css";
import { chatCompletion, getSettings, saveSettings } from "./commands";
import { IChatCompletionMessage, ISettings } from "./types";
import { ChatBubble } from "./components/ChatBubble";
import { SettingsModal } from "./components/SettingsModal";

function App() {
  const [messages, setMessages] = useState<IChatCompletionMessage[]>([]);
  const [input, setInput] = useState("");
  const [attachments, setAttachments] = useState<string[]>([]);
  const chatContainerRef = useRef<HTMLDivElement>(null);
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
  }, []);

  useEffect(() => {
    if (chatContainerRef.current) {
      chatContainerRef.current.scrollTop = chatContainerRef.current.scrollHeight;
    }
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

    const newMessages: IChatCompletionMessage[] = [
      ...messages,
      { role: "user", content },
    ];

    setMessages(newMessages);
    setInput("");
    setAttachments([]);

    chatCompletion(
      {
        apiKey: settings.apiKey,
        modelName: settings.modelName,
        messages: newMessages,
      },
      (update) => {
        switch (update.type) {
          case "Start":
            setIsTyping(true);
            break;
          case "End":
            setIsTyping(false);
            break;
          case "Update":
            const botMessage: IChatCompletionMessage = {
              role: "assistant",
              content: [{ type: "text", text: update.message }],
              isNotification: update.is_notification,
            };
            setMessages((prev) => [...prev, botMessage]);
            break;
        }
      }
    );
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

  const handleDelete = (index: number) => {
    setMessages((prev) => prev.filter((_, i) => i !== index));
  };

  return (
    <div className="container">
      <header>
        <a href="/" style={{ color: "white", textDecoration: "none" }}>
          <h1>Kour-AI</h1>
        </a>
        <div id="path-container">
          <input
            type="text"
            id="path-input"
            placeholder="Enter root directory..."
            value={settings.rootDir}
            onChange={(e) => handleSettingsChange({ rootDir: e.target.value })}
          />
        </div>
        <div style={{ paddingLeft: "10px" }}>
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
            onDelete={() => handleDelete(i)}
          />
        ))}
        {isTyping && (
          <ChatBubble
            role="assistant"
            content={[{ type: "text", text: "Thinking..." }]}
            isNotification
            onCopy={() => { }}
            onDelete={() => { }}
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
