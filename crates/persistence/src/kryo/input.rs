// Port of com.esotericsoftware.kryo.io.Input (Kryo 4.0.2): the primitive encodings.

use super::KryoError;

pub struct Input<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Input<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Input { bytes, position: 0 }
    }


    pub fn is_at_end(&self) -> bool {
        self.position >= self.bytes.len()
    }

    pub fn error(&self, message: impl Into<String>) -> KryoError {
        KryoError { message: message.into(), position: self.position }
    }

    fn byte(&mut self) -> Result<u8, KryoError> {
        let b = *self.bytes.get(self.position).ok_or_else(|| self.error("unexpected end of data"))?;
        self.position += 1;
        Ok(b)
    }

    fn take<const N: usize>(&mut self) -> Result<[u8; N], KryoError> {
        let end = self.position + N;
        let slice = self.bytes.get(self.position..end).ok_or_else(|| self.error("unexpected end of data"))?;
        self.position = end;
        Ok(slice.try_into().unwrap())
    }

    pub fn read_byte(&mut self) -> Result<i8, KryoError> {
        Ok(self.byte()? as i8)
    }

    pub fn read_boolean(&mut self) -> Result<bool, KryoError> {
        Ok(self.byte()? == 1)
    }

    pub fn read_short(&mut self) -> Result<i16, KryoError> {
        Ok(i16::from_be_bytes(self.take()?))
    }

    pub fn read_char(&mut self) -> Result<u16, KryoError> {
        Ok(u16::from_be_bytes(self.take()?))
    }

    pub fn read_fixed_int(&mut self) -> Result<i32, KryoError> {
        Ok(i32::from_be_bytes(self.take()?))
    }

    pub fn read_fixed_long(&mut self) -> Result<i64, KryoError> {
        Ok(i64::from_be_bytes(self.take()?))
    }

    pub fn read_float(&mut self) -> Result<f32, KryoError> {
        Ok(f32::from_bits(self.read_fixed_int()? as u32))
    }

    pub fn read_double(&mut self) -> Result<f64, KryoError> {
        Ok(f64::from_bits(self.read_fixed_long()? as u64))
    }

    /// `Input.readVarInt`: up to 5 groups of 7 bits; zigzag unless `optimize_positive`.
    pub fn read_var_int(&mut self, optimize_positive: bool) -> Result<i32, KryoError> {
        let mut result: u32 = 0;
        for shift in [0, 7, 14, 21, 28] {
            let b = self.byte()?;
            result |= ((b & 0x7f) as u32) << shift;
            if b & 0x80 == 0 {
                break;
            }
        }
        let result = result as i32;
        Ok(if optimize_positive { result } else { ((result as u32) >> 1) as i32 ^ -(result & 1) })
    }

    /// `Input.readVarLong`: 8 groups of 7 bits, then a whole ninth byte, sign-extended like Java's
    /// `(long) b << 56`.
    pub fn read_var_long(&mut self, optimize_positive: bool) -> Result<i64, KryoError> {
        let mut result: i64 = 0;
        let mut more = true;
        for shift in [0, 7, 14, 21, 28, 35, 42, 49] {
            let b = self.byte()?;
            result |= ((b & 0x7f) as i64) << shift;
            if b & 0x80 == 0 {
                more = false;
                break;
            }
        }
        if more {
            result |= (self.byte()? as i8 as i64) << 56;
        }
        Ok(if optimize_positive { result } else { ((result as u64) >> 1) as i64 ^ -(result & 1) })
    }

    /// `Input.readString`. Strings of 2 to 63 ASCII chars end at the byte with the high bit set;
    /// the rest carry a length (chars + 1, 0 meaning null) and Kryo's UTF-8 variant of UTF-16 units.
    pub fn read_string(&mut self) -> Result<Option<String>, KryoError> {
        let first = self.byte()?;
        if first & 0x80 == 0 {
            let start = self.position - 1;
            loop {
                if self.byte()? & 0x80 != 0 {
                    break;
                }
            }
            let mut ascii: Vec<u8> = self.bytes[start..self.position].to_vec();
            *ascii.last_mut().unwrap() &= 0x7f;
            return Ok(Some(String::from_utf8(ascii).map_err(|_| self.error("invalid ASCII string"))?));
        }
        let mut char_count = (first & 0x3f) as u32;
        if first & 0x40 != 0 {
            for shift in [6, 13, 20, 27] {
                let b = self.byte()?;
                char_count |= ((b & 0x7f) as u32) << shift;
                if b & 0x80 == 0 {
                    break;
                }
            }
        }
        match char_count {
            0 => return Ok(None),
            1 => return Ok(Some(String::new())),
            _ => {}
        }
        let count = (char_count - 1) as usize;
        let mut units: Vec<u16> = Vec::with_capacity(count);
        while units.len() < count {
            let b = self.byte()? as u16;
            let unit = match b >> 4 {
                0..=7 => b,
                12 | 13 => ((b & 0x1f) << 6) | (self.byte()? as u16 & 0x3f),
                14 => {
                    let b2 = self.byte()? as u16;
                    let b3 = self.byte()? as u16;
                    ((b & 0x0f) << 12) | ((b2 & 0x3f) << 6) | (b3 & 0x3f)
                }
                // Kryo skips the byte and leaves whatever its char buffer held: not reproducible.
                _ => 0xfffd,
            };
            units.push(unit);
        }
        Ok(Some(String::from_utf16_lossy(&units)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn var_int_zigzag() {
        // Kryo writes -1 as zigzag 1, 300 as 600 = 0xd8 0x04
        assert_eq!(Input::new(&[0x01]).read_var_int(false).unwrap(), -1);
        assert_eq!(Input::new(&[0xd8, 0x04]).read_var_int(false).unwrap(), 300);
        assert_eq!(Input::new(&[0xac, 0x02]).read_var_int(true).unwrap(), 300);
    }

    #[test]
    fn ascii_and_utf8_strings() {
        assert_eq!(Input::new(b"ab\xe3").read_string().unwrap().as_deref(), Some("abc"));
        assert_eq!(Input::new(&[0x80]).read_string().unwrap(), None);
        assert_eq!(Input::new(&[0x81]).read_string().unwrap().as_deref(), Some(""));
        // "ã" = U+00E3, length 1 + 1 = 2 -> 0x82, then 0xc3 0xa3
        assert_eq!(Input::new(&[0x82, 0xc3, 0xa3]).read_string().unwrap().as_deref(), Some("ã"));
    }
}
