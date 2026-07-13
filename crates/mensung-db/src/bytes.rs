//! Bounds-checked little-endian integer reads from a byte slice at a given
//! offset, per the endianness rule in docs/DATABASE_FORMAT.md. These never
//! panic on a short buffer; they return DbError::Truncated instead, because
//! the buffer being too short is exactly the kind of corruption this reader
//! exists to catch.

use crate::DbError;

pub(crate) fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, DbError> {
    let slice = bytes.get(offset..offset + 2).ok_or(DbError::Truncated)?;
    Ok(u16::from_le_bytes(
        slice.try_into().expect("slice of exactly 2 bytes"),
    ))
}

pub(crate) fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, DbError> {
    let slice = bytes.get(offset..offset + 4).ok_or(DbError::Truncated)?;
    Ok(u32::from_le_bytes(
        slice.try_into().expect("slice of exactly 4 bytes"),
    ))
}

pub(crate) fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, DbError> {
    let slice = bytes.get(offset..offset + 8).ok_or(DbError::Truncated)?;
    Ok(u64::from_le_bytes(
        slice.try_into().expect("slice of exactly 8 bytes"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_little_endian_values() {
        assert_eq!(read_u16(&[0x01, 0x02], 0).unwrap(), 0x0201);
        assert_eq!(read_u32(&[0x01, 0x02, 0x03, 0x04], 0).unwrap(), 0x0403_0201);
        assert_eq!(
            read_u64(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08], 0).unwrap(),
            0x0807_0605_0403_0201
        );
    }

    #[test]
    fn reads_at_a_nonzero_offset() {
        assert_eq!(read_u16(&[0xff, 0x01, 0x02], 1).unwrap(), 0x0201);
    }

    #[test]
    fn rejects_a_buffer_too_short_for_the_requested_width() {
        assert_eq!(read_u16(&[0x01], 0).unwrap_err(), DbError::Truncated);
        assert_eq!(
            read_u32(&[0x01, 0x02, 0x03], 0).unwrap_err(),
            DbError::Truncated
        );
        assert_eq!(read_u64(&[0x01; 7], 0).unwrap_err(), DbError::Truncated);
    }

    #[test]
    fn rejects_an_offset_past_the_end_of_the_buffer() {
        assert_eq!(read_u16(&[0x01, 0x02], 2).unwrap_err(), DbError::Truncated);
    }
}
