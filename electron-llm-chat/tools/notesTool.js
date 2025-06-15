import fs from "fs/promises";
import path from "path";

async function getNotesPath(rootDir) {
  if (!rootDir) {
    throw new Error(
      "Root directory is not specified. Please specify a root directory."
    );
  }
  return path.join(path.resolve(rootDir), "_NOTES.md");
}

export async function read_notes(args, toolContext) {
  const { rootDir } = toolContext;
  try {
    const notesPath = await getNotesPath(rootDir);
    try {
      const content = await fs.readFile(notesPath, "utf-8");
      return content || "No notes found.";
    } catch (error) {
      if (error.code === "ENOENT") {
        return "No notes found.";
      }
      throw error;
    }
  } catch (e) {
    return `Error reading notes: ${e.message}`;
  }
}

export async function append_notes(args, toolContext) {
  const { rootDir } = toolContext;
  const { markdown_content } = args;
  if (!markdown_content) {
    return "Error: markdown_content is required.";
  }

  try {
    const notesPath = await getNotesPath(rootDir);
    const timestamp = new Date().toISOString();
    const noteEntry = `# Note entry on [${timestamp}]\n${markdown_content}\n\n`;

    await fs.appendFile(notesPath, noteEntry);
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