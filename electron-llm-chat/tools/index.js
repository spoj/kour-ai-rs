import { ls, ls_tool } from "./ls.js";
import { find, find_tool } from "./find.js";
import { extract, extract_tool } from "./extract.js";
import { map_query, map_query_tool } from "./mapQueryTool.js";
import { produce_crop, produce_crop_tool } from "./produceCropTool.js";
import {
  map_query_glob,
  map_query_glob_tool,
} from "./mapQueryGlobTool.js";
import {
  read_notes,
  read_notes_tool,
  append_notes,
  append_notes_tool,
} from "./notesTool.js";
import { check_online, check_online_tool } from "./checkOnline.js";
import { load_file, load_file_tool } from "./loadFileTool.js";

const roll_dice_tool = {
  type: "function",
  function: {
    name: "roll_dice",
    description: "Roll a six-sided die and get a random number from 1 to 6.",
    parameters: {
      type: "object",
      properties: {},
      required: [],
    },
  },
};

function roll_dice() {
  return Math.floor(Math.random() * 6) + 1;
}

export const tools = [
  roll_dice_tool,
  ls_tool,
  find_tool,
  extract_tool,
  map_query_tool,
  map_query_glob_tool,
  read_notes_tool,
  append_notes_tool,
  produce_crop_tool,
  check_online_tool,
  load_file_tool,
];

export const toolFunctions = {
  roll_dice,
  ls,
  find,
  extract,
  map_query,
  map_query_glob,
  read_notes,
  append_notes,
  produce_crop,
  check_online,
  load_file,
};
