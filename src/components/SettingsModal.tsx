import { useState, useEffect } from "react";
import "./components.css";
import { getSettings, setSettings } from "../commands";
import { ISettings } from "../types";

export const SettingsModal = ({ onClose }: { onClose: Function }) => {
  const handleModalClick = (e: React.MouseEvent) => {
    e.stopPropagation();
  };

  const [settings, setSettingsState] = useState<ISettings>({
    apiKey: "",
    modelName: "",
    providerOrder: "",
    sofficePath: "",
    systemPrompt: "",
    rootDir: "",
  });

  useEffect(() => {
    getSettings().then(setSettingsState);
  }, []);

  const handleClose = () => {
    setSettings(settings);
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
            value={settings.apiKey}
            onChange={(e) =>
              setSettingsState((old) => {
                return { ...old, apiKey: e.target.value };
              })
            }
          />
          <label htmlFor="modelName">Model Name:</label>
          <input
            type="text"
            value={settings.modelName}
            onChange={(e) =>
              setSettingsState((old) => {
                return { ...old, modelName: e.target.value };
              })
            }
          />
          <label htmlFor="system-prompt">System Prompt:</label>
          <textarea
            id="system-prompt"
            value={settings.systemPrompt}
            onChange={(e) =>
              setSettingsState((old) => {
                return { ...old, systemPrompt: e.target.value };
              })
            }
          ></textarea>
          <label htmlFor="sofficePath">LibreOffice Path (soffice.com):</label>
          <input
            type="text"
            style={{ marginBottom: 0 }}
            placeholder="e.g., C:\Program Files\LibreOffice\program\soffice.com"
            value={settings.sofficePath}
            onChange={(e) =>
              setSettingsState((old) => {
                return { ...old, sofficePath: e.target.value };
              })
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
            value={settings.providerOrder}
            onChange={(e) =>
              setSettingsState((old) => {
                return { ...old, providerOrder: e.target.value };
              })
            }
          />
        </div>
        <div id="modal-footer">
          <button onClick={handleClose}>Cancel</button>
          <button
            id="save-button"
            onClick={handleClose}
          >
            Save
          </button>
        </div>
      </div>
    </div>
  );
};