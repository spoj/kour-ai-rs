import path from "path";
import { safelyReadDir } from "../fileManager.js";

export async function ls(args, toolContext) {
  const { rootDir } = toolContext;
  const targetPath = args.path ? path.join(rootDir, args.path) : rootDir;
  
  // The safelyReadDir function now handles all validation and logic.
  // Note: The function is async, so this function must be async too.
  return await safelyReadDir(targetPath, toolContext);
}

export const ls_tool = {
  type: "function",
  function: {
    name: "ls",
    description:
      "List files and directories in the specified path relative to root directory.",
    parameters: {
      type: "object",
      properties: {
        path: {
          type: "string",
          description:
            "The path to list files and directories in. Defaults to the root directory.",
        },
      },
      required: [],
    },
  },
};
