import fs from "fs";
import path from "path";
import sharp from "sharp";
import pkg from 'pdfjs-dist/legacy/build/pdf.js';
const { getDocument } = pkg;
import { createCanvas } from "canvas";
import { convertOfficeToPdf } from "./utils.js";

export const produce_crop_tool = {
  type: "function",
  function: {
    name: "produce_crop",
    description:
      "Crops a region from a supported file, saves it as crop.png in the root directory. Coordinates are given as integers from 0 to 1000 and are scaled to page dimensions. Supported formats: .pdf, .pptx, .docx, .png, .jpg, .jpeg.",
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
        y_min: {
          type: "integer",
          description: "The top coordinate of the crop box (0-1000).",
        },
        x_min: {
          type: "integer",
          description: "The left coordinate of the crop box (0-1000).",
        },
        y_max: {
          type: "integer",
          description: "The bottom coordinate of the crop box (0-1000).",
        },
        x_max: {
          type: "integer",
          description: "The right coordinate of the crop box (0-1000).",
        },
      },
      required: ["filename", "page", "y_min", "x_min", "y_max", "x_max"],
    },
  },
};

export async function produce_crop(args, toolContext) {
  const { filename, page, y_min, x_min, y_max, x_max } = args;
  const { rootDir, sandboxDir } = toolContext;
  const filePath = path.join(rootDir, filename);

  if (!fs.existsSync(filePath)) {
    return { error: `File not found: ${filename}` };
  }

  const outputFilename = `crop_${Date.now()}.png`;
  const outputPath = path.join(sandboxDir, outputFilename);
  const fileExtension = path.extname(filename).toLowerCase();

  try {
    let imageBuffer;

    if (['.png', '.jpg', '.jpeg'].includes(fileExtension)) {
      const metadata = await sharp(filePath).metadata();
      const cropRegion = {
        left: Math.floor((x_min / 1000) * metadata.width),
        top: Math.floor((y_min / 1000) * metadata.height),
        width: Math.floor(((x_max - x_min) / 1000) * metadata.width),
        height: Math.floor(((y_max - y_min) / 1000) * metadata.height),
      };
      imageBuffer = await sharp(filePath).extract(cropRegion).png().toBuffer();
    } else {
      let pdfBuffer;
      if (['.docx', '.pptx'].includes(fileExtension)) {
        const fileType = fileExtension.substring(1);
        pdfBuffer = await convertOfficeToPdf(filePath, fileType);
      } else if (fileExtension === '.pdf') {
        pdfBuffer = await fs.promises.readFile(filePath);
      } else {
        return { error: `Unsupported file type for cropping: ${fileExtension}` };
      }

      if (pdfBuffer) {
        const doc = await getDocument({ data: pdfBuffer, useSystemFonts: true }).promise;
        if (page < 1 || page > doc.numPages) {
          return { error: `Invalid page number: ${page}. File has ${doc.numPages} pages.` };
        }
        const pdfPage = await doc.getPage(page);
        const viewport = pdfPage.getViewport({ scale: 2.0 }); // High resolution
        
        const canvas = createCanvas(viewport.width, viewport.height);
        const context = canvas.getContext('2d');
        
        await pdfPage.render({ canvasContext: context, viewport: viewport }).promise;

        const cropRegion = {
            left: Math.floor((x_min / 1000) * viewport.width),
            top: Math.floor((y_min / 1000) * viewport.height),
            width: Math.floor(((x_max - x_min) / 1000) * viewport.width),
            height: Math.floor(((y_max - y_min) / 1000) * viewport.height),
        };

        imageBuffer = await sharp(canvas.toBuffer('image/png')).extract(cropRegion).png().toBuffer();
      }
    }

    if (imageBuffer) {
      await fs.promises.writeFile(outputPath, imageBuffer);
      return { savedTo: `sandbox://${outputFilename}` };
    } else {
      return { error: "Failed to generate image crop." };
    }
  } catch (error) {
    console.error("Error in produce_crop:", error);
    return { error: `An error occurred: ${error.message}` };
  }
}