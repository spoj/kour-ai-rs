import { useState, useEffect, useRef } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import "./App.css";
import {
  chat,
  getSettings,
  saveSettings,
  replayHistory,
  clearHistory,
  onChatCompletionUpdate,
  cancelOutstandingRequest,
  delete_message,
  delete_tool_interaction,
  search_files_by_name,
} from "./commands";
import { fileToAttachment } from "./helpers";
import { IChatCompletionMessage, ISettings, MessageContent } from "./types";
import { SettingsModal } from "./components/SettingsModal";
import { getVersion } from "@tauri-apps/api/app";
import { Bounce, ToastContainer } from "react-toastify";
import { TopBar } from "./components/TopBar";
import { FilePicker } from "./components/FilePicker";
import { ChatStream } from "./components/ChatStream";
import { useAutoScroll } from "./hooks/useAutoScroll";
import { useFileSelection } from "./hooks/useFileSelection";
import { useDebounce } from "./hooks/useDebounce";

type Attachment = {
  type: string;
  content: string;
  filename: string;
};

function App() {
  const [messages, setMessages] = useState<IChatCompletionMessage[]>([]);
  const [input, setInput] = useState("");
  const [attachments, setAttachments] = useState<Attachment[]>([]);
  const [appVersion, setAppVersion] = useState("");
  const [isFlapOpen, setIsFlapOpen] = useState(false);
  const rootDirInputRef = useRef<HTMLInputElement>(null);
  const messageInputRef = useRef<HTMLTextAreaElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const [openSettingsModal, setOpenSettingsModal] = useState(false);
  const [isTyping, setIsTyping] = useState(false);
  const [fileList, setFileList] = useState<string[]>([]);
  const [searchTerm, setSearchTerm] = useState("");
  const debouncedSearchTerm = useDebounce(searchTerm, 300);
  const chatContainerRef = useAutoScroll(messages);
  const {
    selectedFiles,
    handleFileSelect,
    handleAddAll,
    handleSubtractAll,
    handleAsShown,
    handleClearSelection,
    setSelectionRange,
    setSelectedFiles,
  } = useFileSelection(fileList);
  const [settings, setSettings] = useState<ISettings>({
    apiKey: "",
    modelName: "",
    rootDir: "",
    sofficePath: "",
    providerOrder: "",
  });

  const handleClearAll = () => {
    clearHistory().then(() => setMessages([]));
    setSearchTerm("");
    setSelectedFiles([]);
    setIsFlapOpen(false);
  };

  const handleChatUpdate = (update: any) => {
    switch (update.type) {
      case "Start":
        setIsTyping(true);
        break;
      case "End":
        setIsTyping(false);
        break;
      case "Message":
        setMessages((prev) => {
          if (prev.some((m) => m.id === update.id)) return prev;
          return [
            ...prev,
            {
              id: update.id,
              role: update.role as "user" | "assistant",
              content: update.content,
            },
          ];
        });
        break;
      case "ToolCall":
        setMessages((prev) => {
          if (
            prev.some(
              (m) =>
                m.id === update.id && m.tool_call_id === update.tool_call_id
            )
          )
            return prev;
          return [
            ...prev,
            {
              id: update.id,
              tool_call_id: update.tool_call_id,
              role: "assistant",
              content: [],
              isNotification: true,
              toolName: update.tool_name,
              toolArgs: update.tool_args,
            },
          ];
        });
        break;
      case "ToolDone":
        setMessages((prev) => {
          const newMessages = [...prev];
          const foundIndex = newMessages.findIndex(
            (m) => m.tool_call_id === update.tool_call_id && !m.toolResult
          );
          if (foundIndex !== -1) {
            const updatedMessage = {
              ...newMessages[foundIndex],
              toolResult: update.tool_result,
            };
            newMessages[foundIndex] = updatedMessage;
          }
          return newMessages;
        });
        break;
    }
  };

  useEffect(() => {
    getVersion().then(setAppVersion);
    getSettings().then((s) => {
      setSettings(s);
    });
    messageInputRef.current?.focus();
    const unlisten = onChatCompletionUpdate(handleChatUpdate);
    setMessages([]);
    replayHistory();
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.altKey) {
        if (isFlapOpen) {
          switch (e.key) {
            case "a":
              e.preventDefault();
              handleAddAll();
              break;
            case "s":
              e.preventDefault();
              handleSubtractAll();
              break;
            case "d":
              e.preventDefault();
              handleAsShown();
              break;
            case "c":
              e.preventDefault();
              handleClearSelection();
              break;
          }
        }
      }
      if (e.ctrlKey) {
        switch (e.key) {
          case "k":
            e.preventDefault();
            handleClearAll();
            break;
          case "o":
            e.preventDefault();
            handleSelectFolder();
            break;
          case "r":
            e.preventDefault();
            messageInputRef.current?.select();
            setIsFlapOpen(false);
            break;
          case "f":
            e.preventDefault();
            setIsFlapOpen((prev) => {
              const nextState = !prev;
              if (nextState) {
                setTimeout(() => searchInputRef.current?.select(), 0);
              }
              return nextState;
            });
            break;
        }
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [isFlapOpen, fileList, handleAddAll, handleSubtractAll, handleAsShown, handleClearSelection, handleClearAll]);

  useEffect(() => {
    if (settings.rootDir) {
      search_files_by_name(debouncedSearchTerm)
        .then(setFileList)
        .catch((_) => setFileList([]));
    }
  }, [settings.rootDir, debouncedSearchTerm]);

  useEffect(() => {
    if (messageInputRef.current) {
      messageInputRef.current.style.height = "auto";
      messageInputRef.current.style.height = `${messageInputRef.current.scrollHeight}px`;
    }
  }, [input]);

  const handleSettingsChange = (newSettings: Partial<ISettings>) => {
    const updatedSettings = { ...settings, ...newSettings };
    setSettings(updatedSettings);
    saveSettings(updatedSettings);
  };

  const handleSend = async () => {
    if (isTyping) return;
    const messageContent: MessageContent = [];
    if (input.trim() !== "") {
      messageContent.push({ type: "text", text: input });
    }
    attachments.forEach((a) => {
      if (a.type.startsWith("image/")) {
        messageContent.push({
          type: "image_url",
          image_url: { url: a.content },
        });
      } else {
        messageContent.push({
          type: "file",
          file: { filename: a.filename, file_data: a.content },
        });
      }
    });
    setInput("");
    setAttachments([]);
    chat(messageContent);
  };

  const handleKeyDown = (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      handleSend();
    }
  };

  const saveAttachment = (item: Attachment) => {
    setAttachments((prev) => [...prev, item]);
  };

  const handlePaste = (event: React.ClipboardEvent<HTMLTextAreaElement>) => {
    for (const item of event.clipboardData.items) {
      if (item.kind === "file") {
        event.preventDefault();
      }
      fileToAttachment(item, saveAttachment);
    }
  };

  const handleCopy = (content: IChatCompletionMessage["content"]) => {
    const textToCopy = content
      .filter((item) => item.type === "text")
      .map((item: any) => item.text)
      .join("\n");
    navigator.clipboard.writeText(textToCopy);
  };

  const handleDelete = (id: number) => {
    const message_to_delete = messages.find((m) => m.id === id);
    if (!message_to_delete) return;
    const tool_call_ids_to_delete = new Set(
      message_to_delete.tool_calls?.map((tc) => tc.id)
    );
    setMessages((prev) =>
      prev.filter(
        (m) =>
          m.id !== id &&
          !(m.tool_call_id && tool_call_ids_to_delete.has(m.tool_call_id))
      )
    );
    delete_message(id);
  };

  const handleDeleteTool = (llm_interaction_id: number, tool_call_id: string) => {
    setMessages((prev) =>
      prev.filter(
        (m) =>
          !(m.id === llm_interaction_id && m.tool_call_id === tool_call_id)
      )
    );
    delete_tool_interaction(llm_interaction_id, tool_call_id);
  };

  const handleCancel = () => {
    cancelOutstandingRequest().then(() => {
      setMessages([]);
      replayHistory();
    });
  };

  const handleSelectFolder = async () => {
    const result = await open({ directory: true, multiple: false });
    if (typeof result === "string") {
      handleSettingsChange({ rootDir: result });
    }
  };

  return (
    <div className="container">
      <TopBar
        appVersion={appVersion}
        settings={settings}
        handleSettingsChange={handleSettingsChange}
        onClearHistory={handleClearAll}
        onOpenSettings={() => setOpenSettingsModal(true)}
        onSelectFolder={handleSelectFolder}
        rootDirInputRef={rootDirInputRef}
        onToggleFlap={() => setIsFlapOpen((prev) => !prev)}
      />
      <main id="main-content">
        {isFlapOpen && (
          <div className="flap">
            <FilePicker
              searchTerm={searchTerm}
              setSearchTerm={setSearchTerm}
              fileList={fileList}
              searchInputRef={searchInputRef}
              selectedFiles={selectedFiles}
              onFileSelect={handleFileSelect}
              onAddAll={handleAddAll}
              onSubtractAll={handleSubtractAll}
              onAsShown={handleAsShown}
              onClearSelection={handleClearSelection}
              setSelectionRange={setSelectionRange}
            />
          </div>
        )}
        {isFlapOpen && <div className="backdrop" onClick={() => setIsFlapOpen(false)} />}
        <ChatStream
          messages={messages}
          isTyping={isTyping}
          onCopy={handleCopy}
          onDelete={handleDelete}
          onDeleteTool={handleDeleteTool}
          chatContainerRef={chatContainerRef}
          attachments={attachments}
          setAttachments={setAttachments}
          input={input}
          setInput={setInput}
          handleKeyDown={handleKeyDown}
          handlePaste={handlePaste}
          handleSend={handleSend}
          handleCancel={handleCancel}
          messageInputRef={messageInputRef}
          selectedFiles={selectedFiles}
          onToggleFlap={() => setIsFlapOpen(true)}
        />
      </main>
      {openSettingsModal && (
        <SettingsModal
          settings={settings}
          onClose={() => setOpenSettingsModal(false)}
          onSave={handleSettingsChange}
        />
      )}
      <ToastContainer
        position="top-right"
        autoClose={5000}
        hideProgressBar={false}
        newestOnTop={false}
        closeOnClick
        rtl={false}
        pauseOnFocusLoss={false}
        draggable
        pauseOnHover={false}
        theme="light"
        transition={Bounce}
      />
    </div>
  );
}

export default App;
