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

#[derive(Debug, PartialEq, Eq)]
enum BTreeType {
    InteriorIndex = 0x02,
    InteriorTable = 0x05,
    LeafIndex = 0x0a,
    LeafTable = 0x0d,
}

enum BTree {
    InteriorIndex { raw: Vec<u8>, header: BTreeHeader },
    InteriorTable { raw: Vec<u8>, header: BTreeHeader },
    LeafIndex { raw: Vec<u8>, header: BTreeHeader },
    LeafTable { raw: Vec<u8>, header: BTreeHeader },
}

struct BTreeHeader {
    type_: BTreeType,
    number_of_cells: u16,
    content_area_offset: u16,
    rightmost_pointer: Option<u32>,
}

impl BTreeHeader {
    fn parse(bytes: &[u8]) -> BTreeHeader {
        let type_ = match bytes[0] {
            0x02 => BTreeType::InteriorIndex,
            0x05 => BTreeType::InteriorTable,
            0x0a => BTreeType::LeafIndex,
            0x0d => BTreeType::LeafTable,
            x => panic!("{x} is not a valid btree type"),
        };

        let number_of_cells = u16::from_be_bytes([bytes[3], bytes[4]]);
        let content_area_offset = u16::from_be_bytes([bytes[5], bytes[6]]);

        let rightmost_pointer =
            if type_ == BTreeType::InteriorIndex || type_ == BTreeType::InteriorTable {
                Some(u32::from_be_bytes([
                    bytes[8], bytes[9], bytes[10], bytes[11],
                ]))
            } else {
                None
            };

        BTreeHeader {
            type_,
            number_of_cells,
            content_area_offset,
            rightmost_pointer,
        }
    }
}

impl BTree {
    fn parse(page_size: u16, file: &mut File) -> Result<BTree> {
        use BTree::*;

        let mut raw = Vec::with_capacity(page_size.into());
        file.read_exact(&mut raw)?;

        let header = BTreeHeader::parse(&raw);

        match header.type_ {
            BTreeType::InteriorIndex => Ok(InteriorIndex { raw, header }),
            BTreeType::InteriorTable => Ok(InteriorTable { raw, header }),
            BTreeType::LeafIndex => Ok(LeafIndex { raw, header }),
            BTreeType::LeafTable => Ok(LeafTable { raw, header }),
        }
    }
}

mod var_int {
    pub fn read(bytes: &[u8]) -> i64 {
        let mut value: u64 = 0;
        for i in 0..9 {
            if i == 8 {
                let additional_bits: u64 = bytes[i].into();
                value = value << 8 | additional_bits;
            } else {
                let additional_bits: u64 = (bytes[i] & !(1 << 7)).into();
                value = value << 7 | additional_bits;
                if (bytes[i] & (1 << 7)) == 0 {
                    break;
                }
            }
        }
        value as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn var_int() {
        assert_eq!(var_int::read(&[0x01]), 1);
        assert_eq!(var_int::read(&[0x81, 0x01]), 0x81);
        assert_eq!(
            var_int::read(&[0xc0, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x01]),
            {
                let expected: u64 = (1 << 63) | 1;
                expected as i64
            }
        );
    }

    #[test]
    fn parses_metadata() {
        let db = Db::open("sample.db").unwrap();
        let metadata = db.metadata;

        assert_eq!(metadata.page_size, 4096);
        assert_eq!(metadata.number_of_tables, 3);
    }

    #[test]
    fn parses_leaf_index_header() {
        let header = BTreeHeader::parse(&[0x0a, 0, 0, 0x12, 0x34, 0x56, 0x78, 0]);

        assert_eq!(header.type_, BTreeType::LeafIndex);
        assert_eq!(header.number_of_cells, 0x1234);
        assert_eq!(header.content_area_offset, 0x5678);
        assert_eq!(header.rightmost_pointer, None);
    }

    #[test]
    fn parses_leaf_table_header() {
        let header = BTreeHeader::parse(&[0x0d, 0, 0, 0, 1, 0, 0, 0]);

        assert_eq!(header.type_, BTreeType::LeafTable);
        assert_eq!(header.number_of_cells, 1);
        assert_eq!(header.content_area_offset, 0);
        assert_eq!(header.rightmost_pointer, None);
    }

    #[test]
    fn parses_interior_index_header() {
        let header = BTreeHeader::parse(&[
            0x02, 0, 0, 0x12, 0x34, 0x56, 0x78, 0, 0x9a, 0xbc, 0xde, 0xf0,
        ]);

        assert_eq!(header.type_, BTreeType::InteriorIndex);
        assert_eq!(header.number_of_cells, 0x1234);
        assert_eq!(header.content_area_offset, 0x5678);
        assert_eq!(header.rightmost_pointer, Some(0x9abcdef0));
    }

    #[test]
    fn parses_interior_table_header() {
        let header = BTreeHeader::parse(&[
            0x05, 0, 0, 0, 1, 0, 8, 0, 0, 0, 0, 42,
        ]);

        assert_eq!(header.type_, BTreeType::InteriorTable);
        assert_eq!(header.number_of_cells, 1);
        assert_eq!(header.content_area_offset, 8);
        assert_eq!(header.rightmost_pointer, Some(42));
    }

    #[test]
    #[should_panic(expected = "is not a valid btree type")]
    fn rejects_invalid_btree_type() {
        BTreeHeader::parse(&[0xff; 12]);
    }
}
