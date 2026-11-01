use crate::diagnostic::{Diagnostic, Result, Span};

#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    Integer(i64),
    Decimal(f64),
    Text(String),
    Ident(String),
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Dot,
    Colon,
    Newline,
    Semicolon,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Pow,
    Eq,
    EqEq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Not,
    Range,
    Arrow,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    Eof,
}
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: Kind,
    pub span: Span,
}

pub fn lex(source: &str) -> Result<Vec<Token>> {
    Lexer {
        source,
        offset: 0,
        line: 1,
        column: 1,
        tokens: Vec::new(),
    }
    .scan()
}
struct Lexer<'a> {
    source: &'a str,
    offset: usize,
    line: usize,
    column: usize,
    tokens: Vec<Token>,
}
impl<'a> Lexer<'a> {
    fn peek(&self) -> Option<char> {
        self.source[self.offset..].chars().next()
    }
    fn next_char(&self) -> Option<char> {
        self.source[self.offset..].chars().nth(1)
    }
    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.offset += c.len_utf8();
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(c)
    }
    fn span(&self) -> Span {
        Span {
            start: self.offset,
            end: self.offset,
            line: self.line,
            column: self.column,
        }
    }
    fn push(&mut self, kind: Kind, mut span: Span) {
        span.end = self.offset;
        self.tokens.push(Token { kind, span });
    }
    fn scan(mut self) -> Result<Vec<Token>> {
        while let Some(c) = self.peek() {
            let s = self.span();
            match c {
                ' ' | '\t' | '\r' => {
                    self.bump();
                }
                '\n' => {
                    self.bump();
                    self.push(Kind::Newline, s);
                }
                '#' => {
                    while self.peek().is_some_and(|x| x != '\n') {
                        self.bump();
                    }
                }
                '/' if self.next_char() == Some('/') => {
                    while self.peek().is_some_and(|x| x != '\n') {
                        self.bump();
                    }
                }
                '/' if self.next_char() == Some('*') => {
                    self.bump();
                    self.bump();
                    let mut depth = 1;
                    while depth > 0 {
                        match (self.peek(), self.next_char()) {
                            (None, _) => {
                                return Err(Diagnostic::new("Unterminated block comment", s))
                            }
                            (Some('/'), Some('*')) => {
                                self.bump();
                                self.bump();
                                depth += 1;
                            }
                            (Some('*'), Some('/')) => {
                                self.bump();
                                self.bump();
                                depth -= 1;
                            }
                            (Some('\n'), _) => {
                                let n = self.span();
                                self.bump();
                                self.push(Kind::Newline, n);
                            }
                            _ => {
                                self.bump();
                            }
                        }
                    }
                }
                '"' | '\'' => {
                    let value = self.string(c, s)?;
                    self.push(Kind::Text(value), s);
                }
                '0'..='9' => {
                    let k = self.number(s)?;
                    self.push(k, s);
                }
                c if c == '_' || c.is_alphabetic() => {
                    let start = self.offset;
                    self.bump();
                    while self.peek().is_some_and(|x| x == '_' || x.is_alphanumeric()) {
                        self.bump();
                    }
                    let word = &self.source[start..self.offset];
                    let k = match word {
                        "and" => Kind::And,
                        "or" => Kind::Or,
                        "not" => Kind::Not,
                        _ => Kind::Ident(word.into()),
                    };
                    self.push(k, s);
                }
                _ => {
                    self.bump();
                    let second = self.peek();
                    let kind = match (c, second) {
                        ('+', Some('=')) => {
                            self.bump();
                            Kind::PlusEq
                        }
                        ('-', Some('=')) => {
                            self.bump();
                            Kind::MinusEq
                        }
                        ('*', Some('=')) => {
                            self.bump();
                            Kind::StarEq
                        }
                        ('/', Some('=')) => {
                            self.bump();
                            Kind::SlashEq
                        }
                        ('%', Some('=')) => {
                            self.bump();
                            Kind::PercentEq
                        }
                        ('*', Some('*')) => {
                            self.bump();
                            Kind::Pow
                        }
                        ('=', Some('=')) => {
                            self.bump();
                            Kind::EqEq
                        }
                        ('!', Some('=')) => {
                            self.bump();
                            Kind::Ne
                        }
                        ('<', Some('=')) => {
                            self.bump();
                            Kind::Le
                        }
                        ('>', Some('=')) => {
                            self.bump();
                            Kind::Ge
                        }
                        ('-', Some('>')) => {
                            self.bump();
                            Kind::Arrow
                        }
                        ('&', Some('&')) => {
                            self.bump();
                            Kind::And
                        }
                        ('|', Some('|')) => {
                            self.bump();
                            Kind::Or
                        }
                        ('.', Some('.')) => {
                            self.bump();
                            Kind::Range
                        }
                        ('(', _) => Kind::LParen,
                        (')', _) => Kind::RParen,
                        ('{', _) => Kind::LBrace,
                        ('}', _) => Kind::RBrace,
                        ('[', _) => Kind::LBracket,
                        (']', _) => Kind::RBracket,
                        (',', _) => Kind::Comma,
                        ('.', _) => Kind::Dot,
                        (':', _) => Kind::Colon,
                        (';', _) => Kind::Semicolon,
                        ('+', _) => Kind::Plus,
                        ('-', _) => Kind::Minus,
                        ('*', _) => Kind::Star,
                        ('/', _) => Kind::Slash,
                        ('%', _) => Kind::Percent,
                        ('=', _) => Kind::Eq,
                        ('!', _) => Kind::Not,
                        ('<', _) => Kind::Lt,
                        ('>', _) => Kind::Gt,
                        _ => return Err(Diagnostic::new(format!("Unexpected character {c:?}"), s)),
                    };
                    self.push(kind, s);
                }
            }
        }
        let s = self.span();
        self.push(Kind::Eof, s);
        Ok(self.tokens)
    }
    fn string(&mut self, quote: char, span: Span) -> Result<String> {
        self.bump();
        let mut value = String::new();
        loop {
            match self.bump() {
                None | Some('\n') => {
                    return Err(Diagnostic::new("Unterminated text literal", span))
                }
                Some(c) if c == quote => break,
                Some('\\') => {
                    let esc = self
                        .bump()
                        .ok_or_else(|| Diagnostic::new("Unterminated escape", span))?;
                    value.push(match esc {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        '0' => '\0',
                        '\\' => '\\',
                        '\'' => '\'',
                        '"' => '"',
                        _ => return Err(Diagnostic::new(format!("Unknown escape \\{esc}"), span)),
                    });
                }
                Some(c) => value.push(c),
            }
        }
        Ok(value)
    }
    fn number(&mut self, span: Span) -> Result<Kind> {
        let start = self.offset;
        while self.peek().is_some_and(|x| x.is_ascii_digit() || x == '_') {
            self.bump();
        }
        if self.peek() == Some('d')
            && matches!(&self.source[start..self.offset], "2" | "3")
            && self
                .next_char()
                .map_or(true, |c| !c.is_alphanumeric() && c != '_')
        {
            self.bump();
            return Ok(Kind::Ident(self.source[start..self.offset].into()));
        }
        let mut decimal = false;
        if self.peek() == Some('.') && self.next_char().is_some_and(|x| x.is_ascii_digit()) {
            decimal = true;
            self.bump();
            while self.peek().is_some_and(|x| x.is_ascii_digit() || x == '_') {
                self.bump();
            }
        }
        if matches!(self.peek(), Some('e' | 'E')) {
            decimal = true;
            self.bump();
            if matches!(self.peek(), Some('+' | '-')) {
                self.bump();
            }
            let exp = self.offset;
            while self.peek().is_some_and(|x| x.is_ascii_digit() || x == '_') {
                self.bump();
            }
            if exp == self.offset {
                return Err(Diagnostic::new("Expected digits after exponent", span));
            }
        }
        let raw = &self.source[start..self.offset];
        if raw.ends_with('_')
            || raw.contains("__")
            || raw.contains("_.")
            || raw.contains("._")
            || raw.contains("_e")
            || raw.contains("_E")
            || raw.contains("e_")
            || raw.contains("E_")
        {
            return Err(Diagnostic::new("Underscores must separate digits", span));
        }
        let raw = raw.replace('_', "");
        if decimal {
            let x = raw
                .parse::<f64>()
                .map_err(|_| Diagnostic::new("Invalid decimal literal", span))?;
            if !x.is_finite() {
                return Err(Diagnostic::new("Decimal literal is out of range", span));
            }
            Ok(Kind::Decimal(x))
        } else {
            raw.parse::<i64>().map(Kind::Integer).map_err(|_| {
                Diagnostic::new("Integer literal is out of range (signed 64-bit)", span)
            })
        }
    }
}
