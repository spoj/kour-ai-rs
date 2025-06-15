import fs from 'fs';
import path from 'path';
import { fileTypeFromBuffer } from 'file-type';
import { exec } from 'child_process';
import { promisify } from 'util';
import * as XLSX from 'xlsx';
import os from 'os';

const execAsync = promisify(exec);

// Custom Error Classes
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
        return pdfBuffer;
    } finally {
        // Clean up temporary files
        const pdfFileName = path.basename(tempFilePath, `.${fileExtension}`) + '.pdf';
        const pdfPath = path.join(tempDir, pdfFileName);
        if (fs.existsSync(tempFilePath)) await fs.promises.unlink(tempFilePath);
        if (fs.existsSync(pdfPath)) await fs.promises.unlink(pdfPath);
    }
}

export function extractTextFromSpreadsheet(buffer) {
    const workbook = XLSX.read(buffer, { type: 'buffer' });
    let fullText = '';
    workbook.SheetNames.forEach(sheetName => {
        const worksheet = workbook.Sheets[sheetName];
        const csv = XLSX.utils.sheet_to_csv(worksheet);
        fullText += `Sheet: ${sheetName}\n\n${csv}\n\n`;
    });
    return fullText;
}


// --- High-Level Abstractions ---
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
        const textContent = extractTextFromSpreadsheet(fileBuffer);
        return {
            type: 'text',
            content: textContent,
            filename: filename,
            isSpreadsheet: true
        };
    }
    
    // Handle Text-based files
    if (fileType.mime.startsWith('text/')) {
        const textContent = fileBuffer.toString('utf-8');
        return {
            type: 'text',
            content: textContent,
            filename: filename,
        };
    }
    
    throw new UnsupportedFileTypeError(`Unsupported file type: ${fileType.mime || 'unknown'}`);
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