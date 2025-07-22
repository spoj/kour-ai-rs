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
  selection_add,
  selection_remove,
  selection_clear,
} from "./commands";
import { fileToAttachment } from "./helpers";
import { IChatCompletionMessage, ISettings, MessageContent } from "./types";
import { SettingsModal } from "./components/SettingsModal";
import { getVersion } from "@tauri-apps/api/app";
import { Bounce, ToastContainer } from "react-toastify";
import { TopBar } from "./components/TopBar";
import { FilePicker } from "./components/FilePicker";
import { ChatStream } from "./components/ChatStream";


type Attachment = {
  type: string; // Mime type e.g. "image/png"
  content: string; // data URL of the content
  filename: string;
};

function App() {
  const [messages, setMessages] = useState<IChatCompletionMessage[]>([]);
  const [input, setInput] = useState("");
  const [attachments, setAttachments] = useState<Attachment[]>([]);
  const [appVersion, setAppVersion] = useState("");
  const [selectedFiles, setSelectedFiles] = useState<string[]>([]);
  const [isFlapOpen, setIsFlapOpen] = useState(false);
  const chatContainerRef = useRef<HTMLDivElement>(null);
  const rootDirInputRef = useRef<HTMLInputElement>(null);
  const messageInputRef = useRef<HTMLTextAreaElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const [openSettingsModal, setOpenSettingsModal] = useState(false);
  const [isTyping, setIsTyping] = useState(false);
  const [fileList, setFileList] = useState<string[]>([]);
  const [searchTerm, setSearchTerm] = useState("");
  const [debouncedSearchTerm, setDebouncedSearchTerm] = useState(searchTerm);
  const [settings, setSettings] = useState<ISettings>({
    apiKey: "",
    modelName: "",
    rootDir: "",
    sofficePath: "",
    providerOrder: "",
  });

  useEffect(() => {
    getVersion().then(setAppVersion);
    getSettings().then((s) => {
      setSettings(s);
    });
    messageInputRef.current?.focus();

    const unlisten = onChatCompletionUpdate((update) => {
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
            let foundIndex = -1;
            for (let i = newMessages.length - 1; i >= 0; i--) {
              const m = newMessages[i];
              if (m.tool_call_id === update.tool_call_id && !m.toolResult) {
                foundIndex = i;
                break;
              }
            }

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
    });

    // Clear messages before replaying history to prevent duplication
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
              handleSelectAll();
              break;
            case "c":
              e.preventDefault();
              handleClearSelection();
              break;
          }
        }
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [isFlapOpen, fileList]);


  useEffect(() => {
    const handleGlobalKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey) {
        switch (e.key) {
          case "k":
            e.preventDefault();
            handleClearAll();
            break;
          case "r":
            e.preventDefault();
            messageInputRef.current?.select();
            setIsFlapOpen(false);
            break;
          case "b":
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

    window.addEventListener("keydown", handleGlobalKeyDown);
    return () => {
      window.removeEventListener("keydown", handleGlobalKeyDown);
    };
  }, []);

  useEffect(() => {
    const handler = setTimeout(() => {
      setDebouncedSearchTerm(searchTerm);
    }, 300);

    return () => {
      clearTimeout(handler);
    };
  }, [searchTerm]);

  useEffect(() => {
    if (settings.rootDir) {
      search_files_by_name(debouncedSearchTerm).then(setFileList).catch((_) => setFileList([]));
    }
  }, [settings.rootDir, debouncedSearchTerm]);

  const prevMessagesLength = useRef(messages.length);
  useEffect(() => {
    if (messages.length > prevMessagesLength.current) {
      setTimeout(() => {
        if (chatContainerRef.current) {
          chatContainerRef.current.scrollTop =
            chatContainerRef.current.scrollHeight;
        }
      }, 100);
    }
    prevMessagesLength.current = messages.length;
  }, [messages]);

  useEffect(() => {
    if (messageInputRef.current) {
      messageInputRef.current.style.height = "auto";
      messageInputRef.current.style.height = `${messageInputRef.current.scrollHeight}px`;
    }
  }, [input]);

  const prevSelectedFiles = useRef<string[]>([]);
  useEffect(() => {
    const prev = prevSelectedFiles.current;
    const next = selectedFiles;

    if (next.length === 0 && prev.length > 0) {
      selection_clear();
    } else {
      const addedFiles = next.filter((f) => !prev.includes(f));
      const removedFiles = prev.filter((f) => !next.includes(f));
      addedFiles.forEach((file) => selection_add(file));
      removedFiles.forEach((file) => selection_remove(file));
    }

    prevSelectedFiles.current = next;
  }, [selectedFiles]);

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
          file: {
            filename: a.filename,
            file_data: a.content,
          },
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

  const saveAttachment = (item: {
    type: string;
    content: string;
    filename: string;
  }) => {
    setAttachments((prev) => [...prev, item]);
  };

  const handlePaste = (event: React.ClipboardEvent<HTMLTextAreaElement>) => {
    const items = event.clipboardData.items;
    for (const item of items) {
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

  const handleFileSelect = (file: string) => {
    setSelectedFiles((prev) =>
      prev.includes(file) ? prev.filter((f) => f !== file) : [...prev, file]
    );
  };

  const handleSelectAll = () => {
    setSelectedFiles(fileList);
  };

  const handleClearSelection = () => {
    setSelectedFiles([]);
  };

  const handleClearAll = () => {
    clearHistory().then(() => setMessages([]));
    setSearchTerm("");
    setSelectedFiles([]);
    setIsFlapOpen(false);
  };

  const setSelectionRange = (files: string[], mode: "add" | "remove") => {
    setSelectedFiles((prev) => {
      if (mode === "add") {
        return [...new Set([...prev, ...files])];
      } else {
        return prev.filter((f) => !files.includes(f));
      }
    });
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
              onSelectAll={handleSelectAll}
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
