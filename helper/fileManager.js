import fs from 'fs';
import path from 'path';
import { fileTypeFromBuffer } from 'file-type';
import { exec } from 'child_process';
import { promisify } from 'util';
import os from 'os';
import crypto from 'crypto';
import { processXlsxInWorker } from './xlsxWorker.js';

const execAsync = promisify(exec);

// Internal helper function to get the cache path. Not exported.
function _getCachePathInternal(fileBuffer, originalExtension, targetExtension, context) {
    const { appDataDir } = context;
    if (!appDataDir) return null;

    const hash = crypto.createHash('sha256').update(fileBuffer).digest('hex');
    const cacheDir = path.join(appDataDir, 'coworker', 'conversion_cache');
    const subDir = hash.substring(0, 2);
    const cacheFileName = `${hash.substring(2)}.${targetExtension}`;

    return path.join(cacheDir, subDir, cacheFileName);
}

/**
 * Reads a converted file from the cache.
 * @param {Buffer} fileBuffer The buffer of the original file.
 * @param {string} fromExt The original file extension.
 * @param {string} toExt The target file extension of the converted file.
 * @param {object} context The tool context.
 * @returns {Promise<Buffer|null>} The buffer of the cached converted file, or null if not found.
 */
async function readConversionCache(fileBuffer, fromExt, toExt, context) {
    const cachePath = _getCachePathInternal(fileBuffer, fromExt, toExt, context);
    if (cachePath && fs.existsSync(cachePath)) {
        return fs.promises.readFile(cachePath);
    }
    return null;
}

/**
 * Writes a converted file to the cache.
 * @param {Buffer} origFileBuffer The buffer of the original file.
 * @param {string} fromExt The original file extension.
 * @param {Buffer} convertedFileBuf The buffer of the converted file to be cached.
 * @param {string} toExt The target file extension of the converted file.
 * @param {object} context The tool context.
 */
async function writeConversionCache(origFileBuffer, fromExt, convertedFileBuf, toExt, context) {
    const cachePath = _getCachePathInternal(origFileBuffer, fromExt, toExt, context);
    if (cachePath) {
        await fs.promises.mkdir(path.dirname(cachePath), { recursive: true });
        await fs.promises.writeFile(cachePath, convertedFileBuf);
    }
}


// --- Custom Error Classes ---
export class SecurityError extends Error {
  constructor(message) {
    super(message);
    this.name = 'SecurityError';
  }
}

export class NotFoundError extends Error {
  constructor(message) {
    super(message);
    this.name = 'NotFoundError';
  }
}

export class ConfigurationError extends Error {
    constructor(message) {
      super(message);
      this.name = 'ConfigurationError';
    }
}

export class UnsupportedFileTypeError extends Error {
    constructor(message) {
        super(message);
        this.name = 'UnsupportedFileTypeError';
    }
}


// --- Core Functions ---

export async function safelyReadFile(filePath, context) {
  const { rootDir } = context;
  if (!rootDir) {
    throw new ConfigurationError('Root directory is not specified.');
  }

  const resolvedFilePath = path.resolve(filePath);
  const resolvedRootDir = path.resolve(rootDir);

  if (!resolvedFilePath.startsWith(resolvedRootDir)) {
    throw new SecurityError(`Access denied. Path '${path.basename(filePath)}' is outside of the root directory.`);
  }

  if (!fs.existsSync(resolvedFilePath)) {
    throw new NotFoundError(`File not found: ${path.basename(filePath)}`);
  }
  
  // Check file size
  const stats = await fs.promises.stat(resolvedFilePath);
  const maxFileSize = 50 * 1024 * 1024; // 50MB limit
  if (stats.size > maxFileSize) {
      throw new Error(`File too large (${(stats.size / 1024 / 1024).toFixed(2)}MB). Maximum size is 50MB.`);
  }

  return fs.promises.readFile(resolvedFilePath);
}

export async function determineFileType(buffer, filePath) {
    let fileTypeResult = await fileTypeFromBuffer(buffer);
    if (!fileTypeResult) {
        // Fallback for text-based files without magic numbers
        const content = buffer.toString('utf-8');
        if (!content.includes('\uFFFD')) { // Check for binary gibberish
            return { mime: 'text/plain', ext: 'txt' };
        }
    }
    // Add extension for fallback if mime type is ambiguous
    if (fileTypeResult && !fileTypeResult.ext) {
      fileTypeResult.ext = path.extname(filePath).substring(1);
    }
    return fileTypeResult;
}

export async function convertToPdf(buffer, fileExtension, context) {
    const cachedBuffer = await readConversionCache(buffer, fileExtension, 'pdf', context);
    if (cachedBuffer) {
        return cachedBuffer;
    }

    const { sofficePath } = context;
    if (!sofficePath) {
        throw new ConfigurationError(`${fileExtension.toUpperCase()} files are not supported. Please configure LibreOffice (soffice.com) path in settings.`);
    }

    const tempDir = os.tmpdir();
    const tempFilePath = path.join(tempDir, `tempfile_${Date.now()}.${fileExtension}`);
    await fs.promises.writeFile(tempFilePath, buffer);

    try {
        const command = `"${sofficePath}" --headless --convert-to pdf --outdir "${tempDir}" "${tempFilePath}"`;
        await execAsync(command);
        
        const pdfFileName = path.basename(tempFilePath, `.${fileExtension}`) + '.pdf';
        const pdfPath = path.join(tempDir, pdfFileName);

        if (!fs.existsSync(pdfPath)) {
            throw new Error('PDF conversion failed: output file not found.');
        }

        const pdfBuffer = await fs.promises.readFile(pdfPath);
        
        await writeConversionCache(buffer, fileExtension, pdfBuffer, 'pdf', context);

        return pdfBuffer;
    } finally {
        const pdfFileName = path.basename(tempFilePath, `.${fileExtension}`) + '.pdf';
        const pdfPath = path.join(tempDir, pdfFileName);
        if (fs.existsSync(tempFilePath)) await fs.promises.unlink(tempFilePath);
        if (fs.existsSync(pdfPath)) await fs.promises.unlink(pdfPath);
    }
}

export async function extractTextFromSpreadsheet(buffer, context) {
    const cachedBuffer = await readConversionCache(buffer, 'xlsx', 'csv', context);
    if (cachedBuffer) {
        return cachedBuffer.toString('utf-8');
    }
    
    const fullText = await processXlsxInWorker(buffer);
    
    await writeConversionCache(buffer, 'xlsx', Buffer.from(fullText, 'utf-8'), 'csv', context);

    return fullText;
}


// --- High-Level Abstractions ---

const WELL_KNOWN_TEXT_EXTENSIONS = [
    // Data formats
    'csv', 'tsv', 'json', 'xml', 'yaml', 'toml', 'ini',
    // Plain text & docs
    'txt', 'md', 'log', 'rst',
    // Web
    'html', 'css', 'js', 'jsx', 'ts', 'tsx', 'vue',
    // Scripting & Backend
    'py', 'rb', 'php', 'java', 'c', 'cpp', 'h', 'hpp', 'cs', 'go', 'swift',
    'kt', 'kts', 'scala', 'sh', 'bat', 'ps1', 'pl', 'sql'
];

export async function processFileBufferForLLM(fileBuffer, filename, context) {
    const fileType = await determineFileType(fileBuffer, filename);

    if (!fileType) {
        throw new UnsupportedFileTypeError("Unsupported file type or file appears to be binary.");
    }
    
    // Handle Images
    if (fileType.mime.startsWith('image/')) {
        const fileContent = fileBuffer.toString('base64');
        return {
            type: 'image',
            mime: fileType.mime,
            content: fileContent,
            filename: filename,
        };
    }
    
    // Handle PDF directly
    if (fileType.mime === 'application/pdf') {
         const fileContent = fileBuffer.toString("base64");
         return {
            type: 'pdf',
            mime: fileType.mime,
            content: fileContent,
            filename: filename,
        };
    }

    // Handle Office Docs (DOCX, PPTX) by converting to PDF
    const officeMimes = {
        'application/vnd.openxmlformats-officedocument.wordprocessingml.document': 'docx',
        'application/vnd.openxmlformats-officedocument.presentationml.presentation': 'pptx'
    };
    if (officeMimes[fileType.mime] || fileType.ext === 'docx' || fileType.ext === 'pptx') {
        const ext = officeMimes[fileType.mime] || fileType.ext;
        const pdfBuffer = await convertToPdf(fileBuffer, ext, context);
        const pdfContent = pdfBuffer.toString("base64");
        return {
            type: 'pdf',
            mime: 'application/pdf',
            content: pdfContent,
            filename: filename,
            originalExtension: ext
        };
    }

    // Handle Spreadsheets
    const spreadsheetMimes = [
        'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet', // .xlsx
        'application/vnd.ms-excel' // .xls
    ];
    if (spreadsheetMimes.includes(fileType.mime) || fileType.ext === 'xlsx' || fileType.ext === 'xls') {
        const textContent = await extractTextFromSpreadsheet(fileBuffer, context);
        return {
            type: 'text',
            content: textContent,
            filename: filename,
            isSpreadsheet: true
        };
    }
    
    // Handle well-known text-based files by extension
    const fileExtension = path.extname(filename).substring(1).toLowerCase();
    if (WELL_KNOWN_TEXT_EXTENSIONS.includes(fileExtension)) {
        const textContent = fileBuffer.toString('utf-8');
        return {
            type: 'text',
            content: textContent,
            filename: filename,
        };
    }
    
    throw new UnsupportedFileTypeError(`Unsupported file type: ${fileType.mime || 'unknown'} or extension: .${fileExtension}`);
}

export async function getFileContentForLLM(filePath, context) {
    const fileBuffer = await safelyReadFile(filePath, context);
    const filename = path.basename(filePath);
    return await processFileBufferForLLM(fileBuffer, filename, context);
}
export async function safelyReadDir(dirPath, context) {
  const { rootDir } = context;
  if (!rootDir) {
    throw new ConfigurationError('Root directory is not specified.');
  }

  const resolvedDirPath = path.resolve(dirPath);
  const resolvedRootDir = path.resolve(rootDir);

  if (!resolvedDirPath.startsWith(resolvedRootDir)) {
    throw new SecurityError(`Access denied. Path '${path.basename(dirPath)}' is outside of the root directory.`);
  }

  if (!fs.existsSync(resolvedDirPath) || !fs.statSync(resolvedDirPath).isDirectory()) {
    throw new NotFoundError(`Directory not found or path is not a directory: ${path.basename(dirPath)}`);
  }

  const items = fs.readdirSync(resolvedDirPath).map((item) => {
    const itemPath = path.join(resolvedDirPath, item);
    return fs.statSync(itemPath).isDirectory() ? `${item}/` : item;
  });

  return items.sort();
}
export async function safelyWriteFile(filePath, content, context) {
  const { rootDir } = context;
  if (!rootDir) {
    throw new ConfigurationError('Root directory is not specified.');
  }
  const resolvedFilePath = path.resolve(filePath);
  const resolvedRootDir = path.resolve(rootDir);

  if (!resolvedFilePath.startsWith(resolvedRootDir)) {
    throw new SecurityError(`Access denied. Path '${path.basename(filePath)}' is outside of the root directory.`);
  }

  // Ensure the directory exists
  const dir = path.dirname(resolvedFilePath);
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }

  return fs.promises.writeFile(resolvedFilePath, content);
}