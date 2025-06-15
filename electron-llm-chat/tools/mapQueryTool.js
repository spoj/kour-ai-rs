import path from "path";
import OpenAI from "openai";
import { z } from "zod";
import { zodResponseFormat } from "openai/helpers/zod";
import { prepareFileForLLM } from "../fileManager.js";

const ResultSchema = z.object({
  ans: z
    .string()
    .describe("The answer to the query based on the file content."),
  relevant_extracts: z
    .array(z.string())
    .describe("Relevant extracts from the file that support the answer."),
});

const MAP_MODEL_NAME = "google/gemini-2.5-flash-preview-05-20:thinking";

export const map_query_tool = {
  type: "function",
  function: {
    name: "map_query",
    description:
      "Answers a query about individual files in a directory, processed concurrently. Supports text-based files, PDFs, images (png, jpg, jpeg), DOCX, PPTX, XLSX, and XLS files",
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

export async function map_query(args, toolContext) {
  const { apiKey, providerOrder, rootDir } = toolContext;
  const { filenames, query, broader_context } = args;

  if (!apiKey) {
    return "Error: API key is not configured. Please set it in the settings.";
  }

  const concurrencyLimit = 10; // Reduced for stability with new file processing
  const results = {};
  const queue = [...filenames];

  const openai = new OpenAI({
    baseURL: "https://openrouter.ai/api/v1",
    apiKey: apiKey,
  });

  const processFile = async (filename) => {
    try {
      const filePath = path.join(rootDir, filename);
      const llmContent = await prepareFileForLLM(filePath, toolContext, query, broader_context);

      const messages = [
        {
          role: "system",
          content:
            "You are a helpful assistant that answers questions about files. Your answer must be grounded.",
        },
        llmContent,
      ];

      console.log(`sub_llm start: ${filename}`);
      const response = await openai.chat.completions.parse({
        model: MAP_MODEL_NAME,
        messages: messages,
        response_format: zodResponseFormat(ResultSchema, "result"),
        provider: {
          order: providerOrder,
        },
      });
      console.log(`sub_llm done: ${filename}`);
      results[filename] = response.choices[0].message?.parsed;

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
