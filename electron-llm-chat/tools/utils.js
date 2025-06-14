import fs from "fs";
import path from "path";
import { exec } from "child_process";
import { promisify } from "util";
import os from "os";
import Store from "electron-store";

const execAsync = promisify(exec);
const store = new Store();

export async function convertOfficeToPdf(filePath, fileType) {
  const startTime = Date.now();
  const fileName = path.basename(filePath);
  console.log(`[SOFFICE] Starting ${fileType.toUpperCase()} conversion: ${fileName}`);

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
    fs.promises.rm(tempDir, { recursive: true, force: true }).catch(() => { });
    fs.promises.rm(tempProfileDir, { recursive: true, force: true }).catch(() => { });

    const endTime = Date.now();
    const duration = ((endTime - startTime) / 1000).toFixed(2);
    console.log(`[SOFFICE] Completed ${fileType.toUpperCase()} conversion: ${fileName} (${duration}s)`);

    return pdfBuffer;
  } catch (error) {
    // Cleanup on error asynchronously (don't wait)
    fs.promises.rm(tempDir, { recursive: true, force: true }).catch(() => { });
    fs.promises.rm(tempProfileDir, { recursive: true, force: true }).catch(() => { });

    const endTime = Date.now();
    const duration = ((endTime - startTime) / 1000).toFixed(2);
    console.log(`[SOFFICE] Failed ${fileType.toUpperCase()} conversion: ${fileName} (${duration}s) - ${error.message}`);

    throw new Error(`Office to PDF conversion failed: ${error.message}`);
  }
}