use std::fs;
use std::io::Write;
use std::path::Path;

use regex::Regex;
use sanitize_filename::sanitize;
use tiny_msg::Email;

use crate::Result;

fn format_addresses(addresses: &[(String, String)]) -> String {
    addresses
        .iter()
        .map(|(name, address)| {
            if name.is_empty() || name == address {
                address.to_string()
            } else {
                format!("{name} <{address}>")
            }
        })
        .collect::<Vec<String>>()
        .join(", ")
}

fn process_email(email: &Email, output_dir: &Path) -> Result<()> {
    fs::create_dir_all(output_dir)?;

    let md_path = output_dir.join("EMAIL.md");
    let mut file = fs::File::create(md_path).expect("Failed to create file");

    if let Some(from) = &email.from {
        writeln!(file, "From: {}", format_addresses(&[from.clone()]))?;
    }
    if let Some(date) = email.sent_date {
        writeln!(file, "Sent: {date}")?;
    }
    if !email.to.is_empty() {
        writeln!(file, "To: {}", format_addresses(&email.to))?;
    }
    if !email.cc.is_empty() {
        writeln!(file, "CC: {}", format_addresses(&email.cc))?;
    }
    if !email.bcc.is_empty() {
        writeln!(file, "BCC: {}", format_addresses(&email.bcc))?;
    }
    if let Some(subject) = &email.subject {
        writeln!(file, "Subject: {subject}")?;
    }

    writeln!(file, "\n---")?;

    if let Some(body) = &email.body {
        let re = Regex::new(r"(?s)<!--.*?-->").unwrap();
        let cleaned_html = re.replace_all(body, "");
        let markdown = html2md::parse_html(&cleaned_html);
        writeln!(file, "{markdown}")?;
    }

    for attachment in &email.attachments {
        let file_path = output_dir.join(sanitize(&attachment.name));
        fs::write(file_path, &attachment.data).expect("Failed to write attachment");
    }

    for embedded_message in &email.embedded_messages {
        let subject = embedded_message
            .subject
            .as_deref()
            .unwrap_or("embedded_email");
        let dir_name = sanitize(subject);
        let new_dir = output_dir.join(dir_name);
        let _ = process_email(embedded_message, &new_dir);
    }

    Ok(())
}

pub fn extract_msg(file_path: &Path, output_dir: &Path) -> Result<()> {
    let email = Email::from_path(file_path);
    process_email(&email, output_dir)
}
