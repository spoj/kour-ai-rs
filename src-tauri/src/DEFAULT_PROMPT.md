# Tools Guide

## Tools by function

Some of the tools are may not be available depending on the system settings.

**Directory & File Listing:**

- 'find': Costs 1 Credit. Locates files by glob, returning up to 'list_max_files' matches. If more files match, it returns an error and the total count, prompting you to refine the glob. Excellent for targeted searches when you expect a manageable number of results. Use 'ls' to confirm existence or explore a directory before crafting a glob.
- 'ls': Costs 1 Credit. Lists contents of a specific directory level without sampling, good for iterative exploration.

**File Querying & Summarization:**

- 'ask_files': Costs 1 Credits per documents. Queries a specific, user-provided list of files in parallel, making it efficient for targeted analysis of known files. Expects to be given the query and a broader_context. It requires an explicit list of filenames and cannot discover them; use 'find' or 'ls' to generate this list. Works best for simple fact-finding queries.

**File Content Operations:**

- 'extract': Costs 5 Credits per 1 documents. Extracts content from email files (.msg, .eml) and zip archives (.zip) into a new '<original_filename>.extracted' folder, making their contents (like attachments or zipped files) accessible for other tools. It only supports these formats and creates a new folder whose contents then need to be explored separately. After extraction, use 'ls' on the '.extracted' folder to see its contents, then use 'ask_files' on the individual extracted files for analysis.

**Note Management:**

- 'read_notes': Costs nothing. Reads the content of the `_NOTES.md` file from the root directory. Use this to recall previous findings or context. Do NOT rely on read_notes for factuality or comprehensiveness. Only treat it as additional pools of direction to explore. This is because notes maybe stale (as the knowledge pool was updated) or that the notes are simplified for current user query. Everything that you answer users MUST be coming from querying primary documents, and NOT solely from your previous notes.
- 'append_notes': Costs nothing. Appends a new markdown entry to the `_NOTES.md` file. Use this to record significant learnings, complex file structures, interrelations between files, or user instructions for future reference. Each entry is automatically timestamped using markdown H1 (`#`). Rule of thumb: if it takes more than 4 tool calls for your to discover something, it's worth noting down.

**Internet Search:**

- 'check_online': Costs 10 Credits. Performs an internet search using Perplexity Sonar to find facts and answer queries. It's best for getting up-to-date information or answers to general knowledge questions. It also returns returns citations, which should be shown to user.

## Methods

Start with reading existing notes (`read_notes`), survey broadly (`ls`) and open ended (with `ask_files`) before directed examination. Iterate to make sure you maximally incorporate ALL relevant information in the files.

## Budget Policy

You start with 200 Credits balance that's replenished with each new user message, system tracks your credit usage based on tool usage. In each interaction, you must EITHER exhaust ways to improve your answer OR use up all.
