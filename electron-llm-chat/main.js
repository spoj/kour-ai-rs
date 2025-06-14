import { app, BrowserWindow, ipcMain } from 'electron';
import path from 'path';
import { fileURLToPath } from 'url';
import OpenAI from 'openai';
import Store from 'electron-store';
import { tools, toolFunctions } from './tools/index.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const store = new Store();

let mainWindow;

const createWindow = () => {
  mainWindow = new BrowserWindow({
    width: 600,
    height: 800,
    webPreferences: {
      preload: path.join(__dirname, 'preload.js')
    }
  });

  mainWindow.loadFile('index.html');
};

app.whenReady().then(createWindow);

ipcMain.handle('get-api-key', () => store.get('apiKey'));
ipcMain.handle('set-api-key', (event, apiKey) => store.set('apiKey', apiKey));
ipcMain.handle('get-model-name', () => store.get('modelName', 'anthropic/claude-3-haiku'));
ipcMain.handle('set-model-name', (event, modelName) => store.set('modelName', modelName));
ipcMain.handle('get-root-dir', () => store.get('rootDir'));
ipcMain.handle('set-root-dir', (event, rootDir) => store.set('rootDir', rootDir));
ipcMain.handle('get-system-prompt', () => store.get('systemPrompt', ''));
ipcMain.handle('set-system-prompt', (event, systemPrompt) => store.set('systemPrompt', systemPrompt));
ipcMain.handle('get-soffice-path', () => store.get('sofficePath', ''));
ipcMain.handle('set-soffice-path', (event, sofficePath) => store.set('sofficePath', sofficePath));
ipcMain.handle('get-provider-order', () => store.get('providerOrder', 'google-vertex,anthropic,openai,amazon-bedrock'));
ipcMain.handle('set-provider-order', (event, providerOrder) => store.set('providerOrder', providerOrder));


ipcMain.handle('send-message', async (event, { apiKey, modelName, systemPrompt, messages, rootDir }) => {
  const logToRenderer = (payload) => mainWindow.webContents.send('debug-log', payload);
  const updateHistory = (newHistory) => mainWindow.webContents.send('update-history', newHistory);

  const openai = new OpenAI({
    baseURL: 'https://openrouter.ai/api/v1',
    apiKey: apiKey,
  });

  let history = [...messages];

  try {
    // Main conversation loop
    while (true) {
      // Prepare messages with system prompt
      const requestMessages = [...history];
      if (systemPrompt && (requestMessages.length === 0 || requestMessages[0].role !== 'system')) {
        requestMessages.unshift({ role: 'system', content: systemPrompt });
      }

      // Make API request
      logToRenderer({ type: 'API_REQUEST', data: { modelName, messages: requestMessages, tools } });
       const providerOrder = store.get('providerOrder', '').split(',').map(p => p.trim());
      const response = await openai.chat.completions.create({
        model: modelName,
        messages: requestMessages,
        provider: {
          order: providerOrder,
        },
        tools: tools,
      });
      logToRenderer({ type: 'API_SUCCESS', data: response });

      // Add assistant message to history
      const assistantMessage = response.choices[0].message;
      history.push(assistantMessage);
      
      // If no tool calls, we're done
      if (!assistantMessage.tool_calls) {
        updateHistory(history);
        break;
      }

      // Show assistant message with tool calls immediately
      updateHistory(history);

      // Process tool calls
      for (const toolCall of assistantMessage.tool_calls) {
        const toolResult = await executeToolCall(toolCall, rootDir, logToRenderer);
        history.push(toolResult);
        
        // Update UI after each tool result
        updateHistory(history);
      }
    }
    
    return history;
  } catch (error) {
    logToRenderer({ type: 'API_ERROR', data: error });
    throw new Error(`API Error: ${error.message || 'Could not get a response from the model.'}`);
  }
});

// Helper function to execute a single tool call
async function executeToolCall(toolCall, rootDir, logToRenderer) {
  const functionName = toolCall.function.name;
  
  if (!toolFunctions[functionName]) {
    return {
      tool_call_id: toolCall.id,
      role: 'tool',
      name: functionName,
      content: JSON.stringify({ error: `Unknown tool: ${functionName}` }),
    };
  }

  try {
    // Handle cases where arguments might be undefined, null, or empty string
    let functionArgs = {};
    if (toolCall.function.arguments) {
      try {
        functionArgs = JSON.parse(toolCall.function.arguments);
      } catch (parseError) {
        // If parsing fails, default to empty object
        logToRenderer({
          type: 'TOOL_PARSE_WARNING',
          data: {
            functionName,
            arguments: toolCall.function.arguments,
            warning: 'Failed to parse arguments, using empty object'
          }
        });
        functionArgs = {};
      }
    }
    
    const result = await toolFunctions[functionName](functionArgs, rootDir);
    const content = typeof result === 'object' ? JSON.stringify(result) : result.toString();
    
    return {
      tool_call_id: toolCall.id,
      role: 'tool',
      name: functionName,
      content: content,
    };
  } catch (error) {
    logToRenderer({
      type: 'TOOL_ERROR',
      data: {
        functionName,
        arguments: toolCall.function.arguments,
        error: error.message
      }
    });
    
    return {
      tool_call_id: toolCall.id,
      role: 'tool',
      name: functionName,
      content: JSON.stringify({ error: `Tool execution failed: ${error.message}` }),
    };
  }
}

app.on('activate', () => {
  if (BrowserWindow.getAllWindows().length === 0) {
    createWindow();
  }
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});