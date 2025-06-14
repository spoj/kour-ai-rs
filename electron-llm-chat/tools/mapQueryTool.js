import fs from "fs";
import path from "path";
import OpenAI from "openai";
import Store from "electron-store";
import { fileTypeFromBuffer } from "file-type";
import { exec } from "child_process";
import { promisify } from "util";
import os from "os";

const execAsync = promisify(exec);
const store = new Store();
const MAP_MODEL_NAME = "google/gemini-2.5-flash-preview-05-20:thinking";

// Helper functions for Office document conversion
async function convertOfficeToPdf(filePath, fileType) {
  // Get soffice path from settings
  const sofficePath = store.get('sofficePath');
  
  if (!sofficePath) {
    throw new Error('LibreOffice (soffice.com) path is not configured. Please set it in the settings to enable DOCX/PPTX support.');
  }
  
  // Check if the soffice path exists
  if (!fs.existsSync(sofficePath)) {
    throw new Error(`LibreOffice (soffice.com) not found at configured path: ${sofficePath}`);
  }
  
  const tempDir = await fs.promises.mkdtemp(path.join(os.tmpdir(), 'office-convert-'));
  const tempProfileDir = await fs.promises.mkdtemp(path.join(os.tmpdir(), 'office-profile-'));
  
  try {
    // Use soffice.com with headless mode and a temporary profile for parallelization
    const outputDir = tempDir;
    const command = `"${sofficePath}" --headless --invisible --nodefault --nolockcheck --nologo --norestore --convert-to pdf --outdir "${outputDir}" "-env:UserInstallation=file:///${tempProfileDir.replace(/\\/g, '/')}" "${filePath}"`;
    
    // Execute conversion asynchronously without blocking
    await execAsync(command);
    
    // Poll for the output file instead of using a fixed delay
    const baseName = path.basename(filePath, path.extname(filePath));
    const pdfPath = path.join(outputDir, `${baseName}.pdf`);
    
    // Poll for file existence with exponential backoff
    let attempts = 0;
    const maxAttempts = 20;
    while (attempts < maxAttempts) {
      if (fs.existsSync(pdfPath)) {
        // Check if file size is stable (conversion complete)
        const size1 = fs.statSync(pdfPath).size;
        await new Promise(resolve => setTimeout(resolve, 100));
        const size2 = fs.statSync(pdfPath).size;
        
        if (size1 === size2 && size1 > 0) {
          break;
        }
      }
      
      // Exponential backoff: 50ms, 100ms, 200ms, etc.
      await new Promise(resolve => setTimeout(resolve, Math.min(50 * Math.pow(2, attempts), 1000)));
      attempts++;
    }
    
    if (!fs.existsSync(pdfPath)) {
      throw new Error(`PDF conversion failed - output file not found after ${maxAttempts} attempts: ${pdfPath}`);
    }
    
    // Read the PDF file
    const pdfBuffer = await fs.promises.readFile(pdfPath);
    
    // Cleanup temp files asynchronously (don't wait)
    fs.promises.rm(tempDir, { recursive: true, force: true }).catch(() => {});
    fs.promises.rm(tempProfileDir, { recursive: true, force: true }).catch(() => {});
    
    return pdfBuffer;
  } catch (error) {
    // Cleanup on error asynchronously (don't wait)
    fs.promises.rm(tempDir, { recursive: true, force: true }).catch(() => {});
    fs.promises.rm(tempProfileDir, { recursive: true, force: true }).catch(() => {});
    
    throw new Error(`Office to PDF conversion failed: ${error.message}`);
  }
}

async function docxToPdf(filePath) {
  return convertOfficeToPdf(filePath, 'docx');
}

async function pptxToPdf(filePath) {
  return convertOfficeToPdf(filePath, 'pptx');
}

// File type handlers
const fileHandlers = {
  async handleImage(fileBuffer, filename, fileTypeResult, query, broader_context) {
    const fileContent = fileBuffer.toString("base64");
    return {
      role: "user",
      content: [
        {
          type: "image_url",
          image_url: {
            url: `data:${fileTypeResult.mime};base64,${fileContent}`,
          },
        },
        { type: "text", text: `File: ${filename}` },
        { type: "text", text: `Broader context:\n${broader_context}` },
        {
          type: "text",
          text: `Based on the file and context, answer the below query. Your answer must be grounded.`,
        },
        { type: "text", text: `Query:\n${query}` },
      ],
    };
  },

  async handlePdf(fileBuffer, filename, fileTypeResult, query, broader_context) {
    const fileContent = fileBuffer.toString("base64");
    return {
      role: "user",
      content: [
        {
          type: "file",
          file: {
            filename: filename,
            file_data: `data:${fileTypeResult.mime};base64,${fileContent}`,
          },
        },
        { type: "text", text: `File: ${filename}` },
        { type: "text", text: `Broader context:\n${broader_context}` },
        {
          type: "text",
          text: `Based on the file and context, answer the below query. Your answer must be grounded.`,
        },
        { type: "text", text: `Query:\n${query}` },
      ],
    };
  },

  async handleText(fileBuffer, filename, query, broader_context) {
    const content = fileBuffer.toString("utf-8");
    return {
      role: "user",
      content: [
        { type: "text", text: `File Content:\n${content}` },
        { type: "text", text: `File: ${filename}` },
        { type: "text", text: `Broader context:\n${broader_context}` },
        {
          type: "text",
          text: `Based on the above file and context, answer the below query. Your answer must be grounded.`,
        },
        { type: "text", text: `Query:\n${query}` },
      ],
    };
  },

  async handleDocx(filePath, filename, query, broader_context) {
    // Check if soffice is configured
    const sofficePath = store.get('sofficePath');
    if (!sofficePath) {
      return { error: `DOCX files are not supported. Please configure LibreOffice (soffice.com) path in settings to enable DOCX support.` };
    }
    
    try {
      const pdfBuffer = await docxToPdf(filePath);
      const pdfContent = pdfBuffer.toString("base64");
      return {
        role: "user",
        content: [
          {
            type: "file",
            file: {
              filename: filename,
              file_data: `data:application/pdf;base64,${pdfContent}`,
            },
          },
          { type: "text", text: `File: ${filename} (converted from DOCX to PDF)` },
          { type: "text", text: `Broader context:\n${broader_context}` },
          {
            type: "text",
            text: `Based on the file and context, answer the below query. Your answer must be grounded.`,
          },
          { type: "text", text: `Query:\n${query}` },
        ],
      };
    } catch (error) {
      return { error: error.message };
    }
  },

  async handlePptx(filePath, filename, query, broader_context) {
    // Check if soffice is configured
    const sofficePath = store.get('sofficePath');
    if (!sofficePath) {
      return { error: `PPTX files are not supported. Please configure LibreOffice (soffice.com) path in settings to enable PPTX support.` };
    }
    
    try {
      const pdfBuffer = await pptxToPdf(filePath);
      const pdfContent = pdfBuffer.toString("base64");
      return {
        role: "user",
        content: [
          {
            type: "file",
            file: {
              filename: filename,
              file_data: `data:application/pdf;base64,${pdfContent}`,
            },
          },
          { type: "text", text: `File: ${filename} (converted from PPTX to PDF)` },
          { type: "text", text: `Broader context:\n${broader_context}` },
          {
            type: "text",
            text: `Based on the file and context, answer the below query. Your answer must be grounded.`,
          },
          { type: "text", text: `Query:\n${query}` },
        ],
      };
    } catch (error) {
      return { error: error.message };
    }
  },
};

// Main handler dispatcher
async function getMessageContent(fileBuffer, filename, filePath, fileTypeResult, query, broader_context) {
  // Handle images
  if (fileTypeResult && fileTypeResult.mime.startsWith("image/")) {
    return fileHandlers.handleImage(fileBuffer, filename, fileTypeResult, query, broader_context);
  }
  
  // Handle PDFs
  if (fileTypeResult && fileTypeResult.mime === "application/pdf") {
    return fileHandlers.handlePdf(fileBuffer, filename, fileTypeResult, query, broader_context);
  }
  
  // Handle DOCX files
  if (fileTypeResult && (
    fileTypeResult.mime === "application/vnd.openxmlformats-officedocument.wordprocessingml.document" ||
    fileTypeResult.ext === "docx"
  )) {
    return fileHandlers.handleDocx(filePath, filename, query, broader_context);
  }
  
  // Handle PPTX files
  if (fileTypeResult && (
    fileTypeResult.mime === "application/vnd.openxmlformats-officedocument.presentationml.presentation" ||
    fileTypeResult.ext === "pptx"
  )) {
    return fileHandlers.handlePptx(filePath, filename, query, broader_context);
  }
  
  // Handle text files
  if (fileTypeResult && fileTypeResult.mime.startsWith("text/")) {
    return fileHandlers.handleText(fileBuffer, filename, query, broader_context);
  }
  
  // Fallback for plain text files that file-type may not identify
  if (!fileTypeResult) {
    const content = fileBuffer.toString("utf-8");
    // Basic check to see if it's likely binary gibberish
    if (content.includes("\uFFFD")) {
      return { error: "Unsupported file type. Appears to be a binary file." };
    }
    return fileHandlers.handleText(fileBuffer, filename, query, broader_context);
  }
  
  // Unsupported file type
  return {
    error: `Unsupported file type: ${fileTypeResult.mime}. Only images, PDFs, DOCX, PPTX, and text-based files are supported.`,
  };
}

export const map_query_tool = {
  type: "function",
  function: {
    name: "map_query",
    description:
      "Answers a query about individual files in a directory, processed concurrently. Supports text-based files, PDFs, images (png, jpg, jpeg), DOCX, and PPTX files",
    parameters: {
      type: "object",
      properties: {
        filenames: {
          type: "array",
          items: {
            type: "string",
          },
          description:
            "An explicit list of filenames to run the query against.",
        },
        query: {
          type: "string",
          description: "The query to run against each file.",
        },
        broader_context: {
          type: "string",
          description: "broader context to help answer the query",
        },
      },
      required: ["filenames", "query", "broader_context"],
    },
  },
};

export async function map_query(args, rootDir) {
  if (!rootDir) {
    return "Error: Root directory is not specified. Please specify a root directory.";
  }

  const apiKey = store.get("apiKey");
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
    baseURL: "https://openrouter.ai/api/v1",
    apiKey: apiKey,
  });
  
  const processFile = async (filename) => {
    try {
      const filePath = path.join(resolvedRootDir, filename);
      const resolvedFilePath = path.resolve(filePath);

      // Security check: ensure file is within rootDir
      if (!resolvedFilePath.startsWith(resolvedRootDir)) {
        results[filename] = {
          ans: `Error: Access denied. Path is outside of the root directory.`,
          relevant_extracts: [],
        };
        return;
      }

      if (!fs.existsSync(resolvedFilePath)) {
        results[filename] = {
          ans: `File not found: ${filename}`,
          relevant_extracts: [],
        };
        return;
      }

      // Check file size to prevent memory issues
      const stats = await fs.promises.stat(resolvedFilePath);
      const maxFileSize = 50 * 1024 * 1024; // 50MB limit
      if (stats.size > maxFileSize) {
        results[filename] = {
          ans: `Error: File too large (${(stats.size / 1024 / 1024).toFixed(
            2
          )}MB). Maximum size is 50MB.`,
          relevant_extracts: [],
        };
        return;
      }

      // Read file asynchronously
      const fileBuffer = await fs.promises.readFile(resolvedFilePath);
      const fileTypeResult = await fileTypeFromBuffer(fileBuffer);

      // Get message content using the handler system
      const messageContent = await getMessageContent(
        fileBuffer,
        filename,
        resolvedFilePath,
        fileTypeResult,
        query,
        broader_context
      );

      // Check if there was an error
      if (messageContent.error) {
        results[filename] = {
          ans: `Error: ${messageContent.error}`,
          relevant_extracts: [],
        };
        return;
      }

      const messages = [
        {
          role: "system",
          content:
            "You are a helpful assistant that answers questions about files. Your answer must be grounded.",
        },
        messageContent,
      ];
      console.log(`sub_llm start: ${filename}`);
      const response = await openai.chat.completions.create({
        model: MAP_MODEL_NAME,
        messages: messages,
      });
      console.log(`sub_llm done: ${filename}`);

      results[filename] = {
        ans: response.choices[0].message.content,
        relevant_extracts: [],
      };
    } catch (error) {
      results[filename] = {
        ans: `Error processing file: ${error.message}`,
        relevant_extracts: [],
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

  const workers = Array(concurrencyLimit)
    .fill(null)
    .map(() => worker());
  await Promise.all(workers);

  return results;
}
