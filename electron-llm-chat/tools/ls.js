import fs from "fs";
import path from "path";

export function ls(args, toolContext) {
  const { rootDir } = toolContext;
  if (!rootDir) {
    return "Error: Root directory is not specified. Please specify a root directory.";
  }

  const resolvedRootDir = path.resolve(rootDir);
  const targetPath = args.path
    ? path.join(resolvedRootDir, args.path)
    : resolvedRootDir;
  const resolvedTargetPath = path.resolve(targetPath);

  if (!resolvedTargetPath.startsWith(resolvedRootDir)) {
    return `Error: Access denied. Path is outside of the root directory.`;
  }

  if (
    !fs.existsSync(resolvedTargetPath) ||
    !fs.statSync(resolvedTargetPath).isDirectory()
  ) {
    return `Error: ${resolvedTargetPath} is not a directory or does not exist.`;
  }

  const items = fs.readdirSync(resolvedTargetPath).map((item) => {
    const itemPath = path.join(resolvedTargetPath, item);
    return fs.statSync(itemPath).isDirectory() ? `${item}/` : item;
  });

  return items.sort();
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
            "The path to list files and directories in. Defaults to the current directory.",
        },
      },
      required: [],
    },
  },
};
