import fs from 'fs';
import path from 'path';
import EmlParser from "eml-parser";
import TurndownService from "turndown";

function _sanitizeFilename(filename) {
  return filename.replace(/[<>:"/\\|?*]/g, '_');
}

async function _extractRecursive(fileStream, outputDir) {
  const parser = new EmlParser(fileStream);
  const fileExtension = path.extname(fileStream.path).toLowerCase();
  const result = fileExtension === '.eml' ? await parser.parseEml() : await parser.parseMsg();

  // Save body
  let markdownContent = '';

  // Add headers to markdown
  if (result.dataType === 'msg') { // MSG specific headers
    if (result.senderName) markdownContent += `**From:** ${result.senderName} <${result.senderSmtpAddress}>\n\n`;
    if (result.recipients) {
        const to = result.recipients.filter(r => r.recipType === 'to').map(r => `${r.name} <${r.smtpAddress}>`).join(', ');
        if (to) markdownContent += `**To:** ${to}\n\n`;
        const cc = result.recipients.filter(r => r.recipType === 'cc').map(r => `${r.name} <${r.smtpAddress}>`).join(', ');
        if (cc) markdownContent += `**Cc:** ${cc}\n\n`;
    }
    if (result.creationTime) markdownContent += `**Sent:** ${result.creationTime}\n\n`;
    if (result.subject) markdownContent += `**Subject:** ${result.subject}\n\n`;
  } else { // EML specific headers
      if (result.from && result.from.text) markdownContent += `**From:** ${result.from.text}\n\n`;
      if (result.to && result.to.text) markdownContent += `**To:** ${result.to.text}\n\n`;
      if (result.cc && result.cc.text) markdownContent += `**Cc:** ${result.cc.text}\n\n`;
      if (result.date) markdownContent += `**Sent:** ${result.date}\n\n`;
      if (result.subject) markdownContent += `**Subject:** ${result.subject}\n\n`;
  }

  markdownContent += '---\n\n'; // Separator

  if (result.html) {
    const turndownService = new TurndownService();
    markdownContent += turndownService.turndown(result.html);
    fs.writeFileSync(path.join(outputDir, 'EMAIL.md'), markdownContent);
  } else if (result.text) {
    fs.writeFileSync(path.join(outputDir, 'EMAIL.txt'), result.text);
  }
  
  // EML file handling
  if (result.dataType === 'msg') {
    if (result.attachments) {
      for (const attachment of result.attachments) {
        let fileName = attachment.name;
        if (!fileName) continue;
        
        fileName = _sanitizeFilename(fileName);
        const attachmentPath = path.join(outputDir, fileName);

        if (attachment.innerMsgContent && attachment.content) {
          const subDir = attachmentPath + '.extracted';
          if (!fs.existsSync(subDir)) {
            fs.mkdirSync(subDir);
          }
          
          const tempFilePath = path.join(outputDir, `temp_${fileName}`);
          fs.writeFileSync(tempFilePath, attachment.content);
          const tempFileStream = fs.createReadStream(tempFilePath);
          await _extractRecursive(tempFileStream, subDir);
          fs.unlinkSync(tempFilePath);

        } else if (attachment.content) {
          fs.writeFileSync(attachmentPath, attachment.content);
        }
      }
    }
  // EML file handling
  } else {
    let embeddedEmailCounter = 1;
    if (result.attachments) {
      for (const attachment of result.attachments) {
        let fileName = attachment.filename || attachment.name;
        
        if (attachment.contentType === 'message/rfc822' && attachment.content) {
          // Parse embedded email to get subject for folder name
          const tempEmbeddedPath = path.join(outputDir, `temp_embedded_${embeddedEmailCounter++}.eml`);
          fs.writeFileSync(tempEmbeddedPath, attachment.content);
          const embeddedStream = fs.createReadStream(tempEmbeddedPath);
          const embeddedParser = new EmlParser(embeddedStream);
          const embeddedResult = await embeddedParser.parseEml();
          fs.unlinkSync(tempEmbeddedPath);

          if (embeddedResult.subject) {
            fileName = embeddedResult.subject + '.eml';
          } else {
            fileName = `embedded_email_${embeddedEmailCounter -1}.eml`;
          }
        }
        
        if (!fileName) continue;

        fileName = _sanitizeFilename(fileName);
        const attachmentPath = path.join(outputDir, fileName);

        if (attachment.contentType === 'message/rfc822' && attachment.content) {
          const subDir = attachmentPath + '.extracted';
          if (!fs.existsSync(subDir)) {
            fs.mkdirSync(subDir);
          }
          
          const tempFilePath = path.join(outputDir, `temp_${fileName}`);
          fs.writeFileSync(tempFilePath, attachment.content);
          const tempFileStream = fs.createReadStream(tempFilePath);
          await _extractRecursive(tempFileStream, subDir);
          fs.unlinkSync(tempFilePath); 

        } else if (attachment.content) {
          fs.writeFileSync(attachmentPath, attachment.content);
        }
      }
    }
  }
}


export async function extractEmail(filePath, outputDir) {
    const fileStream = fs.createReadStream(filePath);
    await _extractRecursive(fileStream, outputDir);
}