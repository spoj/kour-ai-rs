import { useState } from "react";
import { ensureLibreoffice } from "../commands";
import "./components.css";
import { ISettings } from "../types";

export const SettingsModal = ({
  settings,
  onClose,
  onSave,
}: {
  settings: ISettings;
  onClose: Function;
  onSave: (settings: Partial<ISettings>) => void;
}) => {
  const [isDownloading, setIsDownloading] = useState(false);
  const handleModalClick = (e: React.MouseEvent) => {
    e.stopPropagation();
  };

  const handleClose = () => {
    onClose(false);
  };

  const handleEnsureLibreoffice = async () => {
    setIsDownloading(true);
    await ensureLibreoffice();
    setIsDownloading(false);
  };

  return (
    <div className="settings-background" onClick={handleClose}>
      <div className="settings-modal " onClick={handleModalClick}>
        <div className="close-button" onClick={handleClose}>
          Close
        </div>
        <h2>Settings</h2>
        <div style={{ display: "flex", flexDirection: "column" }}>
          <label htmlFor="apiKey">OpenRouter API Key:</label>
          <input
            type="password"
            value={settings.apiKey}
            onChange={(e) => onSave({ apiKey: e.target.value })}
          />
          <label htmlFor="modelName">Model Name:</label>
          <input
            type="text"
            value={settings.modelName}
            onChange={(e) => onSave({ modelName: e.target.value })}
          />
          <label htmlFor="sofficePath">LibreOffice Path (soffice.com):</label>
          <input
            type="text"
            style={{ marginBottom: 0 }}
            placeholder="e.g., C:\Program Files\LibreOffice\program\soffice.com"
            value={settings.sofficePath}
            onChange={(e) => onSave({ sofficePath: e.target.value })}
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
            <button
              onClick={handleEnsureLibreoffice}
              disabled={isDownloading}
              style={{ marginLeft: "10px" }}
            >
              {isDownloading
                ? "Downloading/Installing..."
                : "Install LibreOffice automatically"}
            </button>
          </small>
          <label htmlFor="providerOrder">Provider Order:</label>
          <input
            type="text"
            placeholder="e.g., google-vertex,anthropic,openai"
            value={settings.providerOrder}
            onChange={(e) => onSave({ providerOrder: e.target.value })}
          />
        </div>
      </div>
    </div>
  );
};