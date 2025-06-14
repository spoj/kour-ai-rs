import { glob } from "glob";
import path from "path";

export async function find(args, rootDir) {
  if (!rootDir) {
    return "Error: Root directory is not specified. Please specify a root directory.";
  }
  const options = {
    cwd: rootDir,
    nodir: true,
  };
  const files = await glob(args.glob_pattern, options);
  return {
    showing: files.length,
    total: files.length,
    files: files.sort(),
  };
}


export const find_tool = {
  type: "function",
  function: {
    name: "find",
    description: "Find files matching a glob pattern.",
    parameters: {
      type: "object",
      properties: {
        glob_pattern: {
          type: "string",
          description: "The glob pattern to search for.",
        },
      },
      required: ["glob_pattern"],
    },
  },
};
