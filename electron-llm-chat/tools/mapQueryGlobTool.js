import { find } from "./find.js";
import { map_query } from "./mapQueryTool.js";

const MAX_FILES = 200;

export async function map_query_glob(args, toolContext) {
  const { glob_pattern, query, broader_context } = args;

  const find_args = { glob_pattern };
  const find_result = await find(find_args, toolContext);

  // Directly return if find encountered an error (e.g., rootDir not specified)
  if (typeof find_result === "string" && find_result.startsWith("Error:")) {
    return find_result;
  }

  const filenames = find_result.files;

  if (filenames.length > MAX_FILES) {
    return "Error: Too many files matched. Please use the map_query tool directly or use more specific patterns iteratively.";
  }

  if (!filenames || filenames.length === 0) {
    return {
      status: "success",
      results: {},
      message: "No files found matching the glob pattern.",
    };
  }

  const map_query_args = { filenames, query, broader_context };
  const result = await map_query(map_query_args, toolContext);
  return result;
}

export const map_query_glob_tool = {
  type: "function",
  function: {
    name: "map_query_glob",
    description: "Runs a query against all files matching a glob pattern.",
    parameters: {
      type: "object",
      properties: {
        glob_pattern: {
          type: "string",
          description: "The glob pattern to find files.",
        },
        query: {
          type: "string",
          description: "The query to run against each file.",
        },
        broader_context: {
          type: "string",
          description: "Broader context to help answer the query.",
        },
      },
      required: ["glob_pattern", "query", "broader_context"],
    },
  },
};