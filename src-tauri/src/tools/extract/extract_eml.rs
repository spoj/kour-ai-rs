use std::fs;
use std::io::Write;
use std::path::Path;

use mail_parser::{Address, Message, MessageParser, MimeHeaders};
use regex::Regex;
use sanitize_filename::sanitize;

fn format_addresses(address: &Address) -> String {
    address
        .iter()
        .map(|addr| {
            let name = addr.name().unwrap_or("");
            let address = addr.address().unwrap_or("");
            if name.is_empty() {
                address.to_string()
            } else {
                format!("{} <{}>", name, address)
            }
        })
        .collect::<Vec<String>>()
        .join(", ")
}

pub fn extract_eml(message: &Message, output_dir: &Path) {
    fs::create_dir_all(output_dir).expect("Failed to create directory");

    let md_path = output_dir.join("EMAIL.md");
    let mut file = fs::File::create(md_path).expect("Failed to create file");

    if let Some(from) = message.from() {
        writeln!(file, "From: {}", format_addresses(from)).unwrap();
    }
    if let Some(date) = message.date() {
        writeln!(file, "Sent: {}", date).unwrap();
    }
    if let Some(to) = message.to() {
        writeln!(file, "To: {}", format_addresses(to)).unwrap();
    }
    if let Some(cc) = message.cc() {
        writeln!(file, "CC: {}", format_addresses(cc)).unwrap();
    }
    if let Some(subject) = message.subject() {
        writeln!(file, "Subject: {}", subject).unwrap();
    }

    writeln!(file, "\n---").unwrap();

    if let Some(html_body) = message.body_html(0) {
        let re = Regex::new(r"(?s)<!--.*?-->").unwrap();
        let cleaned_html = re.replace_all(&html_body, "");
        let markdown = html2md::parse_html(&cleaned_html);
        writeln!(file, "{}", markdown).unwrap();
    } else if let Some(text_body) = message.body_text(0) {
        writeln!(file, "{}", text_body).unwrap();
    }

    let parser = MessageParser::default();
    for attachment in message.attachments() {
        if attachment.is_message() {
            if let Some(embedded_message) = parser.parse(attachment.contents()) {
                let subject = embedded_message.subject().unwrap_or("embedded_email");
                let dir_name = sanitize(subject);
                let new_dir = output_dir.join(dir_name);
                extract_eml(&embedded_message, &new_dir);
            }
        } else if let Some(filename) = attachment.attachment_name() {
            let file_path = output_dir.join(filename);
            fs::write(file_path, attachment.contents()).expect("Failed to write attachment");
        }
    }
}