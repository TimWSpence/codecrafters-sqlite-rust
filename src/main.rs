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
    let mut db = Db::open(&args[1])?;

    match command.as_str() {
        ".dbinfo" => {
            let metadata = db.metadata;
            println!("database page size: {}", metadata.page_size);

            println!("number of tables: {}", metadata.number_of_tables);
        }
        ".tables" => {
            let tables = db.tables()?;
            println!("{}", tables.join(" "));
        }
        _ => bail!("Missing or invalid command passed: {}", command),
    }

    Ok(())
}
