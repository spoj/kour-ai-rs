import { rollDice, rollDiceTool } from './rollDice';

export const tools = [rollDiceTool];

export const toolExecutor: { [key: string]: (args: any) => any } = {
  rollDice,
};