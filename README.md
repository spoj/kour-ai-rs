# Kour-AI

Your Windows desktop AI assistant for all file lookup, summarising, and instant chat needs.

![{C77845A8-A83E-4374-8523-B62EF9006765}](https://github.com/user-attachments/assets/fe884128-39da-46b2-89cc-3e2a55cc66b7)

Kour-AI is written in Electron to be cross platform, and is a budding application that is capable of interacting with system directories to parse files.


---
## Tech

It uses Vue framework, and written entirely in vanilla JavaScript, with plans to port to TypeScript.

Kour-AI is reliant on the following:

- nodejs: v22.14.0
- npm (**not yarn**)
- [LibreOffice, portable or regular download](https://www.libreoffice.org/)
- [OpenRouter](https://openrouter.ai/)

## Installation and running

1. Create a fork of the repository
2. Download it via HTTP or ssh onto your system
3. Install dependencies
 ```shell
   cd kour-ai && npm install
   ```
4. Run the app
```shell
npm run start
``` 
5. Build
   ```shell
   npm run make
   ```
### To run Kour-AI to its full potential, you will need the following:
- An OpenRouter key: This is needed to make API calls for the AI
- LibreOffice: To allow the app to read `.pptx`, `.docx` formats
  - In particular, you will need to provide the app the link to the `soffice.com` file in the LibreOffice installation: `C:\Users\<path to LibreOffice>\App\libreoffice\program`
