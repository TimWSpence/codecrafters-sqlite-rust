use anyhow::{bail, Result};

mod model;
use model::*;

fn main() -> Result<()> {
    // Parse arguments
    let args = std::env::args().collect::<Vec<_>>();
    match args.len() {
        0 | 1 => bail!("Missing <database path> and <command>"),
        2 => bail!("Missing <command>"),
        _ => {}
    }

    // Parse command and act accordingly
    let command = &args[2];
    match command.as_str() {
        ".dbinfo" => {
            let db = Db::open(&args[1])?;
            let metadata = db.metadata;

            println!("database page size: {}", metadata.page_size);

            println!("number of tables: {}", metadata.number_of_tables);
        }
        _ => bail!("Missing or invalid command passed: {}", command),
    }

    Ok(())
}
