#[allow(dead_code)]
pub fn line_start(text: &str, offset: usize) -> usize {
    text[..offset].rfind('\n').map(|i| i + 1).unwrap_or(0)
}

#[allow(dead_code)]
pub fn line_end(text: &str, offset: usize) -> usize {
    text[offset..].find('\n').map(|i| offset + i).unwrap_or(text.len())
}

#[allow(dead_code)]
pub fn prev_word_start(text: &str, offset: usize) -> usize {
    let s = &text[..offset];
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    let n = chars.len();
    if n == 0 {
        return 0;
    }
    let mut i = n;

    // skip trailing whitespace (moving backwards)
    while i > 0 && chars[i - 1].1.is_whitespace() {
        i -= 1;
    }
    if i == 0 {
        return 0;
    }

    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let class = is_word(chars[i - 1].1);

    while i > 0 && is_word(chars[i - 1].1) == class {
        i -= 1;
    }

    chars.get(i).map(|(idx, _)| *idx).unwrap_or(0)
}

#[allow(dead_code)]
pub fn next_word_end(text: &str, offset: usize) -> usize {
    let s = &text[offset..];
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    let n = chars.len();
    if n == 0 {
        return text.len();
    }
    let mut i = 0;

    // skip leading whitespace
    while i < n && chars[i].1.is_whitespace() {
        i += 1;
    }
    if i == n {
        return text.len();
    }

    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let class = is_word(chars[i].1);

    while i < n && is_word(chars[i].1) == class {
        i += 1;
    }

    if i == n {
        text.len()
    } else {
        offset + chars[i].0
    }
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
