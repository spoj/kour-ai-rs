import os
import tempfile
import extract_msg
import email
import mimetypes
import re
import shutil
from pathlib import Path
from typing import Optional, Union
from markitdown import MarkItDown

# Constants
SUPPORTED_ENCODINGS = ['utf-8', 'iso-2022-jp', 'cp1252', 'latin1']
MAX_FILENAME_LENGTH = 100
SUPPORTED_EXTENSIONS = ['.msg', '.eml']


def _log_extraction(message: str, indent_level: int = 0):
    """Log extraction progress with consistent formatting."""
    indent = "  " * indent_level
    print(f"{indent}{message}")


def _sanitize_filename(filename: Optional[str]) -> str:
    """Create a safe filename by removing problematic characters."""
    if not filename:
        return "unnamed"
    
    # Replace problematic characters with underscores
    safe_name = re.sub(r'[<>:"/\\|?*\x00-\x1f]', '_', filename)
    # Remove multiple consecutive underscores
    safe_name = re.sub(r'_+', '_', safe_name)
    # Remove leading/trailing underscores and spaces
    safe_name = safe_name.strip('_ ')
    
    # Limit length
    if len(safe_name) > MAX_FILENAME_LENGTH:
        safe_name = safe_name[:MAX_FILENAME_LENGTH]
    
    return safe_name if safe_name else "unnamed"


def _detect_file_extension(data: bytes) -> str:
    """Detect file extension from binary data."""
    if not data or len(data) < 4:
        return ""
    
    data_start = data[:4]
    if data_start.startswith(b'\x89PNG'):
        return ".png"
    elif data_start.startswith(b'\xff\xd8\xff'):
        return ".jpg"
    elif data_start.startswith(b'PK'):
        return ".zip"
    elif data_start.startswith(b'%PDF'):
        return ".pdf"
    elif data_start.startswith(b'GIF8'):
        return ".gif"
    elif data_start.startswith(b'\x00\x00\x01\x00'):
        return ".ico"
    return ""


def _ensure_unique_filename(output_folder: Path, filename: str) -> Path:
    """Ensure filename is unique in the output folder."""
    output_path = output_folder / filename
    counter = 1
    while output_path.exists():
        name_part = Path(filename).stem
        ext_part = Path(filename).suffix
        output_path = output_folder / f"{name_part}_{counter}{ext_part}"
        counter += 1
    return output_path


def _convert_html_to_markdown(html_content: Union[str, bytes], md_instance: Optional[MarkItDown] = None) -> str:
    """Convert HTML content to markdown using markitdown."""
    if md_instance is None:
        md_instance = MarkItDown()
    
    # Handle bytes vs string for HTML content
    if isinstance(html_content, bytes):
        for encoding in SUPPORTED_ENCODINGS:
            try:
                html_content = html_content.decode(encoding)
                break
            except UnicodeDecodeError:
                continue
        else:
            html_content = html_content.decode('utf-8', errors='replace')
    
    try:
        with tempfile.NamedTemporaryFile(mode='w', suffix='.html', delete=False, encoding='utf-8') as tmp:
            tmp.write(html_content)
            tmp_path = tmp.name
        
        result = md_instance.convert(tmp_path)
        os.unlink(tmp_path)
        return result.text_content
    except Exception:
        return f"```html\n{html_content}\n```\n"


def _clean_msg_text(text: Optional[str]) -> Optional[str]:
    """Clean MSG text by removing null characters and extra whitespace."""
    if not text:
        return text
    
    # Remove null characters that are common in MSG format
    cleaned = text.replace('\x00', '')
    # Remove other control characters except newlines and tabs
    cleaned = ''.join(char for char in cleaned if ord(char) >= 32 or char in '\n\t\r')
    # Clean up extra whitespace
    cleaned = cleaned.strip()
    
    return cleaned if cleaned else None


def _generate_email_headers(msg, is_msg_format: bool = True) -> str:
    """Generate markdown headers section for email."""
    headers = "## Headers\n\n"
    
    if is_msg_format:
        # MSG format - clean null characters from strings
        header_map = {
            'From': _clean_msg_text(msg.sender),
            'To': _clean_msg_text(msg.to),
            'Cc': _clean_msg_text(msg.cc),
            'Bcc': _clean_msg_text(msg.bcc),
            'Subject': _clean_msg_text(msg.subject),
            'Date': str(msg.date) if msg.date else None
        }
    else:
        # EML format
        header_map = {
            'From': msg.get('From'),
            'To': msg.get('To'),
            'Cc': msg.get('Cc'),
            'Bcc': msg.get('Bcc'),
            'Subject': msg.get('Subject'),
            'Date': msg.get('Date')
        }
    
    for header_name, value in header_map.items():
        if value:
            headers += f"**{header_name}:** {value}\n\n"
    
    return headers


def _process_attachment_data(attachment_data: bytes, filename: Optional[str], output_folder: Path, attachment_index: int) -> Optional[str]:
    """Process and save attachment data with proper filename handling."""
    safe_filename = _sanitize_filename(filename or f"attachment_{attachment_index}")
    
    if isinstance(attachment_data, bytes):
        if not Path(safe_filename).suffix:
            detected_ext = _detect_file_extension(attachment_data)
            if detected_ext:
                safe_filename += detected_ext
        
        output_path = _ensure_unique_filename(output_folder, safe_filename)
        with open(output_path, 'wb') as f:
            f.write(attachment_data)
        return output_path.name
    return None


def _create_email_body_md(email_path: Path, output_folder: Path, email_format: str):
    """Create EMAIL.md file with markdown representation of the email."""
    md = MarkItDown()
    
    try:
        if email_format == 'msg':
            # Copy MSG file to temp location and extract from there
            temp_msg_file = None
            try:
                with tempfile.NamedTemporaryFile(suffix='.msg', delete=False) as tmp:
                    temp_msg_file = tmp.name
                    shutil.copy2(email_path, temp_msg_file)
                
                # Use extract-msg library to get detailed content from temp file
                with extract_msg.Message(temp_msg_file) as msg:
                    markdown_content = "# Email Content\n\n"
                    markdown_content += _generate_email_headers(msg, is_msg_format=True)
                    markdown_content += "## Body\n\n"
                    
                    # Try to get HTML body first, then plain text
                    body_content = ""
                    
                    if hasattr(msg, 'htmlBody') and msg.htmlBody:
                        try:
                            body_content = _convert_html_to_markdown(msg.htmlBody, md)
                        except Exception as e:
                            _log_extraction(f"HTML conversion failed, trying plain text: {e}")
                            try:
                                html_content = msg.htmlBody
                                if isinstance(html_content, bytes):
                                    html_content = html_content.decode('utf-8', errors='replace')
                                body_content = f"```html\n{html_content}\n```\n"
                            except:
                                body_content = "*Failed to extract HTML content*\n"
                    
                    elif hasattr(msg, 'body') and msg.body:
                        body_content = _clean_msg_text(msg.body) or "*No body content found*\n"
                    else:
                        body_content = "*No body content found*\n"
                    
                    markdown_content += body_content
            finally:
                # Clean up temp file
                if temp_msg_file and os.path.exists(temp_msg_file):
                    os.unlink(temp_msg_file)
                
        else:  # eml
            # Copy EML file to temp location and extract from there
            temp_eml_file = None
            try:
                with tempfile.NamedTemporaryFile(suffix='.eml', delete=False) as tmp:
                    temp_eml_file = tmp.name
                    shutil.copy2(email_path, temp_eml_file)
                
                # For EML files, extract content manually and format as markdown
                with open(temp_eml_file, 'rb') as f:
                    msg = email.message_from_bytes(f.read())
            finally:
                # Clean up temp file
                if temp_eml_file and os.path.exists(temp_eml_file):
                    os.unlink(temp_eml_file)
            
            markdown_content = "# Email Content\n\n"
            markdown_content += _generate_email_headers(msg, is_msg_format=False)
            markdown_content += "## Body\n\n"
            
            # Extract text and HTML parts
            text_content = ""
            html_content = ""
            
            for part in msg.walk():
                content_type = part.get_content_type()
                if content_type == 'text/plain':
                    payload = part.get_payload(decode=True)
                    if payload:
                        text_content = payload.decode('utf-8', errors='ignore')
                elif content_type == 'text/html':
                    payload = part.get_payload(decode=True)
                    if payload:
                        html_content = payload.decode('utf-8', errors='ignore')
            
            # Prefer HTML content, fall back to text
            if html_content:
                try:
                    markdown_content += _convert_html_to_markdown(html_content, md)
                except Exception:
                    markdown_content += f"```html\n{html_content}\n```\n"
            elif text_content:
                markdown_content += text_content
            else:
                markdown_content += "*No body content found*\n"
        
        # Write EMAIL.md file
        body_file = output_folder / "EMAIL.md"
        with open(body_file, 'w', encoding='utf-8') as f:
            f.write(markdown_content)
        
        _log_extraction("Created email content: EMAIL.md")
        return True
        
    except Exception as e:
        _log_extraction(f"Failed to create EMAIL.md: {e}")
        return False


def _extract_embedded_msg(embedded_msg, output_folder: Path, base_name: str):
    """Extract content from an embedded MSG file."""
    clean_subject = _clean_msg_text(embedded_msg.subject) or "Unknown Subject"
    _log_extraction(f"Extracting embedded MSG: {clean_subject}", 1)
    
    # Create subfolder for this embedded message
    safe_folder_name = _sanitize_filename(base_name)
    msg_folder = output_folder / f"{safe_folder_name}_embedded"
    msg_folder.mkdir(exist_ok=True)
    
    # Create EMAIL.md for the embedded message
    body_content = "# Embedded Email Content\n\n"
    body_content += _generate_email_headers(embedded_msg, is_msg_format=True)
    body_content += "## Body\n\n"
    
    # Try to get HTML body first, then plain text
    body_added = False
    
    if hasattr(embedded_msg, 'htmlBody') and embedded_msg.htmlBody:
        try:
            body_content += _convert_html_to_markdown(embedded_msg.htmlBody)
            body_added = True
        except Exception as e:
            _log_extraction(f"HTML conversion failed, trying plain text: {e}", 2)
            try:
                html_content = embedded_msg.htmlBody
                if isinstance(html_content, bytes):
                    html_content = html_content.decode('utf-8', errors='replace')
                body_content += f"```html\n{html_content}\n```\n"
                body_added = True
            except:
                pass
    
    if not body_added and hasattr(embedded_msg, 'body') and embedded_msg.body:
        clean_body = _clean_msg_text(embedded_msg.body)
        if clean_body:
            body_content += clean_body
            body_added = True
    
    if not body_added:
        body_content += "*No body content found*"
    
    body_file = msg_folder / "EMAIL.md"
    with open(body_file, 'w', encoding='utf-8') as f:
        f.write(body_content)
    _log_extraction("Created: EMAIL.md", 2)
    
    # Extract attachments from the embedded message
    extracted_count = 0
    if hasattr(embedded_msg, 'attachments') and embedded_msg.attachments:
        _log_extraction(f"Extracting {len(embedded_msg.attachments)} attachment(s) from embedded message", 2)
        for i, att in enumerate(embedded_msg.attachments):
            att_name = att.longFilename or att.shortFilename or f"attachment_{i+1}"
            
            if hasattr(att, 'data') and att.data:
                if isinstance(att.data, bytes):
                    # Regular file attachment
                    saved_name = _process_attachment_data(att.data, att_name, msg_folder, i+1)
                    if saved_name:
                        _log_extraction(f"Saved: {saved_name}", 3)
                        extracted_count += 1
                elif hasattr(att.data, 'subject'):
                    # Nested embedded MSG file
                    _log_extraction(f"Found nested embedded MSG: {att_name}", 3)
                    _extract_embedded_msg(att.data, msg_folder, att_name)
                    extracted_count += 1
                else:
                    _log_extraction(f"Skipping unknown attachment type: {att_name} (type: {type(att.data)})", 3)
    
    return extracted_count


def _extract_embedded_rfc822(embedded_msg, output_folder: Path, base_name: str):
    """Extract content from an embedded RFC822 message."""
    _log_extraction(f"Extracting embedded RFC822: {embedded_msg.get('Subject')}", 1)
    
    # Create subfolder for this embedded message
    safe_folder_name = _sanitize_filename(base_name or "embedded_message")
    msg_folder = output_folder / f"{safe_folder_name}_embedded"
    msg_folder.mkdir(exist_ok=True)
    
    # Create EMAIL.md for the embedded message
    body_content = "# Embedded Email Content\n\n"
    body_content += _generate_email_headers(embedded_msg, is_msg_format=False)
    body_content += "## Body\n\n"
    
    # Extract text and HTML parts from embedded message
    text_content = ""
    html_content = ""
    
    for part in embedded_msg.walk():
        content_type = part.get_content_type()
        if content_type == 'text/plain':
            payload = part.get_payload(decode=True)
            if payload:
                text_content = payload.decode('utf-8', errors='ignore')
        elif content_type == 'text/html':
            payload = part.get_payload(decode=True)
            if payload:
                html_content = payload.decode('utf-8', errors='ignore')
    
    # Prefer HTML content, fall back to text
    if html_content:
        try:
            body_content += _convert_html_to_markdown(html_content)
        except Exception as e:
            _log_extraction(f"HTML conversion failed, using raw HTML: {e}", 2)
            body_content += f"```html\n{html_content}\n```\n"
    elif text_content:
        body_content += text_content
    else:
        body_content += "*No body content found*"
    
    body_file = msg_folder / "EMAIL.md"
    with open(body_file, 'w', encoding='utf-8') as f:
        f.write(body_content)
    _log_extraction("Created: EMAIL.md", 2)
    
    # Extract attachments and inline content from the embedded message
    extracted_count = 0
    inline_count = 0
    
    for i, part in enumerate(embedded_msg.walk()):
        content_disposition = part.get_content_disposition()
        content_type = part.get_content_type()
        filename = part.get_filename()
        
        # Check if this is an attachment or inline content we want to extract
        is_attachment = content_disposition == 'attachment'
        is_inline_image = (content_disposition == 'inline' and
                          content_type and content_type.startswith('image/'))
        is_embedded_image = (content_type and content_type.startswith('image/') and
                           part.get('Content-ID'))
        
        if is_attachment or is_inline_image or is_embedded_image:
            if not filename:
                # Generate filename based on content type
                ext = mimetypes.guess_extension(content_type) or ""
                if is_attachment:
                    filename = f"attachment_{extracted_count + 1}{ext}"
                else:
                    filename = f"inline_image_{inline_count + 1}{ext}"
            
            # Get attachment/image data
            payload = part.get_payload(decode=True)
            if payload:
                saved_name = _process_attachment_data(payload, filename, msg_folder, extracted_count + 1)
                if saved_name:
                    if is_attachment:
                        _log_extraction(f"Saved attachment: {saved_name}", 3)
                        extracted_count += 1
                    else:
                        _log_extraction(f"Saved inline image: {saved_name}", 3)
                        inline_count += 1
    
    total_extracted = extracted_count + inline_count
    if total_extracted > 0:
        _log_extraction(f"Extracted {extracted_count} attachment(s) and {inline_count} inline image(s)", 2)
    
    return total_extracted


def _extract_eml_attachments(eml_path: Path, output_folder: Path):
    """Extract attachments and inline images from EML file."""
    temp_eml_file = None
    try:
        # Copy EML file to temp location
        with tempfile.NamedTemporaryFile(suffix='.eml', delete=False) as tmp:
            temp_eml_file = tmp.name
            shutil.copy2(eml_path, temp_eml_file)
        
        # Extract from temp file
        with open(temp_eml_file, 'rb') as f:
            msg = email.message_from_bytes(f.read())
        
        attachment_count = 0
        inline_count = 0
        
        for part in msg.walk():
            content_disposition = part.get_content_disposition()
            content_type = part.get_content_type()
            
            # Check if this is an attachment or inline content we want to extract
            is_attachment = content_disposition == 'attachment'
            is_inline_image = (content_disposition == 'inline' and
                              content_type and content_type.startswith('image/'))
            is_embedded_image = (content_type and content_type.startswith('image/') and
                               part.get('Content-ID'))
            
            # Handle embedded RFC822 messages first
            if content_type == 'message/rfc822' and content_disposition == 'attachment':
                _log_extraction("Found embedded RFC822 message")
                try:
                    embedded_payload = part.get_payload()
                    if embedded_payload and isinstance(embedded_payload, list):
                        for embedded_msg in embedded_payload:
                            if hasattr(embedded_msg, 'get'):
                                base_name = embedded_msg.get('Subject') or f"embedded_message_{attachment_count + 1}"
                                _extract_embedded_rfc822(embedded_msg, output_folder, base_name)
                                attachment_count += 1
                    elif embedded_payload and hasattr(embedded_payload, 'get'):
                        # Single embedded message
                        base_name = embedded_payload.get('Subject') or f"embedded_message_{attachment_count + 1}"
                        _extract_embedded_rfc822(embedded_payload, output_folder, base_name)
                        attachment_count += 1
                except Exception as e:
                    _log_extraction(f"Failed to extract embedded RFC822 message: {e}")
            
            elif is_attachment or is_inline_image or is_embedded_image:
                filename = part.get_filename()
                
                if not filename:
                    # Try to get filename from content-type
                    ext = mimetypes.guess_extension(content_type) or ""
                    
                    if is_attachment:
                        filename = f"attachment_{attachment_count + 1}{ext}"
                    else:
                        # For inline images, use a descriptive name
                        content_id = part.get('Content-ID', '').strip('<>')
                        if content_id:
                            # Clean up Content-ID for filename
                            clean_id = content_id.replace('@', '_at_').replace('.', '_')
                            filename = f"inline_{clean_id}{ext}"
                        else:
                            filename = f"inline_image_{inline_count + 1}{ext}"
                
                # Get attachment/image data
                payload = part.get_payload(decode=True)
                if payload:
                    # If still no extension, try to detect from data
                    if not Path(filename).suffix:
                        detected_ext = _detect_file_extension(payload)
                        if detected_ext:
                            filename += detected_ext
                    
                    safe_filename = _sanitize_filename(filename)
                    output_path = _ensure_unique_filename(output_folder, safe_filename)
                    
                    try:
                        with open(output_path, 'wb') as f:
                            f.write(payload)
                        
                        if is_attachment:
                            _log_extraction(f"Extracted attachment: {output_path.name}")
                            attachment_count += 1
                        else:
                            _log_extraction(f"Extracted inline image: {output_path.name}")
                            inline_count += 1
                            
                    except Exception as e:
                        _log_extraction(f"Failed to extract {filename}: {e}")
        
        total_count = attachment_count + inline_count
        if inline_count > 0:
            _log_extraction(f"Extracted {attachment_count} attachment(s) and {inline_count} inline image(s)")
        
        return total_count
        
    except Exception as e:
        _log_extraction(f"Failed to process EML file {eml_path}: {e}")
        return 0
    finally:
        # Clean up temp file
        if temp_eml_file and os.path.exists(temp_eml_file):
            os.unlink(temp_eml_file)


def _extract_msg_attachments(msg_path: Path, output_folder: Path):
    """Extract attachments from MSG file."""
    temp_msg_file = None
    try:
        # Copy MSG file to temp location
        with tempfile.NamedTemporaryFile(suffix='.msg', delete=False) as tmp:
            temp_msg_file = tmp.name
            shutil.copy2(msg_path, temp_msg_file)
        
        # Extract from temp file
        with extract_msg.Message(temp_msg_file) as msg:
            attachments = msg.attachments
            
            attachment_count = len(attachments) if attachments else 0
            
            if attachments:
                for i, attachment in enumerate(attachments):
                    filename = attachment.longFilename or attachment.shortFilename
                    if not filename:
                        ext = ""
                        if hasattr(attachment, 'data') and attachment.data:
                            ext = _detect_file_extension(attachment.data)
                        filename = f"attachment_{i+1}{ext}"
                    
                    output_path = _ensure_unique_filename(output_folder, filename)
                    
                    # Check if this is an embedded MSG file or regular attachment
                    if hasattr(attachment, 'data') and attachment.data:
                        if hasattr(attachment.data, 'subject'):
                            # This is an embedded MSG file
                            _log_extraction(f"Found embedded MSG: {filename}")
                            _extract_embedded_msg(attachment.data, output_folder, filename)
                        elif isinstance(attachment.data, bytes):
                            # Regular file attachment - try normal save first
                            try:
                                attachment.save(customPath=str(output_folder), customFilename=output_path.name)
                                _log_extraction(f"Extracted: {output_path.name}")
                            except Exception as e:
                                # Fallback to manual save with sanitized filename
                                _log_extraction(f"Normal extraction failed for {filename}: {e}")
                                safe_filename = _sanitize_filename(filename)
                                original_suffix = Path(filename).suffix
                                if original_suffix and not safe_filename.endswith(original_suffix):
                                    safe_filename += original_suffix
                                safe_output_path = _ensure_unique_filename(output_folder, safe_filename)
                                
                                try:
                                    with open(safe_output_path, 'wb') as file_out:
                                        file_out.write(attachment.data)
                                    _log_extraction(f"Saved with sanitized filename: {safe_output_path.name}")
                                except Exception as fallback_error:
                                    _log_extraction(f"Failed to save {filename}: {fallback_error}")
                        else:
                            _log_extraction(f"Unknown attachment data type for {filename}: {type(attachment.data)}")
                    else:
                        # Try normal extraction for attachments without data attribute
                        try:
                            attachment.save(customPath=str(output_folder), customFilename=output_path.name)
                            _log_extraction(f"Extracted: {output_path.name}")
                        except Exception as e:
                            _log_extraction(f"Failed to extract {filename}: {e}")
            
            return attachment_count
            
    except Exception as e:
        _log_extraction(f"Failed to open MSG file {msg_path}: {e}")
        return 0
    finally:
        # Clean up temp file
        if temp_msg_file and os.path.exists(temp_msg_file):
            os.unlink(temp_msg_file)


def extract_email_attachments(email_path: Union[str, Path]) -> str:
    """
    Extract all attachments from an email file (MSG or EML) to a folder with the same name + '_files' suffix.
    
    Args:
        email_path: Path to the email file (e.g., 'path/to/email.msg' or 'path/to/email.eml')
    
    Returns:
        str: Path to the created folder containing extracted attachments
    """
    email_path = Path(email_path)
    
    if not email_path.exists():
        raise FileNotFoundError(f"Email file not found: {email_path}")
    
    file_ext = email_path.suffix.lower()
    if file_ext not in SUPPORTED_EXTENSIONS:
        raise ValueError(f"File must have {' or '.join(SUPPORTED_EXTENSIONS)} extension: {email_path}")
    
    # Create output folder: same name as email file + '.extracted'
    output_folder = email_path.parent / f"{email_path.name}.extracted"
    output_folder.mkdir(exist_ok=True)
    
    # Extract attachments based on file type
    if file_ext == '.msg':
        attachment_count = _extract_msg_attachments(email_path, output_folder)
    else:  # .eml
        attachment_count = _extract_eml_attachments(email_path, output_folder)
    
    # Create EMAIL.md file
    _create_email_body_md(email_path, output_folder, file_ext[1:])
    
    if attachment_count == 0:
        _log_extraction(f"No attachments found in {email_path}")
    else:
        _log_extraction(f"Found {attachment_count} attachment(s) in {email_path}")
    
    _log_extraction(f"Content extracted to: {output_folder}")
    return str(output_folder)


def extract_msg_attachments(msg_path: Union[str, Path]) -> str:
    """
    Legacy function for MSG files only. Use extract_email_attachments() for both MSG and EML support.
    """
    return extract_email_attachments(msg_path)


if __name__ == "__main__":
    import sys
    
    if len(sys.argv) != 2:
        print("Usage: python email_extract.py <path_to_email_file>")
        print("Supports both .msg and .eml files")
        sys.exit(1)
    
    email_file = sys.argv[1]
    try:
        extract_email_attachments(email_file)
    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)