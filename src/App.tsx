import { useState, useEffect } from "react";
import "./App.css";
import { chatCompletion, getSettings, saveSettings } from "./commands";
import { IChatCompletionMessage, ISettings } from "./types";
import { ChatBubble } from "./components/ChatBubble";
import { SettingsModal } from "./components/SettingsModal";

function App() {
  const [messages, setMessages] = useState<IChatCompletionMessage[]>([]);
  const [input, setInput] = useState("");
  const [openSettingsModal, setOpenSettingsModal] = useState(false);
  const [isTyping, setIsTyping] = useState(false);
  const [settings, setSettings] = useState<ISettings>({
    apiKey: "",
    modelName: "",
    rootDir: "",
    systemPrompt: "",
    sofficePath: "",
    providerOrder: ""
  });

  useEffect(() => {
    getSettings().then(setSettings);
  }, []);

  const handleSettingsChange = (newSettings: Partial<ISettings>) => {
    const updatedSettings = { ...settings, ...newSettings };
    setSettings(updatedSettings);
    saveSettings(updatedSettings);
  };

  const handleSend = async () => {
    const newMessages: IChatCompletionMessage[] = [
      ...messages,
      { role: "user", content: input },
    ];
    setMessages(newMessages);
    setInput("");
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
              content: update.message,
              isNotification: update.is_notification,
            };
            setMessages((prev) => [...prev, botMessage]);
            break;
        }
      }
    );
  };

  const handleKeyDown = (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      handleSend();
    }
  };

  return (
    <div className="container">
      <header>
        <h1>Kour-AI</h1>
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
          <button id="header-button" title="Restart Session">
            Restart
          </button>
          <button
            id="header-button"
            title="Settings"
            onClick={() => setOpenSettingsModal(true)}
          >
            Settings
          </button>
        </div>
      </header>
      <div id="chat-container">
        {messages.map((m, i) => (
          <ChatBubble key={i} role={m.role} content={m.content} isNotification={m.isNotification} />
        ))}
        {isTyping && <ChatBubble role="assistant" content="Thinking..." isNotification />}
      </div>
      <div id="input-container">
        <textarea
          id="message-input"
          placeholder="Type a message..."
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
        ></textarea>
        <button id="send-button" onClick={handleSend}>Send</button>
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
