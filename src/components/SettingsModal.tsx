import { useState } from "react";
import "./components.css";
import { ISettings } from "../types";

export const SettingsModal = ({
  settings,
  onClose,
  onSave,
}: {
  settings: ISettings;
  onClose: Function;
  onSave: (settings: ISettings) => void;
}) => {
  const [localSettings, setLocalSettings] = useState<ISettings>(settings);

  const handleModalClick = (e: React.MouseEvent) => {
    e.stopPropagation();
  };

  const handleSave = () => {
    onSave(localSettings);
    onClose(false);
  };

  const handleClose = () => {
    onClose(false);
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
            value={localSettings.apiKey}
            onChange={(e) =>
              setLocalSettings((old) => ({ ...old, apiKey: e.target.value }))
            }
          />
          <label htmlFor="modelName">Model Name:</label>
          <input
            type="text"
            value={localSettings.modelName}
            onChange={(e) =>
              setLocalSettings((old) => ({ ...old, modelName: e.target.value }))
            }
          />
          <label htmlFor="system-prompt">System Prompt:</label>
          <textarea
            id="system-prompt"
            value={localSettings.systemPrompt}
            onChange={(e) =>
              setLocalSettings((old) => ({
                ...old,
                systemPrompt: e.target.value,
              }))
            }
          ></textarea>
          <label htmlFor="sofficePath">LibreOffice Path (soffice.com):</label>
          <input
            type="text"
            style={{ marginBottom: 0 }}
            placeholder="e.g., C:\Program Files\LibreOffice\program\soffice.com"
            value={localSettings.sofficePath}
            onChange={(e) =>
              setLocalSettings((old) => ({
                ...old,
                sofficePath: e.target.value,
              }))
            }
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
            value={localSettings.providerOrder}
            onChange={(e) =>
              setLocalSettings((old) => ({
                ...old,
                providerOrder: e.target.value,
              }))
            }
          />
        </div>
        <div id="modal-footer">
          <button onClick={handleClose}>Cancel</button>
          <button id="save-button" onClick={handleSave}>
            Save
          </button>
        </div>
      </div>
    </div>
  );
};