import { useEffect, useState } from "react";
import { platform } from "@tauri-apps/plugin-os";
import { ensureLibreoffice, onLibreofficeUpdate } from "../commands";
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
  const [status, setStatus] = useState<string>("");
  const [os, setOs] = useState("");

  useEffect(() => {
    const getPlatform = async () => {
      setOs(platform());
    };
    getPlatform();
    const unlisten = onLibreofficeUpdate((update) => {
      if (update.type === "Downloading") {
        setStatus("Downloading...");
      } else if (update.type === "Installing") {
        setStatus("Installing...");
      } else if (update.type === "Success") {
        onSave({ sofficePath: update.payload });
        setStatus("Installation successful!");
        setIsDownloading(false);
      } else if (update.type === "Error") {
        setStatus(`Error: ${update.payload}`);
        setIsDownloading(false);
      }
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  const handleModalClick = (e: React.MouseEvent) => {
    e.stopPropagation();
  };

  const handleClose = () => {
    onClose(false);
  };

  const handleEnsureLibreoffice = async () => {
    setIsDownloading(true);
    setStatus("");
    await ensureLibreoffice();
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
            {os === "windows" && (
              <>
                <button
                  onClick={handleEnsureLibreoffice}
                  disabled={isDownloading}
                  style={{ marginLeft: "10px" }}
                >
                  {isDownloading
                    ? status || "Starting..."
                    : "Install LibreOffice automatically"}
                </button>
                {status && (
                  <small
                    style={{
                      marginLeft: "10px",
                      color: status.startsWith("Error") ? "red" : "green",
                    }}
                  >
                    {status}
                  </small>
                )}
              </>
            )}
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