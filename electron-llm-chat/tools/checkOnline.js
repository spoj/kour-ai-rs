import OpenAI from "openai";
import Store from "electron-store";

const store = new Store();

export async function check_online(args) {
  const apiKey = store.get("apiKey");

  const { query, broader_context = "" } = args;

  const openai = new OpenAI({
    baseURL: "https://openrouter.ai/api/v1",
    apiKey: apiKey,
  });

  const response = await openai.chat.completions.create({
    model: "perplexity/sonar",
    messages: [
      {
        role: "user",
        content: [
          {
            type: "text",
            text: "Research user query on the internet. take the broader context in consideration. Give both answer and citations.",
          },
          {
            type: "text",
            text: `Broader context:\n${broader_context}`,
          },
          {
            type: "text",
            text: `Query:\n${query}`,
          },
        ],
      },
    ],
  });

  if (response.choices && response.choices[0].message.content) {
    return {
      content: response.choices[0].message.content,
      citations: response.choices[0].message.annotations,
    };
  }

  return "";
}

export const check_online_tool = {
  type: "function",
  function: {
    name: "check_online",
    description: "Perform an internet search for facts.",
    parameters: {
      type: "object",
      properties: {
        query: {
          type: "string",
          description: "The query to search for.",
        },
        broader_context: {
          type: "string",
          description: "The broader context of the query.",
        },
      },
      required: ["query"],
    },
  },
};
