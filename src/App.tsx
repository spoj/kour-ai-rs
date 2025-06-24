import { useState } from "react";
import "./App.css";
import { chatCompletion, getSettings, setSettings } from "./commands";
import { IChatCompletionMessage } from "./types";

function App() {
  const [messages, setMessages] = useState<IChatCompletionMessage[]>([]);
  const [input, setInput] = useState("");

  const handleSend = async () => {
    const newMessages: IChatCompletionMessage[] = [
      ...messages,
      { role: "user", content: input },
    ];
    setMessages(newMessages);
    setInput("");
    const response = await chatCompletion({
      apiKey: "dummy",
      modelName: "dummy",
      messages: newMessages,
    });
    setMessages([...newMessages, { role: "assistant", content: response }]);
  };

  return (
    <main
      style={{
        height: "100vh",
        display: "flex",
        flexDirection: "column",
        margin: "auto",
        maxWidth: "768px",
      }}
    >
      <div style={{ flex: "1 1 auto", overflowY: "auto" }}>
        {messages.map((m, i) => (
          <div key={i}>
            <b>{m.role}</b>: {m.content}
          </div>
        ))}
      </div>
      <div style={{ display: "flex" }}>
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          style={{ flex: "1 1 auto" }}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              handleSend();
            }
          }}
        />
        <button onClick={handleSend}>send</button>
      </div>
    </main>
  );
}

export default App;
