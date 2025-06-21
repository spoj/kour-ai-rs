import { useState } from "react";
import { createRoot } from "react-dom/client";
import { SettingsModal } from "../components";
import "./app.css";

export const App = () => {
  const [openSettingsModal, setOpenSettingsModal] = useState(false);

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
        <textarea id="message-input" placeholder="Type a message..."></textarea>
        <button id="send-button">Send</button>
      </div>
      {openSettingsModal && <SettingsModal onClose={setOpenSettingsModal} />}
    </div>
  );
};

const root = createRoot(document.body);
root.render(<App />);
