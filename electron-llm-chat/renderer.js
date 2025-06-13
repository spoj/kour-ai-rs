const { createApp, ref, nextTick, onMounted, watch } = Vue;

createApp({
  setup() {
    const chatHistory = ref([]);
    const newMessage = ref('');
    const apiKey = ref('');
    const modelName = ref('anthropic/claude-3-haiku');
    const showSettings = ref(false);
    const isTyping = ref(false);
    const chatContainer = ref(null);

    onMounted(async () => {
      apiKey.value = await window.electronAPI.getApiKey();
      modelName.value = await window.electronAPI.getModelName();
      
      window.electronAPI.onDebugLog((payload) => {
        console.log(`[MAIN PROCESS] ${payload.type}:`, payload.data);
      });
      
      // Focus on message input
      await nextTick();
      const messageInput = document.getElementById('message-input');
      if (messageInput) {
        messageInput.focus();
      }
    });

    watch(apiKey, (newApiKey) => {
        window.electronAPI.setApiKey(newApiKey);
    });

    watch(modelName, (newModelName) => {
        window.electronAPI.setModelName(newModelName);
    });

    const adjustTextareaHeight = (event) => {
      const textarea = event.target;
      
      // If empty or very short content, use default height
      if (!textarea.value || textarea.value.length < 30) {
        textarea.style.height = '38px';
        return;
      }
      
      // Reset to default height to get accurate scrollHeight
      textarea.style.height = '38px';
      
      // If content requires more height, expand
      if (textarea.scrollHeight > textarea.clientHeight) {
        textarea.style.height = textarea.scrollHeight + 'px';
      }
    };

    const sendMessage = async () => {
      if (!newMessage.value.trim()) return;

      chatHistory.value.push({ role: 'user', content: newMessage.value });
      const currentMessage = newMessage.value;
      newMessage.value = '';
      
      // Reset textarea height after sending
      await nextTick();
      const textarea = document.getElementById('message-input');
      if (textarea) {
        textarea.style.height = '38px';
      }

      isTyping.value = true;
      scrollToBottom();

      try {
        
        // Convert Vue reactive objects to plain objects for IPC
        const plainMessages = JSON.parse(JSON.stringify(chatHistory.value));
        
        const response = await window.electronAPI.sendMessage({
          apiKey: apiKey.value,
          modelName: modelName.value,
          messages: plainMessages,
        });

        isTyping.value = false;
        if (response) {
            chatHistory.value = response;
            scrollToBottom();
        }
      } catch (error) {
        isTyping.value = false;
        console.error('Error sending message:', error);
        console.error('Error details:', error.message, error.stack);
        
        // Optionally add error message to chat
        chatHistory.value.push({
          role: 'assistant',
          content: `Sorry, there was an error processing your message: ${error.message || 'Unknown error'}. Please check your API key and try again.`
        });
        scrollToBottom();
      }
    };

    const scrollToBottom = () => {
      nextTick(() => {
        if(chatContainer.value) {
            chatContainer.value.scrollTop = chatContainer.value.scrollHeight;
        }
      });
    };

    return {
      chatHistory,
      newMessage,
      apiKey,
      modelName,
      showSettings,
      isTyping,
      chatContainer,
      adjustTextareaHeight,
      sendMessage,
    };
  },
}).mount('#app');