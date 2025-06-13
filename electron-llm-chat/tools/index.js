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

const tools = [
  roll_dice_tool,
];

const toolFunctions = {
  roll_dice,
};

module.exports = {
  tools,
  toolFunctions,
};