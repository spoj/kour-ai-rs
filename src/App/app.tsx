import { useState } from "react";
import { createRoot } from "react-dom/client";
import { ChatBubble, SettingsModal } from "../components";
import "./app.css";
import { IMessage } from "../components/components";

export const App = () => {
  const [openSettingsModal, setOpenSettingsModal] = useState(false);
  const [message, setMessage] = useState("");
  const [chatHistory, setChatHistory] = useState<IMessage[]>([]);
  const [isTyping, setIsTyping] = useState(false);

  const handleSendMessage = async () => {
    if (message.trim()) {
      const userMessage: IMessage = { role: 'user', content: message };
      const newChatHistory = [...chatHistory, userMessage];
      setChatHistory(newChatHistory);
      setMessage('');

      const settings = window.electron.getSettings();

      const result = await window.electron.chatCompletion({
        apiKey: settings.apiKey,
        modelName: settings.modelName,
        messages: newChatHistory.map(m => ({ role: m.role, content: m.content })),
      });

      const botMessage: IMessage = {
        role: 'assistant',
        content: result.message,
        isNotification: !result.success,
      };

      setChatHistory(prev => [...prev, botMessage]);
    }
  };

  const handleKeyDown = (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      handleSendMessage();
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
          />
        </div>
        <div style={{ paddingLeft: "10px" }}>
          <button id="header-button" title="Restart Session">
            Restart
          </button>
          <button
            id="header-button"
            title="Settings"
            onClick={() => setOpenSettingsModal(!openSettingsModal)}
          >
            Settings
          </button>
        </div>
      </header>
      <div id="chat-container">
        {chatHistory.map((chat, index) => (
          <ChatBubble key={index} role={chat.role} content={chat.content} />
        ))}
        {isTyping && <ChatBubble role="assistant" content="Thinking..." isNotification />}
      </div>
      <div id="input-container">
        <textarea
          id="message-input"
          placeholder="Type a message..."
          value={message}
          onChange={(e) => setMessage(e.target.value)}
          onKeyDown={handleKeyDown}
        ></textarea>
        <button id="send-button" onClick={handleSendMessage}>Send</button>
      </div>
      {openSettingsModal && <SettingsModal onClose={setOpenSettingsModal} />}
    </div>
  );
};

const root = createRoot(document.body);
root.render(<App />);
