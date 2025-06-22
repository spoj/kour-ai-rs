import { useState } from "react";
import { createRoot } from "react-dom/client";
import { SettingsModal } from "../components";
import "./app.css";

export const App = () => {
  const [openSettingsModal, setOpenSettingsModal] = useState(false);
  const [message, setMessage] = useState("");

  const handleSendMessage = () => {
    console.log("Sending message:", message);
    setMessage("");
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
      <div id="chat-container"></div>
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
