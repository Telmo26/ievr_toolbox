#[derive(Debug)]
pub struct ColumnDescriptor {
    raw: u8,
}

impl ColumnDescriptor {
    const TYPE_MASK: u8 = 0x0F;

    pub fn new(byte: u8) -> Self {
        Self { raw: byte }
    }

    pub fn column_type(&self) -> ColumnType {
        unsafe { std::mem::transmute(self.raw & Self::TYPE_MASK) }
    }

    pub fn flags(&self) -> ColumnFlags {
        ColumnFlags::from_bits_truncate(self.raw & !Self::TYPE_MASK)
    }

    pub fn has_name(&self) -> bool {
        self.flags().contains(ColumnFlags::HAS_NAME)
    }

    pub fn has_default(&self) -> bool {
        self.flags().contains(ColumnFlags::HAS_DEFAULT_VALUE)
    }

    pub fn is_row_storage(&self) -> bool {
        self.flags().contains(ColumnFlags::IS_ROW_STORAGE)
    }

    /// Returns the offset of the string in the string pool.
    /// Assumes `has_name() == true`.
    pub fn string_offset(&self, data: &[u8]) -> u32 {
        // The slice must have at least 5 bytes: [descriptor][u32 offset]
        assert!(data.len() >= 5, "Column data too small for string offset");

        // u32 is big-endian in CPK UTF
        u32::from_be_bytes(data[1..5].try_into().unwrap())
    }

    pub fn value_len(&self) -> u8 {
        match self.column_type() {
            ColumnType::Byte | ColumnType::SByte    => 1,
            ColumnType::UInt16 | ColumnType::Int16  => 2,
            ColumnType::UInt32 | ColumnType::Int32  => 4,
            ColumnType::UInt64 | ColumnType::Int64  => 8,
            ColumnType::Single  => 4,
            ColumnType::Double  => 8,
            ColumnType::String  => 4,
            ColumnType::RawData => 8,
            ColumnType::Guid    => 16,
        }
    }

    pub fn read_number(&self, data: &[u8]) -> i64 {
        match self.column_type() {
            ColumnType::Byte => data[0] as i64,
            ColumnType::SByte => data[0] as i8 as i64,
            ColumnType::UInt16 => u16::from_be_bytes(data[..2].try_into().unwrap()) as i64,
            ColumnType::Int16 => i16::from_be_bytes(data[..2].try_into().unwrap()) as i64,
            ColumnType::UInt32 => u32::from_be_bytes(data[..4].try_into().unwrap()) as i64,
            ColumnType::Int32 => i32::from_be_bytes(data[..4].try_into().unwrap()) as i64,
            ColumnType::UInt64 => u64::from_be_bytes(data[..8].try_into().unwrap()) as i64,
            ColumnType::Int64 => i64::from_be_bytes(data[..8].try_into().unwrap()),
            _ => -1,
        }
    }
}

#[allow(dead_code)]
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ColumnType {
    Byte = 0,
    SByte = 1,
    UInt16 = 2,
    Int16 = 3,
    UInt32 = 4,
    Int32 = 5,
    UInt64 = 6,
    Int64 = 7,
    Single = 8,
    Double = 9,
    String = 10,
    RawData = 11,
    Guid = 12,
}

bitflags::bitflags! {
    pub struct ColumnFlags: u8 {
        const HAS_NAME          = 0x10;
        const HAS_DEFAULT_VALUE = 0x20;
        const IS_ROW_STORAGE    = 0x40;
    }
}
