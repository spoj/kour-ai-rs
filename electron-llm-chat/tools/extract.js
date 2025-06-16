import fs from "fs";
import path from "path";
import AdmZip from "adm-zip";
import { extractEmail } from "./email-extractor.js";
import { safelyReadFile } from "../helper/fileManager.js";

export async function extract(args, toolContext) {
  const { rootDir } = toolContext;
  const { filename } = args;
  const filePath = path.join(rootDir, filename);

  // Use the file manager to read the file securely
  const fileBuffer = await safelyReadFile(filePath, toolContext);
  const resolvedFilePath = path.resolve(filePath);

  const fileExtension = path.extname(resolvedFilePath).toLowerCase();
  const extractionFolder = resolvedFilePath + ".extracted";
  if (!fs.existsSync(extractionFolder)) {
    fs.mkdirSync(extractionFolder);
  }

  if (fileExtension === ".zip") {
    try {
      const zip = new AdmZip(fileBuffer); // Use buffer instead of path
      zip.extractAllTo(extractionFolder, true);
    } catch (e) {
      return `Error extracting zip file: ${e.message}`;
    }
  } else if (fileExtension === ".eml" || fileExtension === ".msg") {
    // extractEmail requires a filePath, so we write the buffer to a temp file
    // This is not ideal, but avoids a larger refactor of email-extractor.js for now.
    const tempFilePath = path.join(extractionFolder, `temp_${filename}`);
    try {
      await fs.promises.writeFile(tempFilePath, fileBuffer);
      await extractEmail(tempFilePath, extractionFolder);
    } catch (err) {
      return `Error parsing email file: ${err.message}`;
    } finally {
      if (fs.existsSync(tempFilePath)) {
        await fs.promises.unlink(tempFilePath);
      }
    }
  } else {
    return `Error: Unsupported file type for extraction: ${fileExtension}.`;
  }

  const extractedFiles = fs.readdirSync(extractionFolder);
  return {
    status: "success",
    extraction_folder: path.relative(rootDir, extractionFolder),
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
