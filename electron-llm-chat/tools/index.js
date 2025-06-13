import fs from 'fs';
import path from 'path';
import AdmZip from "adm-zip";
import { extractEmail } from './email-extractor.js';

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
];

export const toolFunctions = {
  roll_dice,
  ls,
  find,
  extract,
};