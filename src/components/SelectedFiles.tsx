import React from 'react';

type SelectedFilesProps = {
  selectedFiles: string[];
  onFileSelect: (file: string) => void;
};

const SelectedFiles: React.FC<SelectedFilesProps> = ({ selectedFiles, onFileSelect }) => {
  if (selectedFiles.length === 0) {
    return null;
  }

  return (
    <div className="selected-files-container">
      <div className="selected-files-header">
        <span>{selectedFiles.length} file{selectedFiles.length > 1 ? 's' : ''} selected</span>
      </div>
      <ul className="selected-files-list">
        {selectedFiles.map((file) => (
          <li key={file} onClick={() => onFileSelect(file)}>
            {file}
          </li>
        ))}
      </ul>
    </div>
  );
};

export default SelectedFiles;