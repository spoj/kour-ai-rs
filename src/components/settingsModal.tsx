import "./components.css";

export const SettingsModal = ({ onClose }: { onClose: Function }) => {
  const handleModalClick = (e: React.MouseEvent) => {
    e.stopPropagation();
  };
  return (
    <div className="settings-background" onClick={() => onClose(false)}>
      <div className="settings-modal " onClick={handleModalClick}>
        <div className="close-button" onClick={() => onClose(false)}>
          Close
        </div>
        <h2>Settings</h2>
        <div style={{ display: "flex", flexDirection: "column" }}>
          <label htmlFor="apiKey">OpenRouter API Key:</label>
          <input type="password" v-model="settings.apiKey" />
          <label htmlFor="modelName">Model Name:</label>
          <input type="text" v-model="settings.modelName" />
          <label htmlFor="system-prompt">System Prompt:</label>
          <textarea
            id="system-prompt"
            v-model="settings.systemPrompt"
          ></textarea>
          <label htmlFor="sofficePath">LibreOffice Path (soffice.com):</label>
          <input
            type="text"
            style={{ marginBottom: 0 }}
            placeholder="e.g., C:\Program Files\LibreOffice\program\soffice.com"
          />
          <small
            style={{
              color: "#666",
              display: "block",
              marginTop: "5px",
              marginBottom: "10px",
            }}
          >
            Optional: Set this to enable DOCX/PPTX support. Leave empty if
            LibreOffice is not installed.
          </small>
          <label htmlFor="providerOrder">Provider Order:</label>
          <input
            type="text"
            placeholder="e.g., google-vertex,anthropic,openai"
          />
        </div>
      </div>
    </div>
  );
};
