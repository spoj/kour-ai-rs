import litellm
from litellm.utils import ModelResponse


async def check_online(query: str, broader_context: str) -> str:
    """Perform internet search"""
    messages = [
        {
            "role": "system",
            "content": "Research user query on the internet. take the broader context in consideration. Give both answer and citations.",
        },
        {
            "role": "user",
            "content": f"Broader context:\n{broader_context}",
        },
        {
            "role": "user",
            "content": f"Query:\n{query}",
        },
    ]

    response: ModelResponse = await litellm.acompletion(
        model="openrouter/perplexity/sonar",
        messages=messages,
    )

    if response.choices and response.choices[0].message.content:
        return response.choices[0].message.content

    return ""
