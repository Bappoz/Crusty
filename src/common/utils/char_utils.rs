// Possivel reutilizacao de funcoes
// Todos sao #[inline] => faz o compilador colocar o codigo no lugar da chamada
// ZERO OVERHEAD, mas pode aumentar o tamanho do binário

/// Retorna true se o char pode INICIAR um identificador (letra ou _)
#[inline]
pub fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

/// Retorna true se o char pode CONTINUAR um identificador (letra, dígito ou _)
#[inline]
pub fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Retorna true se o char é dígito decimal (0–9)
#[inline]
pub fn is_decimal_digit(c: char) -> bool {
    c.is_ascii_digit()
}

/// Retorna true se o char é dígito hexadecimal (0–9, a–f, A–F)
#[inline]
pub fn is_hex_digit(c: char) -> bool {
    c.is_ascii_hexdigit()
}

/// Retorna true se o char é dígito octal (0–7)
#[inline]
pub fn is_octa_digit(c: char) -> bool {
    matches!(c, '0'..='7')
}

/// Retorna true se o char é whitespace
#[inline]
pub fn is_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r' | '\n')
}

/// Resolve uma sequência de escape: recebe o char APÓS a \ e retorna o resultado.
pub fn resolve_escape(c: char) -> Option<char> {
    match c {
        'n' => Some('\n'),
        't' => Some('\t'),
        'r' => Some('\r'),
        '\\' => Some('\\'),
        '"' => Some('"'),
        '\'' => Some('\''),
        '0' => Some('\0'),
        _ => None,
    }
}
