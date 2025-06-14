const { createApp, ref, nextTick, onMounted, watch } = Vue;

createApp({
  setup() {
    const chatHistory = ref([]);
    const newMessage = ref("");
    const pastedFiles = ref([]);
    const apiKey = ref("");
    const modelName = ref("anthropic/claude-3-haiku");
    const systemPrompt = ref("");
    const rootDir = ref("");
    const sofficePath = ref("");
    const showSettings = ref(false);
    const isTyping = ref(false);
    const chatContainer = ref(null);

    onMounted(async () => {
      apiKey.value = await window.electronAPI.getApiKey();
      modelName.value = await window.electronAPI.getModelName();
      systemPrompt.value = await window.electronAPI.getSystemPrompt();
      rootDir.value = await window.electronAPI.getRootDir();
      sofficePath.value = await window.electronAPI.getSofficePath();

      window.electronAPI.onDebugLog((payload) => {
        console.log(`[MAIN PROCESS] ${payload.type}:`, payload.data);
      });

      // Focus on message input
      await nextTick();
      const messageInput = document.getElementById("message-input");
      if (messageInput) {
        messageInput.focus();
      }

      window.addEventListener("paste", (event) => {
        const files = event.clipboardData.files;
        if (files.length > 0) {
          // Add newly pasted files to the existing array
          pastedFiles.value = [...pastedFiles.value, ...Array.from(files)];
          event.preventDefault(); // Prevent pasting file path as text
        }
      });
    });

    watch(apiKey, (newApiKey) => {
      window.electronAPI.setApiKey(newApiKey);
    });

    watch(modelName, (newModelName) => {
      window.electronAPI.setModelName(newModelName);
    });

    watch(systemPrompt, (newSystemPrompt) => {
      window.electronAPI.setSystemPrompt(newSystemPrompt);
    });

    watch(rootDir, (newRootDir) => {
      window.electronAPI.setRootDir(newRootDir);
    });

    watch(sofficePath, (newSofficePath) => {
      window.electronAPI.setSofficePath(newSofficePath);
    });

    const adjustTextareaHeight = (event) => {
      const textarea = event.target;

      // If empty or very short content, use default height
      if (!textarea.value || textarea.value.length < 30) {
        textarea.style.height = "40px";
        return;
      }

      // Reset to default height to get accurate scrollHeight
      textarea.style.height = "40px";

      // If content requires more height, expand
      if (textarea.scrollHeight > textarea.clientHeight) {
        textarea.style.height = textarea.scrollHeight + "px";
      }
    };

    const sendMessage = async () => {
      if (!newMessage.value.trim() && pastedFiles.value.length === 0) return;

      const userMessageContent = [];

      // 1. Add text part if available
      if (newMessage.value.trim()) {
        userMessageContent.push({
          type: "text",
          text: newMessage.value.trim(),
        });
      }

      // 2. Process and add file parts
      const filePromises = pastedFiles.value.map(
        (file) =>
          new Promise((resolve, reject) => {
            const reader = new FileReader();
            reader.onerror = (error) => reject(error);

            const textExtensions = [
              '.txt', '.md', '.csv', '.js', '.py', '.html', '.css', '.json',
              '.ts', '.jsx', '.tsx', '.yaml', '.yml', '.xml', 'Dockerfile'
            ];
            const isTextFile = textExtensions.some(ext => file.name.endsWith(ext)) || file.type.startsWith('text/');

            // Handle PDF files
            if (file.type === "application/pdf") {
              reader.onload = (e) => {
                const base64PDF = e.target.result;
                resolve({
                  type: 'file',
                  file: { filename: file.name, file_data: base64PDF },
                });
              };
              reader.readAsDataURL(file);

            // Handle images
            } else if (file.type.startsWith("image/")) {
              reader.onload = (e) => {
                resolve({
                  type: "image_url",
                  image_url: { url: e.target.result },
                });
              };
              reader.readAsDataURL(file);

            // Handle whitelisted text-based files
            } else if (isTextFile) {
              reader.onload = (e) => {
                resolve({
                  type: "text",
                  text: `Content of "${file.name}":\n\n${e.target.result}`,
                  isAttachment: true,
                });
              };
              reader.readAsText(file);
              
            // Skip unsupported files
            } else {
              console.warn(`Unsupported file type: ${file.type || 'unknown'} for file ${file.name}. Skipping.`);
              resolve(null);
            }
          })
      );

      try {
        const fileContents = await Promise.all(filePromises);
        // Filter out any null results from non-image files
        const validFileContents = fileContents.filter(content => content !== null);
        userMessageContent.push(...validFileContents);

        // 3. Add the complete message to chat history
        chatHistory.value.push({ role: "user", content: userMessageContent });
        newMessage.value = "";
        pastedFiles.value = []; // Clear files after preparing them

        // 4. Reset textarea height and scroll
        await nextTick();
        const textarea = document.getElementById("message-input");
        if (textarea) {
          textarea.style.height = "40px";
        }
        isTyping.value = true;
        scrollToBottom();

        // 5. Send to main process
        const plainMessages = JSON.parse(JSON.stringify(chatHistory.value));
        const response = await window.electronAPI.sendMessage({
          apiKey: apiKey.value,
          modelName: modelName.value,
          systemPrompt: systemPrompt.value,
          messages: plainMessages,
          rootDir: rootDir.value,
        });

        isTyping.value = false;
        if (response) {
          chatHistory.value = response;
          scrollToBottom();
        }
      } catch (error) {
        isTyping.value = false;
        console.error("Error processing files or sending message:", error);
        chatHistory.value.push({
          role: "assistant",
          content: "Sorry, there was an error processing the files.",
        });
        scrollToBottom();
      }
    };

    const renderMarkdown = (content) => {
      if (typeof content !== 'string') return '';
      return marked.parse(content);
    };

    const scrollToBottom = () => {
      nextTick(() => {
        if (chatContainer.value) {
          chatContainer.value.scrollTop = chatContainer.value.scrollHeight;
        }
      });
    };

    const restartSession = () => {
      chatHistory.value = [];
      pastedFiles.value = [];
    };

    const removeFile = (index) => {
      pastedFiles.value.splice(index, 1);
    };

    return {
      chatHistory,
      newMessage,
      apiKey,
      modelName,
      systemPrompt,
      rootDir,
      sofficePath,
      showSettings,
      isTyping,
      chatContainer,
      adjustTextareaHeight,
      sendMessage,
      restartSession,
      pastedFiles,
      removeFile,
      renderMarkdown,
    };
  },
}).mount("#app");
