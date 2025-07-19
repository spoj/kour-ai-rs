import { FaCog, FaTrash, FaFolderOpen } from "react-icons/fa";
import { ISettings } from "../types";

type TopBarProps = {
  appVersion: string;
  settings: ISettings;
  handleSettingsChange: (newSettings: Partial<ISettings>) => void;
  onClearHistory: () => void;
  onOpenSettings: () => void;
  onSelectFolder: () => void;
  rootDirInputRef: React.RefObject<HTMLInputElement>;
};

export const TopBar = ({
  appVersion,
  settings,
  handleSettingsChange,
  onClearHistory,
  onOpenSettings,
  onSelectFolder,
  rootDirInputRef,
}: TopBarProps) => (
  <header>
    <a href="/" style={{ color: "white", textDecoration: "none" }}>
      <h1 title={`version: ${appVersion}`}>Kour-AI</h1>
    </a>
    <div id="path-container">
      <input
        ref={rootDirInputRef}
        type="text"
        id="path-input"
        placeholder="Enter root directory..."
        value={settings.rootDir}
        onChange={(e) => handleSettingsChange({ rootDir: e.target.value })}
        onFocus={(e) => e.target.select()}
      />
      <button id="header-button" title="Select folder" onClick={onSelectFolder}>
        <FaFolderOpen />
      </button>
    </div>
    <div style={{ paddingLeft: "10px" }}>
      <button id="header-button" title="Clear History" onClick={onClearHistory}>
        <FaTrash />
      </button>
      <button id="header-button" title="Settings" onClick={onOpenSettings}>
        <FaCog />
      </button>
    </div>
  </header>
);