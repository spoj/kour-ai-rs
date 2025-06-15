import path from "path";
import { safelyReadFile, safelyWriteFile } from "../fileManager.js";

async function getNotesPath(toolContext) {
  const { rootDir } = toolContext;
  if (!rootDir) {
    throw new Error(
      "Root directory is not specified. Please specify a root directory."
    );
  }
  return path.join(rootDir, "_NOTES.md");
}

export async function read_notes(args, toolContext) {
  try {
    const notesPath = await getNotesPath(toolContext);
    const content = await safelyReadFile(notesPath, toolContext);
    return content.toString('utf-8') || "No notes found.";
  } catch (error) {
    if (error.name === 'NotFoundError') {
      return "No notes found.";
    }
    return `Error reading notes: ${error.message}`;
  }
}

export async function append_notes(args, toolContext) {
  const { markdown_content } = args;
  if (!markdown_content) {
    return "Error: markdown_content is required.";
  }

  try {
    const notesPath = await getNotesPath(toolContext);
    let existingContent = '';
    try {
      const buffer = await safelyReadFile(notesPath, toolContext);
      existingContent = buffer.toString('utf-8');
    } catch (error) {
      if (error.name !== 'NotFoundError') {
        throw error; // Re-throw unexpected errors
      }
      // If file doesn't exist, existingContent remains empty
    }

    const timestamp = new Date().toISOString();
    const noteEntry = `# Note entry on [${timestamp}]\n${markdown_content}\n\n`;
    const newContent = existingContent + noteEntry;

    await safelyWriteFile(notesPath, newContent, toolContext);
    return "Note appended successfully.";
  } catch (e) {
    return `Error appending note: ${e.message}`;
  }
}

export const read_notes_tool = {
  type: "function",
  function: {
    name: "read_notes",
    description: "Reads all notes from the _NOTES.md file.",
    parameters: {
      type: "object",
      properties: {},
      required: [],
    },
  },
};

export const append_notes_tool = {
  type: "function",
  function: {
    name: "append_notes",
    description: "Appends a markdown string to the _NOTES.md file.",
    parameters: {
      type: "object",
      properties: {
        markdown_content: {
          type: "string",
          description: "The markdown content to append to the notes.",
        },
      },
      required: ["markdown_content"],
    },
  },
};