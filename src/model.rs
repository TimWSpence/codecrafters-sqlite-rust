use anyhow::*;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom::Start},
    path::Path,
};

pub struct Db {
    file: File,
    pub metadata: TableMetadata,
}

impl Db {
    pub fn open<P>(path: P) -> Result<Db>
    where
        P: AsRef<Path>,
    {
        let mut file = File::open(path)?;

        let metadata = TableMetadata::parse(&mut file)?;

        Ok(Db { file, metadata })
    }

    pub fn tables(&mut self) -> Result<Vec<&str>> {
        use BTree::*;
        let btree = self.btree(0)?;

        match btree {
            InteriorIndex { raw: _, header: _ } => Err(anyhow!("First page must be a table btree")),
            InteriorTable { raw: _, header: _ } => {
                Err(anyhow!("Interior tables not implemented yet"))
            }
            LeafIndex { raw: _, header: _ } => Err(anyhow!("First page must be a table btree")),
            LeafTable { raw, header } => todo!(),
        }?;

        Ok(vec![])
    }

    fn btree(&mut self, idx: u16) -> Result<BTree> {
        self.file
            .seek(Start((idx * self.metadata.page_size).into()))?;
        let btree = BTree::parse(self.metadata.page_size, &mut self.file)?;
        Ok(btree)
    }
}

pub struct TableMetadata {
    pub page_size: u16,
    pub number_of_tables: u16,
}

impl TableMetadata {
    fn parse(file: &mut File) -> Result<TableMetadata> {
        let mut header = [0; 100];
        file.read_exact(&mut header)?;

        let page_size = u16::from_be_bytes([header[16], header[17]]);

        let mut page_header = [0; 8];
        file.read_exact(&mut page_header)?;

        let number_of_tables = u16::from_be_bytes([page_header[3], page_header[4]]);

        Ok(TableMetadata {
            page_size,
            number_of_tables,
        })
    }
}

enum BTree {
    InteriorIndex { raw: Vec<u8>, header: BTreeHeader },
    InteriorTable { raw: Vec<u8>, header: BTreeHeader },
    LeafIndex { raw: Vec<u8>, header: BTreeHeader },
    LeafTable { raw: Vec<u8>, header: BTreeHeader },
}

struct BTreeHeader {
    number_of_cells: u16,
    content_area_offset: u16,
}

impl BTreeHeader {
    fn parse(bytes: &[u8]) -> BTreeHeader {
        let number_of_cells = u16::from_be_bytes([bytes[3], bytes[4]]);
        let content_area_offset = u16::from_be_bytes([bytes[5], bytes[6]]);

        BTreeHeader {
            number_of_cells,
            content_area_offset,
        }
    }
}

impl BTree {
    fn parse(page_size: u16, file: &mut File) -> Result<BTree> {
        use BTree::*;

        let mut raw = Vec::with_capacity(page_size.into());
        file.read_exact(&mut raw)?;

        let header = BTreeHeader::parse(&raw);

        match raw[0] {
            0x02 => Ok(InteriorIndex { raw, header }),
            0x05 => Ok(InteriorTable { raw, header }),
            0x0a => Ok(LeafIndex { raw, header }),
            0x0d => Ok(LeafTable { raw, header }),
            x => Err(anyhow!("{x} is not a valid btree type")),
        }
    }
}

struct VarInt {
    value: i64,
}

impl VarInt {
    fn read(bytes: &[u8]) -> VarInt {
        let mut value: u64 = 0;
        for i in 0..9 {
            if i == 8 {
                let additional_bits: u64 = bytes[i].into();
                let value = value << 8 | additional_bits;
            } else {
                let additional_bits: u64 = (bytes[i] & !(1 << 7)).into();
                let value = value << 7 | additional_bits;
                if (bytes[i] & (1 << 7)) == 0 {
                    break;
                }
            }
        }
        VarInt {
            value: value as i64,
        }
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
