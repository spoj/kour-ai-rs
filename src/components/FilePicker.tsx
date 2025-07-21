import { useState, useRef, useMemo } from "react";
import SelectedFiles from "./SelectedFiles";

type SortConfig = {
  key: "name" | "path";
  direction: "ascending" | "descending";
};

type FilePickerProps = {
  searchTerm: string;
  setSearchTerm: (term: string) => void;
  fileList: string[];
  searchInputRef: React.RefObject<HTMLInputElement>;
  selectedFiles: string[];
  onFileSelect: (file: string) => void;
  onSelectAll: () => void;
  onClearSelection: () => void;
  setSelectionRange: (files: string[], mode: "add" | "remove") => void;
};

export const FilePicker = ({
  searchTerm,
  setSearchTerm,
  fileList,
  searchInputRef,
  selectedFiles,
  onFileSelect,
  onSelectAll,
  onClearSelection,
  setSelectionRange,
}: FilePickerProps) => {
  const [sortConfig, setSortConfig] = useState<SortConfig | null>(null);
  const [isDragging, setIsDragging] = useState(false);
  const [dragStartFile, setDragStartFile] = useState<string | null>(null);
  const [initialSelection, setInitialSelection] = useState<string[]>([]);
  const fileListRef = useRef<HTMLTableElement>(null);

  const sortedFiles = useMemo(() => {
    const sortableItems = fileList.map((file) => {
      const parts = file.split(/[\\/]/);
      const name = parts.pop() || file;
      const path = parts.join("/");
      return { name, path, original: file };
    });

    if (sortConfig !== null) {
      sortableItems.sort((a, b) => {
        if (a[sortConfig.key] < b[sortConfig.key]) {
          return sortConfig.direction === "ascending" ? -1 : 1;
        }
        if (a[sortConfig.key] > b[sortConfig.key]) {
          return sortConfig.direction === "ascending" ? 1 : -1;
        }
        return 0;
      });
    }
    return sortableItems;
  }, [fileList, sortConfig]);

  const requestSort = (key: "name" | "path") => {
    let direction: "ascending" | "descending" = "ascending";
    if (
      sortConfig &&
      sortConfig.key === key &&
      sortConfig.direction === "ascending"
    ) {
      direction = "descending";
    }
    setSortConfig({ key, direction });
  };

  const handleMouseDown = (file: string) => {
    setIsDragging(true);
    setDragStartFile(file);
    setInitialSelection(selectedFiles);
    onFileSelect(file);
  };

  const handleMouseUp = () => {
    setIsDragging(false);
    setDragStartFile(null);
    setInitialSelection([]);
  };

  const handleMouseEnter = (file: string) => {
    if (isDragging && dragStartFile) {
      const flatFileList = sortedFiles.map((f) => f.original);
      const startIndex = flatFileList.indexOf(dragStartFile);
      const endIndex = flatFileList.indexOf(file);
      const [start, end] = [
        Math.min(startIndex, endIndex),
        Math.max(startIndex, endIndex),
      ];
      const filesToToggle = flatFileList.slice(start, end + 1);
      const filesToAdd = filesToToggle.filter(
        (f) => !initialSelection.includes(f)
      );
      const filesToRemove = filesToToggle.filter((f) =>
        initialSelection.includes(f)
      );
      setSelectionRange(filesToAdd, "add");
      setSelectionRange(filesToRemove, "remove");
    }
  };

  const getSortIndicator = (key: "name" | "path") => {
    if (!sortConfig || sortConfig.key !== key) {
      return null;
    }
    return sortConfig.direction === "ascending" ? " ▲" : " ▼";
  };

  return (
    <div className="file-picker">
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
          {(
            <button onClick={() => { setSearchTerm(""); onClearSelection(); }} className="clear-button">
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

        <div className="selection-buttons">
          <button onClick={onSelectAll}>Select All</button>
          <button onClick={onClearSelection}>Clear Selection</button>
        </div>
        <div className="file-table-container">
          <table className="file-table" ref={fileListRef}>
            <thead>
              <tr>
                <th onClick={() => requestSort("name")}>
                  Name{getSortIndicator("name")}
                </th>
                <th onClick={() => requestSort("path")}>
                  Path{getSortIndicator("path")}
                </th>
              </tr>
            </thead>
            <tbody onMouseUp={handleMouseUp} onMouseLeave={handleMouseUp}>
              {sortedFiles.length > 0 ? (
                sortedFiles.map(({ name, path, original }) => (
                  <tr
                    key={original}
                    title={original}
                    className={`file-list-item ${selectedFiles.includes(original) ? "selected" : ""
                      }`}
                    onMouseDown={() => handleMouseDown(original)}
                    onMouseEnter={() => handleMouseEnter(original)}
                  >
                    <td>{name}</td>
                    <td className="path-cell">{path}</td>
                  </tr>
                ))
              ) : (
                <tr>
                  <td colSpan={2} className="no-results">
                    no results
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>
      <SelectedFiles
        selectedFiles={selectedFiles}
        onFileSelect={onFileSelect}
      />
    </div>
  );
};