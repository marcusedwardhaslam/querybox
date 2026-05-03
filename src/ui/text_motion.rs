#[allow(dead_code)]
pub fn line_start(text: &str, offset: usize) -> usize {
    text[..offset].rfind('\n').map(|i| i + 1).unwrap_or(0)
}

#[allow(dead_code)]
pub fn line_end(text: &str, offset: usize) -> usize {
    text[offset..].find('\n').map(|i| offset + i).unwrap_or(text.len())
}

#[inline]
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

#[allow(dead_code)]
pub fn prev_word_start(text: &str, offset: usize) -> usize {
    let s = &text[..offset];
    let mut chars = s.char_indices().rev().peekable();

    // skip trailing whitespace
    while matches!(chars.peek(), Some((_, c)) if c.is_whitespace()) {
        chars.next();
    }

    let Some(&(_, first)) = chars.peek() else {
        return 0;
    };
    let class = is_word_char(first);

    // remember the byte offset of the last consumed char
    let mut last_idx = 0;
    while let Some(&(idx, c)) = chars.peek() {
        if is_word_char(c) != class {
            break;
        }
        last_idx = idx;
        chars.next();
    }

    if chars.peek().is_none() {
        0
    } else {
        last_idx
    }
}

#[allow(dead_code)]
pub fn next_word_end(text: &str, offset: usize) -> usize {
    let s = &text[offset..];
    let mut chars = s.char_indices().peekable();

    // skip leading whitespace
    while matches!(chars.peek(), Some((_, c)) if c.is_whitespace()) {
        chars.next();
    }

    let Some(&(_, first)) = chars.peek() else {
        return text.len();
    };
    let class = is_word_char(first);

    let mut last_end = text.len();
    while let Some(&(idx, c)) = chars.peek() {
        if is_word_char(c) != class {
            last_end = offset + idx;
            break;
        }
        chars.next();
    }

    last_end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_start() {
        assert_eq!(line_start("hello\nworld", 8), 6); // inside "world"
        assert_eq!(line_start("hello\nworld", 3), 0); // inside "hello"
        assert_eq!(line_start("hello\nworld", 6), 6); // at start of "world"
        assert_eq!(line_start("hello", 3), 0);         // single line
        assert_eq!(line_start("hello\nworld", 0), 0);  // at document start
    }

    #[test]
    fn test_line_end() {
        assert_eq!(line_end("hello\nworld", 3), 5);   // inside "hello"
        assert_eq!(line_end("hello\nworld", 8), 11);  // inside "world"
        assert_eq!(line_end("hello\nworld", 5), 5);   // at end of "hello"
        assert_eq!(line_end("hello", 3), 5);           // single line
        assert_eq!(line_end("hello", 5), 5);           // at document end
        assert_eq!(line_end("hello\nworld", 11), 11); // at last char of last line
    }

    #[test]
    fn test_word_motion_multibyte() {
        // é is 2 bytes (0xC3 0xA9); offset 7 = after "hé" + "llo"
        let s = "héllo world";
        let after_hello = "héllo".len(); // 6 bytes
        assert_eq!(next_word_end(s, 0), after_hello);
        assert_eq!(prev_word_start(s, after_hello), 0);
    }

    #[test]
    fn test_prev_word_start() {
        assert_eq!(prev_word_start("hello world", 11), 6);  // after "world"
        assert_eq!(prev_word_start("hello world", 5), 0);   // after "hello"
        assert_eq!(prev_word_start("hello  world", 12), 7); // after "world" with double space
        assert_eq!(prev_word_start("hello world", 0), 0);   // at start
        assert_eq!(prev_word_start("hello world", 6), 0);   // at start of "world"
    }

    #[test]
    fn test_next_word_end() {
        assert_eq!(next_word_end("hello world", 0), 5);    // before "hello"
        assert_eq!(next_word_end("hello world", 5), 11);   // at space before "world"
        assert_eq!(next_word_end("hello  world", 0), 5);   // before "hello" with double space
        assert_eq!(next_word_end("hello world", 11), 11);  // at document end
    }
}
