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


ipcMain.handle('send-message', async (event, { apiKey, modelName, messages, rootDir }) => {
  const logToRenderer = (payload) => mainWindow.webContents.send('debug-log', payload);

  logToRenderer({ type: 'API_REQUEST', data: { modelName, messages, tools } });

  const openai = new OpenAI({
    baseURL: 'https://openrouter.ai/api/v1',
    apiKey: apiKey,
  });

  try {
    let response = await openai.chat.completions.create({
      model: modelName,
      messages: messages,
      tools: tools,
    });
    logToRenderer({ type: 'API_SUCCESS', data: response });

    let message = response.choices[0].message;
    messages.push(message);

    while (message.tool_calls) {
      const toolCalls = message.tool_calls;
      for (const toolCall of toolCalls) {
        const functionName = toolCall.function.name;
        if (toolFunctions[functionName]) {
          const functionArgs = JSON.parse(toolCall.function.arguments);
          const result = await toolFunctions[functionName](functionArgs, rootDir);
          const content = typeof result === 'object' ? JSON.stringify(result) : result.toString();
          messages.push({
            tool_call_id: toolCall.id,
            role: 'tool',
            name: functionName,
            content: content,
          });
        }
      }
      logToRenderer({ type: 'API_REQUEST', data: { modelName, messages, tools } });
      response = await openai.chat.completions.create({
        model: modelName,
        messages: messages,
        tools: tools,
      });
      logToRenderer({ type: 'API_SUCCESS', data: response });
      message = response.choices[0].message;
      messages.push(message);
    }
    return messages;
  } catch (error) {
    logToRenderer({ type: 'API_ERROR', data: error });
    throw new Error(`API Error: ${error.message || 'Could not get a response from the model.'}`);
  }
});

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