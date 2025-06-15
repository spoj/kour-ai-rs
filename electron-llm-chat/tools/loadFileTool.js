import path from "path";
import { getFileContentForLLM } from "../fileManager.js";

export const load_file_tool = {
  type: "function",
  function: {
    name: "load_file",
    description:
      "Loads a file directly into the conversation context. Supports the same file types as map_query.",
    parameters: {
      type: "object",
      properties: {
        filename: {
          type: "string",
          description: "The path to the file to load.",
        },
      },
      required: ["filename"],
    },
  },
};

function createFileMessage(fileData) {
    const { type, mime, content, filename, originalExtension, isSpreadsheet } = fileData;
    const messageContent = [];

    const fileText = `File: ${filename}` + (originalExtension ? ` (converted from ${originalExtension.toUpperCase()})` : '');

    // Add content based on file type
    if (type === 'image') {
        messageContent.push({ type: "image_url", image_url: { url: `data:${mime};base64,${content}` } });
    } else if (type === 'pdf') {
        // Assuming your model/API supports a 'file' type for PDFs
        messageContent.push({ type: "file", file: { filename: filename, file_data: `data:${mime};base64,${content}` } });
    } else if (type === 'text') {
        const prefix = isSpreadsheet ? 'File Content (from spreadsheet):\n' : 'File Content:\n';
        messageContent.push({ type: "text", text: `${prefix}${content}` });
    }

    // Add the file name text part
    messageContent.push({ type: "text", text: fileText });

    return {
        role: "assistant", // Changed from "user" to "assistant"
        content: messageContent,
        is_file_viewer: true, // Add a flag for special UI rendering
    };
}


export async function load_file(args, toolContext) {
  const { filename } = args;
  const { rootDir } = toolContext;
  const filePath = path.join(rootDir, filename);

  try {
    const fileData = await getFileContentForLLM(filePath, toolContext);
    const fileMessage = createFileMessage(fileData);

    // This custom flag will be used in main.js to identify this special tool result
    fileMessage.is_file_content = true;

    // The function will return the file content in a format that the main model can understand.
    // The main thread will then inject this into the conversation history.
    return fileMessage;
  } catch (error) {
    // Return a structured error for the main process to handle
    return {
      is_file_content: false, // Ensure this flag is false on error
      error: `Error loading file: ${error.message}`,
    };
  }
}