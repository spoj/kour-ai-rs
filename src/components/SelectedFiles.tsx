import React, { useState } from 'react';

type SelectedFilesProps = {
  selectedFiles: string[];
  onFileSelect: (file: string) => void;
};

const SelectedFiles: React.FC<SelectedFilesProps> = ({ selectedFiles, onFileSelect }) => {
  const [isExpanded, setIsExpanded] = useState(false);

  if (selectedFiles.length === 0) {
    return null;
  }

  const toggleExpand = () => setIsExpanded(!isExpanded);

  return (
    <div className="selected-files-container">
      <div className="selected-files-header" onClick={toggleExpand}>
        <span>{selectedFiles.length} file{selectedFiles.length > 1 ? 's' : ''} selected</span>
        <button className="expand-button">
          {isExpanded ? '▼' : '▲'}
        </button>
      </div>
      {isExpanded && (
        <ul className="selected-files-list">
          {selectedFiles.map((file) => (
            <li key={file} onClick={() => onFileSelect(file)}>
              {file}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
};

export default SelectedFiles;