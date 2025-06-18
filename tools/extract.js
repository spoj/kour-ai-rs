import fs from "fs";
import path from "path";
import AdmZip from "adm-zip";
import { extractEmail } from "./email-extractor.js";
import { safelyReadFile } from "../helper/fileManager.js";

export async function extract(args, toolContext) {
  const { rootDir } = toolContext;
  const { filename } = args;
  
  if (!filename) {
    return "Error: filename is required.";
  }

  const filePath = path.join(rootDir, filename);

  try {
    // Use the file manager to read the file securely
    const fileBuffer = await safelyReadFile(filePath, toolContext);
    const resolvedFilePath = path.resolve(filePath);

    const fileExtension = path.extname(resolvedFilePath).toLowerCase();
    const extractionFolder = resolvedFilePath + ".extracted";
    
    // Ensure extraction folder exists with recursive creation
    try {
      await fs.promises.mkdir(extractionFolder, { recursive: true });
    } catch (err) {
      return `Error creating extraction folder: ${err.message}`;
    }

    if (fileExtension === ".zip") {
      try {
        const zip = new AdmZip(fileBuffer);
        zip.extractAllTo(extractionFolder, true);
      } catch (e) {
        return `Error extracting zip file: ${e.message}`;
      }
    } else if (fileExtension === ".eml" || fileExtension === ".msg") {
      // extractEmail requires a filePath, so we write the buffer to a temp file
      const tempFilePath = path.join(extractionFolder, `temp_${Date.now()}_${path.basename(filename)}`);
      try {
        // Ensure temp file can be written
        await fs.promises.writeFile(tempFilePath, fileBuffer);
        
        // Verify temp file was created successfully
        if (!fs.existsSync(tempFilePath)) {
          throw new Error("Failed to create temporary file");
        }
        
        await extractEmail(tempFilePath, extractionFolder);
      } catch (err) {
        return `Error parsing email file: ${err.message}`;
      } finally {
        // Clean up temp file
        try {
          if (fs.existsSync(tempFilePath)) {
            await fs.promises.unlink(tempFilePath);
          }
        } catch (unlinkErr) {
          console.warn(`Warning: Could not clean up temp file ${tempFilePath}: ${unlinkErr.message}`);
        }
      }
    } else {
      return `Error: Unsupported file type for extraction: ${fileExtension}. Supported types: .zip, .eml, .msg`;
    }

    // Verify extraction folder still exists and read contents
    if (!fs.existsSync(extractionFolder)) {
      return "Error: Extraction folder was not created or was removed during extraction.";
    }

    const extractedFiles = await fs.promises.readdir(extractionFolder);
    return {
      status: "success",
      extraction_folder: path.relative(rootDir, extractionFolder),
      extracted_files: extractedFiles,
      total_files: extractedFiles.length,
    };
  } catch (err) {
    return `Error during extraction: ${err.message}`;
  }
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