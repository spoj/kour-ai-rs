import fs from 'fs';
import path from 'path';
import AdmZip from "adm-zip";
import { extractEmail } from './email-extractor.js';
import OpenAI from 'openai';
import Store from 'electron-store';
import { fileTypeFromBuffer } from 'file-type';

const ls_tool = {
  type: "function",
  function: {
    name: "ls",
    description: "List files and directories in the specified path relative to root directory.",
    parameters: {
      type: "object",
      properties: {
        path: {
          type: "string",
          description: "The path to list files and directories in. Defaults to the current directory.",
        },
      },
      required: [],
    },
  },
};

const find_tool = {
  type: "function",
  function: {
    name: "find",
    description: "Find files matching regex pattern.",
    parameters: {
      type: "object",
      properties: {
        filename_regex: {
          type: "string",
          description: "The regex pattern to search for in file names.",
        },
      },
      required: ["filename_regex"],
    },
  },
};

const extract_tool = {
  type: "function",
  function: {
    name: "extract",
    description: "Extract content from email files (.msg, .eml) and zip archives (.zip).",
    parameters: {
      type: "object",
      properties: {
        filename: {
          type: "string",
          description: "The path to the file to extract.",
        },
      },
      required: ["filename"],
    },
  },
};

const map_query_tool = {
  type: "function",
  function: {
    name: "map_query",
    description: "Answers a query about individual files in a directory, processed concurrently. Supports text-based files, PDFs, and images (png, jpg, jpeg)",
    parameters: {
      type: "object",
      properties: {
        filenames: {
          type: "array",
          items: {
            type: "string"
          },
          description: "An explicit list of filenames to run the query against.",
        },
        query: {
          type: "string",
          description: "The query to run against each file.",
        },
        broader_context: {
          type: "string",
          description: "broader context to help answer the query"
        }
      },
      required: ["filenames", "query", "broader_context"],
    },
  },
}

const roll_dice_tool = {
  type: "function",
  function: {
    name: "roll_dice",
    description: "Roll a six-sided die and get a random number from 1 to 6.",
    parameters: {
      type: "object",
      properties: {},
      required: [],
    },
  },
};

function roll_dice() {
  return Math.floor(Math.random() * 6) + 1;
}

function ls(args, rootDir) {
  if (!rootDir) {
    return "Error: Root directory is not specified. Please specify a root directory.";
  }

  const resolvedRootDir = path.resolve(rootDir);
  const targetPath = args.path ? path.join(resolvedRootDir, args.path) : resolvedRootDir;
  const resolvedTargetPath = path.resolve(targetPath);

  if (!resolvedTargetPath.startsWith(resolvedRootDir)) {
    return `Error: Access denied. Path is outside of the root directory.`;
  }

  if (!fs.existsSync(resolvedTargetPath) || !fs.statSync(resolvedTargetPath).isDirectory()) {
    return `Error: ${resolvedTargetPath} is not a directory or does not exist.`;
  }

  const items = fs.readdirSync(resolvedTargetPath).map(item => {
    const itemPath = path.join(resolvedTargetPath, item);
    return fs.statSync(itemPath).isDirectory() ? `${item}/` : item;
  });

  return items.sort();
}

function find(args, rootDir) {
  if (!rootDir) {
    return "Error: Root directory is not specified. Please specify a root directory.";
  }

  const resolvedRootDir = path.resolve(rootDir);
  
  const regex = new RegExp(args.filename_regex);
  const allFiles = [];

  function recursiveFind(dir) {
    const currentPath = path.resolve(dir);
    if (!currentPath.startsWith(resolvedRootDir)) {
      return;
    }
    
    const items = fs.readdirSync(dir);
    for (const item of items) {
      const fullPath = path.join(dir, item);
      if (fs.statSync(fullPath).isDirectory()) {
        recursiveFind(fullPath);
      } else {
        allFiles.push(path.relative(resolvedRootDir, fullPath));
      }
    }
  }

  recursiveFind(resolvedRootDir);

  const matchingFiles = allFiles.filter(f => regex.test(path.basename(f)));

  return {
    showing: matchingFiles.length,
    total: matchingFiles.length,
    files: matchingFiles.sort(),
  };
}


async function extract(args, rootDir) {
  if (!rootDir) {
    return "Error: Root directory is not specified. Please specify a root directory.";
  }

  const resolvedRootDir = path.resolve(rootDir);
  const filePath = path.join(resolvedRootDir, args.filename);
  const resolvedFilePath = path.resolve(filePath);

  if (!resolvedFilePath.startsWith(resolvedRootDir)) {
    return `Error: Access denied. Path is outside of the root directory.`;
  }
  
  if (!fs.existsSync(resolvedFilePath)) {
    return `Error: File not found: ${args.filename}`;
  }
  
  const fileExtension = path.extname(resolvedFilePath).toLowerCase();
  const extractionFolder = resolvedFilePath + '.extracted';
  if (!fs.existsSync(extractionFolder)) {
    fs.mkdirSync(extractionFolder);
  }

  if (fileExtension === '.zip') {
    try {
      const zip = new AdmZip(resolvedFilePath);
      zip.extractAllTo(extractionFolder, true);
    } catch (e) {
      return `Error extracting zip file: ${e.message}`;
    }
  } else if (fileExtension === '.eml' || fileExtension === '.msg') {
    try {
      await extractEmail(resolvedFilePath, extractionFolder);
    } catch (err) {
      return `Error parsing email file: ${err.message}`;
    }
  } else {
    return `Error: Unsupported file type for extraction: ${fileExtension}.`;
  }

  const extractedFiles = fs.readdirSync(extractionFolder);
  return {
    status: "success",
    extraction_folder: path.relative(resolvedRootDir, extractionFolder),
    extracted_files: extractedFiles,
    total_files: extractedFiles.length
  };
}


export const tools = [
  roll_dice_tool,
  ls_tool,
  find_tool,
  extract_tool,
  map_query_tool,
];

export const toolFunctions = {
  roll_dice,
  ls,
  find,
  extract,
  map_query,
};

const store = new Store();
const MAP_MODEL_NAME = "google/gemini-2.5-flash-preview-05-20:thinking";

async function map_query(args, rootDir) {
  if (!rootDir) {
    return "Error: Root directory is not specified. Please specify a root directory.";
  }

  const apiKey = store.get('apiKey');
  if (!apiKey) {
    return "Error: API key is not configured. Please set it in the settings.";
  }
  const { filenames, query, broader_context } = args;
  const resolvedRootDir = path.resolve(rootDir);
  const concurrencyLimit = 50;
  const results = {};
  const queue = [...filenames];

  // Initialize OpenAI client once
  const openai = new OpenAI({
    baseURL: 'https://openrouter.ai/api/v1',
    apiKey: apiKey,
  });
  const processFile = async (filename) => {
    try {
      const filePath = path.join(resolvedRootDir, filename);
      const resolvedFilePath = path.resolve(filePath);

      // Security check: ensure file is within rootDir
      if (!resolvedFilePath.startsWith(resolvedRootDir)) {
        results[filename] = { ans: `Error: Access denied. Path is outside of the root directory.`, relevant_extracts: [] };
        return;
      }

      if (!fs.existsSync(resolvedFilePath)) {
        results[filename] = { ans: `File not found: ${filename}`, relevant_extracts: [] };
        return;
      }

      // Check file size to prevent memory issues
      const stats = fs.statSync(resolvedFilePath);
      const maxFileSize = 50 * 1024 * 1024; // 50MB limit
      if (stats.size > maxFileSize) {
        results[filename] = { ans: `Error: File too large (${(stats.size / 1024 / 1024).toFixed(2)}MB). Maximum size is 50MB.`, relevant_extracts: [] };
        return;
      }

      let messages = [
        { role: 'system', content: 'You are a helpful assistant that answers questions about files. Your answer must be grounded.' },
      ];

      const fileBuffer = fs.readFileSync(resolvedFilePath);
      const fileTypeResult = await fileTypeFromBuffer(fileBuffer);
      
      if (fileTypeResult && (fileTypeResult.mime.startsWith('image/') || fileTypeResult.mime === 'application/pdf')) {
        const fileContent = fileBuffer.toString('base64');
        messages.push({
          role: 'user',
          content: [
            {
              type: 'file',
              file: {
                filename: filename,
                file_data: `data:${fileTypeResult.mime};base64,${fileContent}`
              },
            },
            { type: 'text', text: `File: ${filename}` },
            { type: 'text', text: `Broader context:\n${broader_context}` },
            { type: 'text', text: `Based on the file and context, answer the below query. Your answer must be grounded.` },
            { type: 'text', text: `Query:\n${query}` },
          ]
        });
      } else { // Assume text
        const content = fileBuffer.toString('utf-8');
        messages.push({
          role: 'user', content:
            [
              { type: 'text', text: `File Content:\n${content}` },
              { type: 'text', text: `File: ${filename}` },
              { type: 'text', text: `Broader context:\n${broader_context}` },
              { type: 'text', text: `Based on the above file and context, answer the below query. Your answer must be grounded.` },
              { type: 'text', text: `Query:\n${query}` },
            ]
        });
      }

      console.log(`sub_llm start: ${filename}`);
      const response = await openai.chat.completions.create({
        model: MAP_MODEL_NAME,
        messages: messages,
      });
      console.log(`sub_llm done: ${filename}`);

      results[filename] = { ans: response.choices[0].message.content, relevant_extracts: [] };
    } catch (error) {
      results[filename] = {
        ans: `Error processing file: ${error.message}`,
        relevant_extracts: []
      };
    }
  };

  const worker = async () => {
    while (queue.length > 0) {
      const filename = queue.shift();
      if (filename) {
        await processFile(filename);
      }
    }
  };

  const workers = Array(concurrencyLimit).fill(null).map(() => worker());
  await Promise.all(workers);

  return results;
}