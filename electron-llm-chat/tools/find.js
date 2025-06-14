import fs from "fs";
import path from "path";

export function find(args, rootDir) {
  if (!rootDir) {
    return "Error: Root directory is not specified. Please specify a root directory.";
  }

  const resolvedRootDir = path.resolve(rootDir);

  const regex = new RegExp(args.filename_regex);
  const allFiles = [];

  function recursiveFind(dir) {
    const currentPath = path.resolve(dir);
    if (!currentPath.startsWith(resolvedRootDir)) {
      return;
    }

    const items = fs.readdirSync(dir);
    for (const item of items) {
      const fullPath = path.join(dir, item);
      if (fs.statSync(fullPath).isDirectory()) {
        recursiveFind(fullPath);
      } else {
        allFiles.push(path.relative(resolvedRootDir, fullPath));
      }
    }
  }

  recursiveFind(resolvedRootDir);

  const matchingFiles = allFiles.filter((f) => regex.test(path.basename(f)));

  return {
    showing: matchingFiles.length,
    total: matchingFiles.length,
    files: matchingFiles.sort(),
  };
}

export const find_tool = {
  type: "function",
  function: {
    name: "find",
    description: "Find files matching regex pattern.",
    parameters: {
      type: "object",
      properties: {
        filename_regex: {
          type: "string",
          description: "The regex pattern to search for in file names.",
        },
      },
      required: ["filename_regex"],
    },
  },
};
