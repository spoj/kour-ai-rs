import { Resizable } from "re-resizable";
import "./LeftPane.css";

const formatFolderPath = (parts: string[]) => {
  if (parts.length <= 2) {
    return parts.join("/");
  }
  const condensedParts = parts
    .slice(0, -2)
    .map((part) => part.charAt(0));
  return [...condensedParts, ...parts.slice(-2)].join("/");
};

type LeftPaneProps = {
  leftPaneWidth: number;
  setLeftPaneWidth: (width: number) => void;
  searchTerm: string;
  setSearchTerm: (term: string) => void;
  fileList: string[];
  searchInputRef: React.RefObject<HTMLInputElement>;
};

export const LeftPane = ({
  leftPaneWidth,
  setLeftPaneWidth,
  searchTerm,
  setSearchTerm,
  fileList,
  searchInputRef,
}: LeftPaneProps) => (
  <Resizable
    className="left-pane"
    size={{ width: leftPaneWidth, height: "100%" }}
    onResizeStop={(_e, _direction, _ref, d) => {
      setLeftPaneWidth(leftPaneWidth + d.width);
    }}
    minWidth={200}
    maxWidth={800}
    enable={{ right: true }}
  >
    <div className="left-pane-container">
      <div className="search-container">
        <svg
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          className="search-icon"
        >
          <circle cx="11" cy="11" r="8" />
          <line x1="21" y1="21" x2="16.65" y2="16.65" />
        </svg>
        <input
          ref={searchInputRef}
          type="text"
          placeholder="Search"
          value={searchTerm}
          onChange={(e) => setSearchTerm(e.target.value)}
          className="search-input"
        />
        {searchTerm && (
          <button onClick={() => setSearchTerm("")} className="clear-button">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        )}
      </div>

      {searchTerm && (
        <p className="results-count">{fileList.length} results</p>
      )}

      <ul className="file-list">
        {fileList.length > 0 ? (
          fileList.map((file) => {
            const parts = file.split(/[\\/]/);
            const fileName = parts.pop() || file;
            const folderPath = formatFolderPath(parts);

            return (
              <li key={file} title={file} className="file-list-item">
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  className="file-icon"
                >
                  <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" />
                  <polyline points="14 2 14 8 20 8" />
                </svg>
                <span className="file-name">{fileName}</span>
                {folderPath && (
                  <span className="folder-path">{folderPath}</span>
                )}
              </li>
            );
          })
        ) : (
          <h2>Please start by selecting a folder</h2>
        )}
      </ul>
    </div>
  </Resizable>
);