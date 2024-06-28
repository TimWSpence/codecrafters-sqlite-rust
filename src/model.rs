use anyhow::*;
use std::{fs::File, io::Read, path::Path};

pub struct Db {
    file: File,
    pub metadata: Metadata,
}

impl Db {
    pub fn open<P>(path: P) -> Result<Db>
    where
        P: AsRef<Path>,
    {
        let mut file = File::open(path)?;

        let metadata = Metadata::parse(&mut file)?;

        Ok(Db { file, metadata })
    }
}

pub struct Metadata {
    pub page_size: u16,
    pub number_of_tables: u16,
}

impl Metadata {
    fn parse(file: &mut File) -> Result<Metadata> {
        let mut header = [0; 100];
        file.read_exact(&mut header)?;

        let page_size = u16::from_be_bytes([header[16], header[17]]);

        let mut page_header = [0; 8];
        file.read_exact(&mut page_header)?;

        let number_of_tables = u16::from_be_bytes([page_header[3], page_header[4]]);

        Ok(Metadata {
            page_size,
            number_of_tables,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_metadata() {
        let db = Db::open("sample.db").unwrap();
        let metadata = db.metadata;

        assert_eq!(metadata.page_size, 4096);
        assert_eq!(metadata.number_of_tables, 3);
    }
}
