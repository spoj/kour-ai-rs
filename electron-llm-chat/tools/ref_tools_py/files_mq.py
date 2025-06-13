import json
import base64
import litellm
import re
import fitz
from pathlib import Path
import asyncio
import mimetypes
from typing import Iterator, List
from datetime import datetime
from pydantic import BaseModel, ValidationError
from google.adk.tools.tool_context import ToolContext
from .email_extract import extract_email_attachments
import aiofiles

DEFAULT_LIST_MAX = 500
DEFAULT_MAP_MAX = 200
DEFAULT_MAX_CONCURRENT_TASKS = 50
DEFAULT_MODEL_NAME = "openrouter/google/gemini-2.5-flash-preview-05-20:thinking"


def _convert_with_docling(file_path: Path) -> str:
    """CPU-intensive Docling conversion in separate thread."""
    from docling.document_converter import (
        DocumentConverter,
    )

    converter = DocumentConverter()
    result = converter.convert(str(file_path))
    return result.document.export_to_markdown(embed_images=False)


def _convert_office_to_pdf(file_path: Path) -> bytes:
    """Convert Office documents to PDF using LibreOffice."""
    import subprocess
    import tempfile

    with tempfile.TemporaryDirectory() as temp_out_dir:
        with tempfile.TemporaryDirectory() as temp_profile_dir:
            try:
                user_profile_arg = f"-env:UserInstallation=file://{temp_profile_dir}"
                cmd = [
                    "libreoffice",
                    user_profile_arg,
                    "--headless",
                    "--convert-to",
                    "pdf",
                    "--outdir",
                    temp_out_dir,
                    str(file_path),
                ]
                result = subprocess.run(
                    cmd, capture_output=True, text=True, timeout=120
                )

                if result.returncode != 0:
                    error_message = f"LibreOffice conversion failed with command '{' '.join(cmd)}'. Error: {result.stderr}"
                    if result.stdout:
                        error_message += f"\nStdout: {result.stdout}"
                    raise Exception(error_message)

                pdf_name = file_path.stem + ".pdf"
                pdf_path = Path(temp_out_dir) / pdf_name

                if not pdf_path.exists():
                    raise Exception(
                        f"PDF file not generated: {pdf_path}. Output from LibreOffice: {result.stdout} {result.stderr}"
                    )

                with open(pdf_path, "rb") as f:
                    return f.read()

            except subprocess.TimeoutExpired:
                raise Exception(
                    f"LibreOffice conversion timed out for command: '{' '.join(cmd if 'cmd' in locals() else ['libreoffice', '...'])}'"
                )
            except FileNotFoundError:
                raise Exception(
                    "LibreOffice not found. Please ensure it is installed and in your PATH."
                )


async def _handle_docx(file_path: Path) -> list[dict]:
    """Handles .docx files, creating a file part."""
    try:
        pdf_bytes = await asyncio.to_thread(_convert_office_to_pdf, file_path)
        encoded_data = base64.b64encode(pdf_bytes).decode("utf-8")
        data_url = f"data:application/pdf;base64,{encoded_data}"
        return [
            {
                "type": "file",
                "file": {"filename": file_path.name, "file_data": data_url},
            }
        ]
    except Exception as e:
        print(f"Error converting {file_path.name} to PDF: {e}")
        return [
            {
                "type": "text",
                "text": f"Failed to convert {file_path.name} to PDF. Error: {e}",
            }
        ]


async def _handle_xlsx(file_path: Path) -> list[dict]:
    """Handles .xlsx files using docling, with a size check."""
    try:
        if file_path.stat().st_size > 1024 * 1024:
            print(f"Skipping {file_path}: xlsx file exceeds 1MB size limit")
            return [
                {
                    "type": "text",
                    "text": f"File {file_path.name} skipped: xlsx file exceeds 1MB size limit",
                }
            ]
        markdown_content = await asyncio.to_thread(_convert_with_docling, file_path)
        return [{"type": "text", "text": markdown_content}]
    except Exception as e:
        print(f"Error converting {file_path} with docling: {e}")
        return [await _process_file_direct(file_path)]


async def _handle_pptx(file_path: Path) -> list[dict]:
    """Handles .pptx files, creating a file part."""
    try:
        pdf_bytes = await asyncio.to_thread(_convert_office_to_pdf, file_path)
        encoded_data = base64.b64encode(pdf_bytes).decode("utf-8")
        data_url = f"data:application/pdf;base64,{encoded_data}"
        return [
            {
                "type": "file",
                "file": {"filename": file_path.name, "file_data": data_url},
            }
        ]
    except Exception as e:
        print(f"Error converting {file_path.name} to PDF: {e}")
        return [
            {
                "type": "text",
                "text": f"Failed to convert {file_path.name} to PDF. Error: {e}",
            }
        ]

async def _handle_text(file_path: Path) -> list[dict]:
    """Handles text files, reading them as plain text."""
    try:
        async with aiofiles.open(file_path, "r", encoding="utf-8") as f:
            content = await f.read()
        return [{"type": "text", "text": content}]
    except Exception as e:
        return [{"type": "text", "text": f"Error reading {file_path.name}: {e}"}]


async def _handle_default(file_path: Path) -> list[dict]:
    """Handles all other file types by direct processing."""
    return [await _process_file_direct(file_path)]


FILE_HANDLERS = {
    ".docx": _handle_docx,
    ".xlsx": _handle_xlsx,
    ".pptx": _handle_pptx,
    ".jpg": _handle_default,
    ".jpeg": _handle_default,
    ".png": _handle_default,
    ".txt": _handle_text,
    ".md": _handle_text,
    ".csv": _handle_text,
}


async def file_to_parts(file) -> list[dict]:
    """
    Convert a file to a list of parts using a handler dispatch system.
    Most files return a single part; emails and PPTX can return multiple.

    Args:
        file: Path to the file to process.
    """
    file_path = Path(file)
    file_extension = file_path.suffix.lower()

    # Use handler if available, otherwise use default
    handler = FILE_HANDLERS.get(file_extension, _handle_default)
    return await handler(file_path)


async def _process_file_direct(file_path: Path) -> dict:
    """Process file directly without conversion."""
    async with aiofiles.open(file_path, "rb") as f:
        bytes_content = await f.read()

    mime_type, _ = mimetypes.guess_type(file_path)
    if mime_type is None:
        mime_type = "application/octet-stream"

    encoded_data = base64.b64encode(bytes_content).decode("utf-8")
    data_url = f"data:{mime_type};base64,{encoded_data}"

    if mime_type and mime_type.startswith("image/"):
        return {"type": "image_url", "image_url": {"url": data_url}}
    else:
        return {
            "type": "file",
            "file": {"filename": file_path.name, "file_data": data_url},
        }


class ProcessSingleFileResponse(BaseModel):
    ans: str
    relevant_extracts: List[str]


def _make_process_single_file(model_name: str):
    async def _process_single_file(
        file: Path, query: str, broader_context: str
    ) -> ProcessSingleFileResponse:
        """Processes a single file against the query using LiteLLM."""
        user_content = await file_to_parts(file)

        user_content.insert(
            0,
            {
                "type": "text",
                "text": f"Broader context:\n{broader_context}\n\nQuery:\n{query}",
            },
        )

        messages = [
            {
                "role": "system",
                "content": "You are a helpful assistant that answers questions about files. Your answer must be grounded.",
            },
            {"role": "user", "content": user_content},
        ]

        response = await litellm.acompletion(
            model=model_name,
            messages=messages,
            response_format=ProcessSingleFileResponse,
        )

        try:
            response_json = json.loads(response.choices[0].message.content)
            return ProcessSingleFileResponse(**response_json)
        except (json.JSONDecodeError, ValidationError, TypeError, AttributeError) as e:
            print(
                f"Error decoding JSON response, validating schema, or unexpected response structure: {e}"
            )
            print(
                f"Raw response: {response.choices[0].message.content if response.choices and response.choices[0].message.content else 'No response text'}"
            )
            raise

    return _process_single_file


def make_mq_toolset(
    dir: str,
    *,
    max_concurrent_tasks: int = DEFAULT_MAX_CONCURRENT_TASKS,
    model_name: str = DEFAULT_MODEL_NAME,
    list_max: int = DEFAULT_LIST_MAX,
    map_max: int = DEFAULT_MAP_MAX,
) -> list:
    root_directory = Path(dir)
    _process_single_file = _make_process_single_file(model_name)
    notes_path = root_directory / "_NOTES.md"
    notes_lock = asyncio.Lock()

    async def map_query_tool(
        query: str,
        broader_context: str,
        filenames: list[str],
    ) -> dict[str, dict]:
        """
        Answers a query about individual files in directory, processed concurrently.

        Files are be specified via an explicit list ('filenames'). The query runs against each file independently.
        """
        if not isinstance(filenames, list):
            return {"error": "Filenames must be a list of strings"}

        target_files = []
        for f in filenames:
            file_path = Path(root_directory).joinpath(f)
            if file_path.is_file():
                target_files.append(file_path)

        if not target_files:
            return {"error": "No valid target files found"}

        semaphore = asyncio.Semaphore(max_concurrent_tasks)
        tasks = []

        async def process_file_with_semaphore(file, query, broader_context):
            async with semaphore:
                return await _process_single_file(file, query, broader_context)

        for file in target_files:
            task = asyncio.create_task(
                process_file_with_semaphore(file, query, broader_context)
            )
            tasks.append(task)

        results = await asyncio.gather(*tasks, return_exceptions=True)
        result_dict: dict[str, dict] = {}
        for i, result in enumerate(results):
            file = target_files[i].relative_to(root_directory)
            key = str(file)
            if isinstance(result, Exception):
                result_dict[key] = {
                    "ans": f"Task failed for file '{file}': {result}",
                    "relevant_extracts": [],
                }
            elif isinstance(result, ProcessSingleFileResponse):
                result_dict[key] = result.model_dump()
            else:
                result_dict[key] = {
                    "ans": f"Unexpected result type for file '{file}': {type(result)}",
                    "relevant_extracts": [],
                }
        return result_dict

    async def map_query_tool_regex(
        query: str,
        broader_context: str,
        filename_regex: str,
    ) -> dict[str, dict]:
        """
        Answers a query about individual files in directory, processed concurrently.
        Processes up to 'max_map_results' matching files.

        Files are be specified via a regex ('filename_regex'). The query runs against each file independently.
        """
        regex = re.compile(filename_regex)
        all_files = list(directory_tree_full())

        matching_files = [f for f in all_files if regex.search(f)]
        num_matching_files = len(matching_files)

        if num_matching_files > map_max:
            return {
                "error": f"Found {num_matching_files} files matching regex, which exceeds the limit of {map_max} for map_query_tool_regex. Please use a more specific regex or use the `map_query_tool` instead.",
                "total_found": num_matching_files,
                "limit": map_max,
            }

        return await map_query_tool(
            query=query, broader_context=broader_context, filenames=matching_files
        )

    def find(filename_regex: str) -> dict[str, any]:
        """Find files matching regex pattern, up to 'list_max_files'."""
        all_files = list(directory_tree_full())
        regex = re.compile(filename_regex)
        files = [f for f in all_files if regex.search(f)]

        total_files = len(files)

        if total_files > list_max:
            return {
                "error": f"Found {total_files} files matching regex, which exceeds the limit of {list_max}. Please use a more specific regex or use `ls` instead.",
                "total_found": total_files,
                "limit": list_max,
            }
        else:
            return {
                "showing": total_files,
                "total": total_files,
                "files": sorted(files),
            }

    def directory_tree_full() -> Iterator[str]:
        """Recursively list all files in the root directory, with paths relative to root."""
        root_path = Path(root_directory)
        if not root_path.is_dir():
            yield f"Error: {root_path} is not a directory or does not exist."
            return

        for item in root_path.rglob("*"):
            relative_path = str(item.relative_to(root_directory))
            if item.is_file():
                yield relative_path

    def ls(path: str = "") -> list[str]:
        """List files and directories in the specified path relative to root directory."""
        target_path = Path(root_directory) / path
        if not target_path.is_dir():
            return [f"Error: {target_path} is not a directory or does not exist."]

        items = []
        for item in target_path.iterdir():
            if item.is_dir():
                items.append(f"{item.name}/")
            else:
                items.append(item.name)

        return sorted(items)

    async def extract(filename: str) -> dict[str, str]:
        """Extract content from email files (.msg, .eml) and return information about extracted files."""
        file_path = Path(root_directory) / filename

        if not file_path.is_file():
            return {"error": f"File not found: {filename}"}

        file_extension = file_path.suffix.lower()
        if file_extension not in [".msg", ".eml", ".zip"]:
            return {
                "error": f"Unsupported file type for extraction: {file_extension}. Only .msg, .eml, and .zip files are supported."
            }

        if file_extension == ".zip":
            try:
                import zipfile

                extraction_folder = file_path.parent / f"{file_path.name}.extracted"

                if not extraction_folder.exists():
                    print(f"Extracting zip content from {file_path}")
                    extraction_folder.mkdir(exist_ok=True)
                    with zipfile.ZipFile(file_path, "r") as zip_ref:
                        zip_ref.extractall(extraction_folder)

                extracted_files = []
                if extraction_folder.exists():
                    for item in extraction_folder.rglob("*"):
                        if item.is_file():
                            rel_path = item.relative_to(root_directory)
                            file_size = item.stat().st_size
                            extracted_files.append(
                                {
                                    "path": str(rel_path),
                                    "size": file_size,
                                    "name": item.name,
                                }
                            )

                return {
                    "status": "success",
                    "extraction_folder": str(
                        extraction_folder.relative_to(root_directory)
                    ),
                    "extracted_files": extracted_files,
                    "total_files": len(extracted_files),
                }
            except Exception as e:
                return {"error": f"Failed to extract {filename}: {str(e)}"}

        try:
            extraction_folder = file_path.parent / f"{file_path.name}.extracted"

            if not extraction_folder.exists():
                print(f"Extracting email content from {file_path}")
                await asyncio.to_thread(extract_email_attachments, file_path)

            extracted_files = []
            if extraction_folder.exists():
                for item in extraction_folder.rglob("*"):
                    if item.is_file():
                        rel_path = item.relative_to(root_directory)
                        file_size = item.stat().st_size
                        extracted_files.append(
                            {
                                "path": str(rel_path),
                                "size": file_size,
                                "name": item.name,
                            }
                        )

            return {
                "status": "success",
                "extraction_folder": str(extraction_folder.relative_to(root_directory)),
                "extracted_files": extracted_files,
                "total_files": len(extracted_files),
            }

        except Exception as e:
            return {"error": f"Failed to extract {filename}: {str(e)}"}

    async def read_notes() -> str:
        """Reads all notes from the _NOTES.md file for the current project."""
        async with notes_lock:
            try:
                if not notes_path.exists():
                    return "No notes found."
                async with aiofiles.open(notes_path, mode="r") as f:
                    content = await f.read()
                    return content if content else "No notes found."
            except Exception as e:
                return f"Error reading notes from file: {str(e)}"

    async def append_notes(markdown_content: str) -> str:
        """Appends a markdown string to the _NOTES.md file for the current project."""
        async with notes_lock:
            try:
                async with aiofiles.open(notes_path, mode="a") as f:
                    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
                    await f.write(
                        f"# Note entry on [{timestamp}]\n{markdown_content}\n\n"
                    )
                return "Note appended successfully."
            except Exception as e:
                return f"Error appending note to file: {str(e)}"

    async def produce_crop(
        filename: str,
        page: int,
        y_min: int,
        x_min: int,
        y_max: int,
        x_max: int,
        tool_context: ToolContext,
    ) -> dict:
        """
        Crops a region from a supported file, saves it as crop.png in the root directory.
        Coordinates are given as integers from 0 to 1000 and are scaled to page dimensions.
        Supported formats: .pdf, .pptx, .docx, .png, .jpg, .jpeg.
        """
        file_path = root_directory / filename
        if not file_path.is_file():
            return {"error": f"File not found: {filename}"}

        artifact_name = f"{file_path.stem}_crop.png"
        output_path = root_directory / artifact_name
        file_extension = file_path.suffix.lower()
        doc = None

        try:
            if file_extension in [".pdf", ".png", ".jpg", ".jpeg"]:
                doc = fitz.open(file_path)
            elif file_extension in [".docx", ".pptx"]:
                pdf_bytes = await asyncio.to_thread(_convert_office_to_pdf, file_path)
                doc = fitz.open(stream=pdf_bytes, filetype="pdf")
            elif file_extension == ".xlsx":
                return {"error": "Cropping .xlsx files is not supported."}
            else:
                return {
                    "error": f"Unsupported file type for cropping: {file_extension}"
                }

            if not doc or doc.page_count < page:
                page_count = doc.page_count if doc else 0
                return {
                    "error": f"Invalid page number: {page}. File has {page_count} pages."
                }

            pdf_page = doc.load_page(page - 1)
            page_rect = pdf_page.rect

            scaled_x_min = (x_min / 1000.0) * page_rect.width
            scaled_y_min = (y_min / 1000.0) * page_rect.height
            scaled_x_max = (x_max / 1000.0) * page_rect.width
            scaled_y_max = (y_max / 1000.0) * page_rect.height

            clip = fitz.Rect(scaled_x_min, scaled_y_min, scaled_x_max, scaled_y_max)
            pix = pdf_page.get_pixmap(clip=clip, dpi=300)
            image_bytes = pix.tobytes("png")

            # Create a genai Part
            from google.genai import types

            part = types.Part.from_bytes(data=image_bytes, mime_type="image/png")
            version = await tool_context.save_artifact(
                filename=artifact_name, artifact=part
            )

            return {"artifact": artifact_name, "version": version}

        except Exception as e:
            return {"error": f"An error occurred: {str(e)}"}
        finally:
            if doc:
                doc.close()

    tools = [
        map_query_tool,
        map_query_tool_regex,
        find,
        ls,
        extract,
        read_notes,
        append_notes,
        produce_crop,
    ]

    return tools


MQ_TOOLSET_PROMPT = """
# Tools by function

Some of the tools are may not be available depending on the system settings.

**Directory & File Listing:**
 - 'find': Costs 1 Credit. Locates files by regex, returning up to 'list_max_files' matches. If more files match, it returns an error and the total count, prompting you to refine the regex. Excellent for targeted searches when you expect a manageable number of results. Use 'ls' to confirm existence or explore a directory before crafting a regex.
 - 'ls': Costs 1 Credit. Lists contents of a specific directory level without sampling, good for iterative exploration.

**File Querying & Summarization:**
 - 'map_query_tool': Costs 10 Credits per 10 documents. Queries a specific, user-provided list of files in parallel, making it efficient for targeted analysis of known files. Expects to be given the query and a broader_context. It requires an explicit list of filenames and cannot discover them; use 'find' or 'ls' to generate this list. Works best for simple fact-finding queries.
 - 'map_query_tool_regex': Costs 10 Credits per 10 documents. Runs a query against files whose filenames match a given regex. If the number of matching files exceeds 'max_map_results', it returns an error prompting you to use a more specific regex. Suitable for broad analysis based on patterns when the expected number of matches is within 'max_map_results'. Always use 'find' first to gauge the number of matches if unsure.

**File Content Operations:**
 - 'extract': Costs 5 Credits per 1 documents. Extracts content from email files (.msg, .eml) and zip archives (.zip) into a new '<original_filename>.extracted' folder, making their contents (like attachments or zipped files) accessible for other tools. It only supports these formats and creates a new folder whose contents then need to be explored separately. After extraction, use 'ls' on the '.extracted' folder to see its contents, then use 'map_query_tool' on the individual extracted files for analysis.
 - 'produce_crop': Costs 5 Credits. Sends an image crop from a document to the user. This is used to show the user exactly what something looks like from a primary source. Hint: 

**Note Management:**
- 'read_notes': Costs nothing. Reads the content of the _NOTES.md file from the root directory. Use this to recall previous findings or context. Do NOT rely on read_notes for factuality or comprehensiveness. Only treat it as additional pools of direction to explore. This is because notes maybe stale (as the knowledge pool was updated) or that the notes are simplified for current user query. Everything that you answer users MUST be coming from querying primary documents, and NOT solely from your previous notes.
- 'append_notes': Costs nothing. Appends a new markdown entry to the _NOTES.md file. Use this to record significant learnings, complex file structures, interrelations between files, or user instructions for future reference. Each entry is automatically timestamped using markdown H1 (`#`). Rule of thumb: if it takes more than 4 tool calls for your to discover something, it's worth noting down.

# Methods

Start with reading existing notes (`read_notes`), survey broadly (`ls`) and open ended (with `map_query_tool`) before directed examination. Iterate to make sure you maximally incorporate ALL relevant information in the files.

# Budget Policy

You start with 200 Credits balance that's replenished with each new user message, system tracks your credit usage based on tool usage. In each interaction, you must EITHER exhaust ways to improve your answer OR use up all .
"""
