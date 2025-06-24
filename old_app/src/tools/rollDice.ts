import { ChatCompletionTool } from "openai/resources/chat/completions";

export const rollDiceTool: ChatCompletionTool = {
  type: 'function',
  function: {
    name: 'rollDice',
    description: 'Rolls a dice with a specified number of sides.',
    parameters: {
      type: 'object',
      properties: {
        sides: {
          type: 'number',
          description: 'The number of sides on the dice.',
        },
      },
      required: ['sides'],
    },
  },
};

export const rollDice = async (args: { sides: number }) => {
  const { sides } = args;
  await new Promise(resolve => setTimeout(resolve, 1000));
  const result = Math.floor(Math.random() * sides) + 1;
  return `You rolled a ${result}.`;
};