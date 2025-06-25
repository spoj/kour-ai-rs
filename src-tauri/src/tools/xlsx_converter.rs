use calamine::{open_workbook, Reader, Xlsx};
use csv;
use std::env;
use std::error::Error;
use std::fs::File;
use std::path::Path;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <input.xlsx> <output.csv>", args[0]);
        return Ok(());
    }

    let input_path = Path::new(&args[1]);
    let output_path = Path::new(&args[2]);

    let mut workbook: Xlsx<std::io::BufReader<File>> =
        open_workbook(input_path).expect("Cannot open Excel file");

    let mut csv_writer = csv::Writer::from_path(output_path)?;

    if let Some(sheet_name) = workbook.sheet_names().first().cloned() {
        if let Some(Ok(range)) = workbook.worksheet_range(&sheet_name) {
            for row in range.rows() {
                let record: Vec<String> = row.iter().map(|cell| cell.to_string()).collect();
                csv_writer.write_record(&record)?;
            }
        }
    }

    csv_writer.flush()?;
    println!("Conversion from {} to {} complete.", args[1], args[2]);
    Ok(())
}