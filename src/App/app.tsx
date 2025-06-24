import { useState } from "react";
import { createRoot } from "react-dom/client";
import { ChatBubble, SettingsModal } from "../components";
import "./app.css";
import { IMessage } from "../components/components";

export const App = () => {
  const [openSettingsModal, setOpenSettingsModal] = useState(false);
  const [message, setMessage] = useState("");
  const [chatHistory, setChatHistory] = useState<IMessage[]>([]);

  const handleSendMessage = () => {
    if (message.trim()) {
      const userMessage: IMessage = { role: 'user', content: message };
      setChatHistory(prevChatHistory => [...prevChatHistory, userMessage]);
      setMessage('');

      // Add a hardcoded response after a short delay
      setTimeout(() => {
        const responses = [
          "I'm just a demo, so I don't have much to say.",
          "That's interesting! Tell me more.",
          "I see. What else is on your mind?",
          "Thanks for sharing.",
        ];
        const randomIndex = Math.floor(Math.random() * responses.length);
        const botMessage: IMessage = {
          role: 'assistant',
          content: responses[randomIndex],
        };

        setChatHistory(prevChatHistory => [...prevChatHistory, botMessage]);
      }, 1000);
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
