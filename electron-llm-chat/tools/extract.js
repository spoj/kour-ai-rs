import fs from "fs";
import path from "path";
import AdmZip from "adm-zip";
import { extractEmail } from "./email-extractor.js";
export async function extract(args, toolContext) {
  const { rootDir } = toolContext;
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
  const extractionFolder = resolvedFilePath + ".extracted";
  if (!fs.existsSync(extractionFolder)) {
    fs.mkdirSync(extractionFolder);
  }

  if (fileExtension === ".zip") {
    try {
      const zip = new AdmZip(resolvedFilePath);
      zip.extractAllTo(extractionFolder, true);
    } catch (e) {
      return `Error extracting zip file: ${e.message}`;
    }
  } else if (fileExtension === ".eml" || fileExtension === ".msg") {
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
    total_files: extractedFiles.length,
  };
}

export const extract_tool = {
  type: "function",
  function: {
    name: "extract",
    description:
      "Extract content from email files (.msg, .eml) and zip archives (.zip).",
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
