import { app, BrowserWindow, ipcMain, protocol, net, shell } from "electron";
import path from "path";
import fs from "fs";
import { fileURLToPath } from "url";
import OpenAI from "openai";
import Store from "electron-store";
import { tools, toolFunctions } from "./tools/index.js";
import {
  getFileContentForLLM,
  processFileBufferForLLM,
} from "./helper/fileManager.js";
import crypto from "crypto";
import squirrelStartup from "electron-squirrel-startup";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

if (squirrelStartup) app.quit();

let mainWindow;
// Track the last known root directory to detect changes.
let lastKnownRootDir = null;

// Read default system prompt before initializing store
let defaultSystemPrompt = "";
try {
  defaultSystemPrompt = fs.readFileSync(
    path.join(__dirname, "DEFAULT_PROMPT.md"),
    "utf-8"
  );
} catch (error) {
  console.error("Failed to read default system prompt:", error);
}

const store = new Store({
  defaults: {
    settings: {
      apiKey: "",
      modelName: "google/gemini-2.5-pro-preview",
      rootDir: "",
      systemPrompt: defaultSystemPrompt,
      sofficePath: "",
      providerOrder: "",
    },
  },
});

const createWindow = () => {
  mainWindow = new BrowserWindow({
    width: 600,
    height: 800,
    webPreferences: {
      preload: path.join(__dirname, "preload.js"),
    },
  });

  mainWindow.loadFile("index.html");
};

app.whenReady().then(() => {
  // Register sandbox protocol using modern API
  protocol.handle("sandbox", (request) => {
    // Parse URL properly instead of using deprecated substr
    const parsedUrl = new URL(request.url);
    const relativePath = parsedUrl.hostname + parsedUrl.pathname;
    const sandboxDir = path.join(app.getPath("userData"), "sandbox");
    const filePath = path.join(sandboxDir, relativePath);

    // Security check: ensure the file is within sandbox directory
    const normalizedPath = path.normalize(filePath);
    if (!normalizedPath.startsWith(sandboxDir)) {
      return new Response("File not found", { status: 404 });
    }

    // Return file response
    return net.fetch(`file://${filePath}`);
  });

  createWindow();
});

ipcMain.handle("get-settings", () => {
  return store.get("settings");
});
ipcMain.handle("set-settings", (event, settings) => {
  store.set("settings", settings);
});

// IPC handler for opening external URLs
ipcMain.handle("open-external-url", async (event, url) => {
  try {
    await shell.openExternal(url);
    return { success: true };
  } catch (error) {
    console.error("Failed to open external URL:", error);
    return { success: false, error: error.message };
  }
});

// IPC handler for processing pasted attachments
ipcMain.handle(
  "process-attachment",
  async (event, { fileBuffer, fileName }) => {
    const settings = store.get("settings");
    const toolContext = {
      // rootDir is not strictly needed for buffer processing, but good to have
      rootDir: settings.rootDir,
      sofficePath: settings.sofficePath,
      appDataDir: app.getPath("appData"),
    };
    try {
      // Use the new buffer-based processing function
      return await processFileBufferForLLM(
        Buffer.from(fileBuffer),
        fileName,
        toolContext
      );
    } catch (error) {
      // Propagate the error back to the renderer
      throw new Error(
        error.message || "An unknown error occurred during file processing."
      );
    }
  }
);

ipcMain.handle("send-message", async (event, { messages }) => {
  const settings = store.get("settings");
  const { apiKey, modelName, systemPrompt, rootDir, providerOrder } = settings;
  const logToRenderer = (payload) =>
    mainWindow.webContents.send("debug-log", payload);
  const updateHistory = (newHistory) =>
    mainWindow.webContents.send("update-history", newHistory);

  const openai = new OpenAI({
    baseURL: "https://openrouter.ai/api/v1",
    apiKey: apiKey,
  });

  let history = [...messages];
  const currentRootDir = rootDir;

  // Check if the root directory has changed since the last message.
  if (currentRootDir !== lastKnownRootDir) {
    history.push({
      role: "user",
      content: `\nNote: root directory has been changed to: "${currentRootDir}".`,
      is_system_notification: true,
    });
    lastKnownRootDir = currentRootDir; // Update the last known state.
    updateHistory(history); // Update the UI with the notification.
  }

  try {
    // Main conversation loop
    while (true) {
      // Prepare messages with system prompt
      const requestMessages = [...history];
      if (
        systemPrompt &&
        (requestMessages.length === 0 || requestMessages[0].role !== "system")
      ) {
        requestMessages.unshift({ role: "system", content: systemPrompt });
      }

      // Make API request
      logToRenderer({
        type: "API_REQUEST",
        data: { modelName, messages: requestMessages, tools },
      });
      const providerOrderArr = providerOrder.split(",").map((p) => p.trim());
      const response = await openai.chat.completions.create({
        model: modelName,
        messages: requestMessages,
        provider: {
          order: providerOrderArr,
        },
        tools: tools,
      });
      logToRenderer({ type: "API_SUCCESS", data: response });

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
        const toolResult = await executeToolCall(
          toolCall,
          settings,
          logToRenderer
        );

        if (toolResult && toolResult.is_file_viewer) {
          // This is a result from load_file, handle it specially
          // Add the content message to history.
          history.push({
            role: toolResult.role,
            content: toolResult.content,
            is_file_viewer: true
          });

          // Also add a simplified success message for the tool call itself
          // to let the model know the tool executed correctly.
          history.push({
            tool_call_id: toolCall.id,
            role: "tool",
            name: toolCall.function.name,
            content: JSON.stringify({
              success: true,
              message: `File loaded into context.`,
            }),
          });
        } else {
          // This is a regular tool result
          history.push(toolResult);
        }

        // Update UI after each tool result
        updateHistory(history);
      }
    }

    return history;
  } catch (error) {
    logToRenderer({ type: "API_ERROR", data: error });
    throw new Error(
      `API Error: ${error.message || "Could not get a response from the model."
      }`
    );
  }
});

// Helper function to execute a single tool call
async function executeToolCall(toolCall, settings, logToRenderer) {
  const functionName = toolCall.function.name;

  const sandboxDir = path.join(app.getPath("userData"), "sandbox");
  if (!fs.existsSync(sandboxDir)) {
    fs.mkdirSync(sandboxDir, { recursive: true });
  }

  const toolContext = {
    ...settings,
    sandboxDir,
    providerOrder: settings.providerOrder.split(",").map((p) => p.trim()),
    appDataDir: app.getPath("appData"),
  };

  if (!toolFunctions[functionName]) {
    return {
      tool_call_id: toolCall.id,
      role: "tool",
      name: functionName,
      content: JSON.stringify({ error: `Unknown tool: ${functionName}` }),
    };
  }

  try {
    let functionArgs = {};
    if (toolCall.function.arguments) {
      try {
        functionArgs = JSON.parse(toolCall.function.arguments);
      } catch (parseError) {
        logToRenderer({
          type: "TOOL_PARSE_WARNING",
          data: {
            functionName,
            arguments: toolCall.function.arguments,
            warning: "Failed to parse arguments, using empty object",
          },
        });
        functionArgs = {};
      }
    }

    const result = await toolFunctions[functionName](functionArgs, toolContext);

    // This is the special handler for the raw file content from load_file
    if (result && result.is_file_viewer) {
      if (result.error) {
        // If the tool returned an error object, format it for the model
        return {
          tool_call_id: toolCall.id,
          role: "tool",
          name: functionName,
          content: JSON.stringify({ error: result.error }),
        };
      }
      return result; // Pass the special file message object through
    }

    // For all other tools, stringify the result
    const content =
      typeof result === "object" ? JSON.stringify(result) : result.toString();

    return {
      tool_call_id: toolCall.id,
      role: "tool",
      name: functionName,
      content: content,
    };
  } catch (error) {
    const errorInfo = {
      functionName,
      args: toolCall.function.arguments,
      message: error.message,
      stack: error.stack,
    };

    logToRenderer({ type: "TOOL_ERROR", data: errorInfo });

    let userErrorMessage = `Tool execution failed: ${error.message}`;
    if (error.name === "SecurityError") {
      userErrorMessage = `Security Error: ${error.message}`;
    } else if (error.name === "NotFoundError") {
      userErrorMessage = `File Not Found: ${error.message}`;
    } else if (error.name === "ConfigurationError") {
      userErrorMessage = `Configuration Error: ${error.message}. Please check your settings.`;
    }

    return {
      tool_call_id: toolCall.id,
      role: "tool",
      name: functionName,
      content: JSON.stringify({ error: userErrorMessage }),
    };
  }
}

app.on("activate", () => {
  if (BrowserWindow.getAllWindows().length === 0) {
    createWindow();
  }
});

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") {
    app.quit();
  }
});
