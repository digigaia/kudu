// see: https://www.rfc-editor.org/rfc/rfc4648.html#section-5

const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
const SEXTET_MASK: u32 = (1 << 6) - 1;

#[inline]
fn extract_sextet(chunk: u32, offset: u32) -> u32 {
    (chunk & (SEXTET_MASK << offset)) >> offset
}

pub fn encode(data: &[u8]) -> String {
    let mut result = String::with_capacity((data.len().saturating_sub(1)) / 3 + 1);

    let mut add_char = |n: u32| {
        result.push(CHARSET[n as usize] as char);
    };

    let byte_length = data.len();
    let byte_remainder = byte_length % 3;
    let main_length = byte_length - byte_remainder;

    // deal with bytes in chunks of 3
    for i in (0..main_length).step_by(3) {
        // combine the three bytes into a single integer
        let chunk: u32 = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8) | (data[i + 2] as u32);

        // use bitmasks to extract 6-bit segments from the triplet
        let a = extract_sextet(chunk, 18);
        let b = extract_sextet(chunk, 12);
        let c = extract_sextet(chunk, 6);
        let d = extract_sextet(chunk, 0);

        add_char(a);
        add_char(b);
        add_char(c);
        add_char(d);
    }

    // deal with the remaining bytes
    if byte_remainder == 1 {
        let chunk: u32 = data[main_length] as u32;

        add_char(extract_sextet(chunk, 2));

        // use last 2 bits and set the 4 least significant bits to zero
        let b = (chunk & 3) << 4;
        add_char(b);
    }
    else if byte_remainder == 2 {
        let chunk: u32 = ((data[main_length] as u32) << 8) | (data[main_length + 1] as u32);
        let a = dbg!(extract_sextet(chunk, 10));
        let b = dbg!(extract_sextet(chunk, 4));

        // use last 4 bits and set the 2 least significant bits to zero
        let c = (chunk & 15) << 2;

        add_char(a);
        add_char(b);
        add_char(c);
    }

    result
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_empty_string() {
        assert_eq!(encode(b""), "");
    }

    #[test]
    fn test_basic_strings() {
        assert_eq!(encode(b"a"), "YQ");
        assert_eq!(encode(b"ab"), "YWI");
        assert_eq!(encode(b"abc"), "YWJj");
        assert_eq!(encode(b"hello"), "aGVsbG8");
        assert_eq!(encode(b"hello!"), "aGVsbG8h");
        assert_eq!(encode(b"hello world :)"), "aGVsbG8gd29ybGQgOik");
    }
}
