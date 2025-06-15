import path from "path";
import sharp from "sharp";
import fs from "fs";
import pkg from 'pdfjs-dist/legacy/build/pdf.js';
const { getDocument } = pkg;
import { createCanvas } from "canvas";
import { safelyReadFile, determineFileType, convertToPdf } from "../fileManager.js";

export const produce_crop_tool = {
  type: "function",
  function: {
    name: "produce_crop",
    description:
      "Crops a region from a supported file, saves it as crop.png in the sandbox directory. Coordinates are given as integers from 0 to 1000 and are scaled to page dimensions. Supported formats: .pdf, .pptx, .docx, .png, .jpg, .jpeg.",
    parameters: {
      type: "object",
      properties: {
        filename: {
          type: "string",
          description: "The path to the file to crop, relative to the root directory.",
        },
        page: {
          type: "integer",
          description: "The page number to crop from (for multi-page documents like PDFs). Starts from 1.",
        },
        y_min: { type: "integer", description: "The top coordinate of the crop box (0-1000)." },
        x_min: { type: "integer", description: "The left coordinate of the crop box (0-1000)." },
        y_max: { type: "integer", description: "The bottom coordinate of the crop box (0-1000)." },
        x_max: { type: "integer", description: "The right coordinate of the crop box (0-1000)." },
      },
      required: ["filename", "page", "y_min", "x_min", "y_max", "x_max"],
    },
  },
};

async function getPageAsImageBuffer(buffer, fileType, page, context) {
    let imageBuffer, metadata;

    if (fileType.mime.startsWith('image/')) {
        imageBuffer = buffer;
        metadata = await sharp(imageBuffer).metadata();
    } else {
        let pdfBuffer;
        if (fileType.mime === 'application/pdf') {
            pdfBuffer = buffer;
        } else if (
            fileType.mime === 'application/vnd.openxmlformats-officedocument.wordprocessingml.document' || fileType.ext === 'docx' ||
            fileType.mime === 'application/vnd.openxmlformats-officedocument.presentationml.presentation' || fileType.ext === 'pptx'
        ) {
            const ext = fileType.ext || (fileType.mime.includes('word') ? 'docx' : 'pptx');
            pdfBuffer = await convertToPdf(buffer, ext, context);
        } else {
            throw new Error(`Unsupported file type for cropping: ${fileType.mime}`);
        }

        const doc = await getDocument({ data: pdfBuffer, useSystemFonts: true }).promise;
        if (page < 1 || page > doc.numPages) {
            throw new Error(`Invalid page number: ${page}. File has ${doc.numPages} pages.`);
        }
        const pdfPage = await doc.getPage(page);
        const viewport = pdfPage.getViewport({ scale: 2.0 }); // High resolution

        const canvas = createCanvas(viewport.width, viewport.height);
        const context_canvas = canvas.getContext('2d');
        
        await pdfPage.render({ canvasContext: context_canvas, viewport: viewport }).promise;
        imageBuffer = canvas.toBuffer('image/png');
        metadata = { width: viewport.width, height: viewport.height };
    }

    return { imageBuffer, width: metadata.width, height: metadata.height };
}


export async function produce_crop(args, toolContext) {
  const { filename, page, y_min, x_min, y_max, x_max } = args;
  const { sandboxDir, rootDir } = toolContext;
  const filePath = path.join(rootDir, filename);

  const fileBuffer = await safelyReadFile(filePath, toolContext);
  const fileType = await determineFileType(fileBuffer, filePath);

  const { imageBuffer, width, height } = await getPageAsImageBuffer(fileBuffer, fileType, page, toolContext);
  
  const cropRegion = {
    left: Math.floor((x_min / 1000) * width),
    top: Math.floor((y_min / 1000) * height),
    width: Math.floor(((x_max - x_min) / 1000) * width),
    height: Math.floor(((y_max - y_min) / 1000) * height),
  };

  if (cropRegion.width <= 0 || cropRegion.height <= 0 || !Number.isFinite(cropRegion.width) || !Number.isFinite(cropRegion.height)) {
    throw new Error("Invalid crop dimensions. Ensure the coordinates result in a positive-sized area.");
  }
  
  const finalImageBuffer = await sharp(imageBuffer)
    .extract(cropRegion)
    .png()
    .toBuffer();

  const outputFilename = `crop_${Date.now()}.png`;
  const outputPath = path.join(sandboxDir, outputFilename);
  await fs.promises.writeFile(outputPath, finalImageBuffer);

  return { savedTo: `sandbox://${outputFilename}` };
}