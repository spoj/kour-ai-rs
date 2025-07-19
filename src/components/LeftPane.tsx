import { Resizable } from "re-resizable";

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
    <div
      style={{
        display: "flex",
        justifyContent: "space-between",
        alignItems: "center",
      }}
    >
      <h2>Files</h2>
    </div>
    <input
      ref={searchInputRef}
      type="text"
      placeholder="Search files..."
      value={searchTerm}
      onChange={(e) => setSearchTerm(e.target.value)}
      style={{ width: "100%", marginBottom: "10px" }}
    />
    <ul>
      {fileList.length > 0 ? (
        fileList.map((file) => <li key={file}>{file}</li>)
      ) : (
        <h2>Please start by selecting a folder</h2>
      )}
    </ul>
  </Resizable>
);