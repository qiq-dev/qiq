use std::collections::{HashMap, HashSet};

use crate::fetch::{self, Sums, SUM_FILE};
use crate::modfile::{self, Version, MOD_FILE};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
enum TokenKind {
    Function,
    Return,
    If,
    Else,
    New,
    Map,
    Foreach,
    While,
    Switch,
    Case,
    Default,
    As,
    Break,
    Continue,
    Try,
    Catch,
    Finally,
    Throw,
    Class,
    Abstract,
    Final,
    Static,
    Public,
    Private,
    Protected,
    Extends,
    Interface,
    Implements,
    Parent,
    Int,
    Double,
    Bool,
    String,
    CharType,
    ByteType,
    Void,
    Print,
    /// `println`: print, with each value on its own line.
    PrintLine,
    Identifier(String),
    /// Integer literal digits (up to 2^63, so that `-9223372036854775808`
    /// can be written); range-checked by the parser.
    Number(u64),
    Decimal(f64),
    Boolean(bool),
    NullLiteral,
    Text(String),
    Character(char),
    Equals,
    EqualEqual,
    BangEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    AndAnd,
    OrOr,
    /// `??`
    QuestionQuestion,
    /// `?->`
    NullsafeArrow,
    FatArrow,
    Plus,
    Minus,
    /// `++` and `--`, which are statements: `count++;`.
    PlusPlus,
    MinusMinus,
    /// `+=`, `-=`, `*=`, `/=`, `%=`, `&=`, `|=`, `^=`, `<<=`, `>>=`.
    AssignOp(BinaryOp),
    Bang,
    Star,
    StarStar,
    Slash,
    Percent,
    Ampersand,
    Pipe,
    Caret,
    Tilde,
    ShiftLeft,
    ShiftRight,
    Comma,
    Semicolon,
    Colon,
    DoubleColon,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Arrow,
    /// `...`, which ends the parameters of a standard library method that
    /// takes query parameters (see Function::binds_values).
    Ellipsis,
    End,
}

#[derive(Debug, Clone)]
struct Token {
    kind: TokenKind,
    line: usize,
    column: usize,
}

/// One piece of an f-string: text, or an expression (its source, the column
/// it starts at, and the format spec after a `:`, if any).
enum FStringPart {
    Text(String),
    Expression(String, usize, Option<String>),
}

/// Reads the f-string starting at `chars[*cursor]` (the `f`) and pushes the
/// tokens of the expression it stands for: its text and expressions joined
/// with `+`, each expression converted by __interpolate (like to_string, with
/// null as "null"), or by sprintf with its format spec, as in `{price:.2f}`.
/// The text takes the same escapes as a string; `{{` and `}}` are braces.
fn lex_fstring(chars: &[char], cursor: &mut usize, line: usize, column: &mut usize, tokens: &mut Vec<Token>) -> Result<(), String> {
    let start_column = *column;
    let at = |column: usize| format!("{line}:{column}");
    *cursor += 2;
    *column += 2;
    let mut parts = Vec::new();
    let mut text = String::new();
    loop {
        let Some(&c) = chars.get(*cursor) else {
            return Err(format!("{}: unterminated string literal", at(start_column)));
        };
        match c {
            '"' => {
                *cursor += 1;
                *column += 1;
                break;
            }
            '\n' => return Err(format!("{}: newline in string literal", at(start_column))),
            '\\' => {
                let escaped = chars.get(*cursor + 1).copied().ok_or_else(|| format!("{}: unterminated string escape", at(start_column)))?;
                text.push(match escaped {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '"' => '"',
                    '\\' => '\\',
                    _ => return Err(format!("{}: unsupported string escape \\{escaped}", at(*column))),
                });
                *cursor += 2;
                *column += 2;
            }
            '{' if chars.get(*cursor + 1) == Some(&'{') => {
                text.push('{');
                *cursor += 2;
                *column += 2;
            }
            '}' if chars.get(*cursor + 1) == Some(&'}') => {
                text.push('}');
                *cursor += 2;
                *column += 2;
            }
            '}' => return Err(format!("{}: single '}}' in an f-string; write '}}}}' for a brace", at(*column))),
            '{' => {
                let open_column = *column;
                *cursor += 1;
                *column += 1;
                let expression_column = *column;
                let mut expression = String::new();
                let mut spec: Option<String> = None;
                let mut depth = 0usize;
                loop {
                    let Some(&c) = chars.get(*cursor) else {
                        return Err(format!("{}: '{{' in an f-string without a closing '}}'", at(open_column)));
                    };
                    if c == '\n' {
                        return Err(format!("{}: '{{' in an f-string without a closing '}}'", at(open_column)));
                    }
                    if let Some(spec) = spec.as_mut() {
                        *cursor += 1;
                        *column += 1;
                        if c == '}' {
                            break;
                        }
                        spec.push(c);
                        continue;
                    }
                    match c {
                        '"' | '\'' | '`' => {
                            // A string or char inside the expression, copied whole.
                            expression.push(c);
                            *cursor += 1;
                            *column += 1;
                            loop {
                                let Some(&inner) = chars.get(*cursor) else {
                                    return Err(format!("{}: '{{' in an f-string without a closing '}}'", at(open_column)));
                                };
                                if inner == '\n' {
                                    return Err(format!("{}: '{{' in an f-string without a closing '}}'", at(open_column)));
                                }
                                expression.push(inner);
                                *cursor += 1;
                                *column += 1;
                                if inner == '\\' && c != '`' {
                                    if let Some(&next) = chars.get(*cursor) {
                                        expression.push(next);
                                        *cursor += 1;
                                        *column += 1;
                                    }
                                } else if inner == c {
                                    break;
                                }
                            }
                            continue;
                        }
                        '(' | '[' | '{' => depth += 1,
                        ')' | ']' => depth = depth.saturating_sub(1),
                        '}' if depth == 0 => {
                            *cursor += 1;
                            *column += 1;
                            break;
                        }
                        '}' => depth -= 1,
                        ':' if depth == 0 && chars.get(*cursor + 1) == Some(&':') => {
                            expression.push_str("::");
                            *cursor += 2;
                            *column += 2;
                            continue;
                        }
                        ':' if depth == 0 => {
                            spec = Some(String::new());
                            *cursor += 1;
                            *column += 1;
                            continue;
                        }
                        _ => {}
                    }
                    expression.push(c);
                    *cursor += 1;
                    *column += 1;
                }
                if expression.trim().is_empty() {
                    return Err(format!("{}: empty expression in an f-string: write {{name}}, or {{{{}}}} for braces", at(open_column)));
                }
                if let Some(spec) = &spec {
                    let valid = !spec.is_empty() && spec.chars().last().is_some_and(|c| c.is_ascii_alphabetic())
                        && spec.chars().all(|c| c.is_ascii_alphanumeric() || "-+ 0#.".contains(c));
                    if !valid {
                        return Err(format!(
                            "{}: invalid format spec '{spec}' in an f-string: write it as in sprintf without the %, ending in its type, such as {{price:.2f}} or {{count:5d}}",
                            at(open_column)
                        ));
                    }
                }
                if !text.is_empty() {
                    parts.push(FStringPart::Text(std::mem::take(&mut text)));
                }
                parts.push(FStringPart::Expression(expression, expression_column, spec));
            }
            _ => {
                text.push(c);
                *cursor += 1;
                *column += 1;
            }
        }
    }
    if !text.is_empty() || parts.is_empty() {
        parts.push(FStringPart::Text(text));
    }
    let token = |kind: TokenKind| Token { kind, line, column: start_column };
    if let [FStringPart::Text(only)] = parts.as_slice() {
        tokens.push(token(TokenKind::Text(only.clone())));
        return Ok(());
    }
    tokens.push(token(TokenKind::LeftParen));
    for (index, part) in parts.into_iter().enumerate() {
        if index > 0 {
            tokens.push(token(TokenKind::Plus));
        }
        match part {
            FStringPart::Text(text) => tokens.push(token(TokenKind::Text(text))),
            FStringPart::Expression(source, expression_column, spec) => {
                match spec {
                    Some(spec) => {
                        tokens.push(token(TokenKind::Identifier("sprintf".to_string())));
                        tokens.push(token(TokenKind::LeftParen));
                        tokens.push(token(TokenKind::Text(format!("%{spec}"))));
                        tokens.push(token(TokenKind::Comma));
                    }
                    None => {
                        tokens.push(token(TokenKind::Identifier("__interpolate".to_string())));
                        tokens.push(token(TokenKind::LeftParen));
                    }
                }
                // The expression's own tokens, at their place in the line.
                let inner = lex(&source).map_err(|error| {
                    // "1:N: message" from the expression: move it to the f-string's line.
                    match error.split_once(':').and_then(|(_, rest)| rest.split_once(':')) {
                        Some((offset, message)) if error.starts_with("1:") => match offset.parse::<usize>() {
                            Ok(offset) => format!("{line}:{}:{message}", expression_column + offset - 1),
                            Err(_) => error.clone(),
                        },
                        _ => error.clone(),
                    }
                })?;
                for mut inner_token in inner {
                    if matches!(inner_token.kind, TokenKind::End) {
                        continue;
                    }
                    inner_token.line = line;
                    inner_token.column += expression_column - 1;
                    tokens.push(inner_token);
                }
                tokens.push(token(TokenKind::RightParen));
            }
        }
    }
    tokens.push(token(TokenKind::RightParen));
    Ok(())
}

fn lex(source: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut cursor = 0;
    let mut line = 1;
    let mut column = 1;

    while cursor < chars.len() {
        let current = chars[cursor];
        if current.is_whitespace() {
            if current == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
            cursor += 1;
            continue;
        }
        if current == '/' && chars.get(cursor + 1) == Some(&'/') {
            while cursor < chars.len() && chars[cursor] != '\n' {
                cursor += 1;
                column += 1;
            }
            continue;
        }
        // A block comment, which may span lines. Block comments don't nest:
        // the first `*/` ends it.
        if current == '/' && chars.get(cursor + 1) == Some(&'*') {
            let (start_line, start_column) = (line, column);
            cursor += 2;
            column += 2;
            loop {
                match chars.get(cursor) {
                    None => return Err(format!("{start_line}:{start_column}: unterminated comment; add '*/' to end it")),
                    Some('*') if chars.get(cursor + 1) == Some(&'/') => {
                        cursor += 2;
                        column += 2;
                        break;
                    }
                    Some('\n') => {
                        cursor += 1;
                        line += 1;
                        column = 1;
                    }
                    Some(_) => {
                        cursor += 1;
                        column += 1;
                    }
                }
            }
            continue;
        }

        // An f-string: f"text {expression} text", made into the tokens of
        // ("text" + __interpolate(expression) + "text").
        if current == 'f' && chars.get(cursor + 1) == Some(&'"') {
            lex_fstring(&chars, &mut cursor, line, &mut column, &mut tokens)?;
            continue;
        }

        let token_line = line;
        let token_column = column;
        let kind = if current.is_ascii_alphabetic() || current == '_' {
            let start = cursor;
            while cursor < chars.len()
            && (chars[cursor].is_ascii_alphanumeric() || chars[cursor] == '_')
            {
                cursor += 1;
                column += 1;
            }
            let word: String = chars[start..cursor].iter().collect();
            match word.as_str() {
                "function" => TokenKind::Function,
                "return" => TokenKind::Return,
                "try" => TokenKind::Try,
                "catch" => TokenKind::Catch,
                "finally" => TokenKind::Finally,
                "throw" => TokenKind::Throw,
                "if" => TokenKind::If,
                "else" => TokenKind::Else,
                "new" => TokenKind::New,
                "map" => TokenKind::Map,
                "foreach" => TokenKind::Foreach,
                "while" => TokenKind::While,
                "switch" => TokenKind::Switch,
                "case" => TokenKind::Case,
                "default" => TokenKind::Default,
                "as" => TokenKind::As,
                "break" => TokenKind::Break,
                "continue" => TokenKind::Continue,
                "class" => TokenKind::Class,
                "abstract" => TokenKind::Abstract,
                "final" => TokenKind::Final,
                "static" => TokenKind::Static,
                "public" => TokenKind::Public,
                "private" => TokenKind::Private,
                "protected" => TokenKind::Protected,
                "extends" => TokenKind::Extends,
                "interface" => TokenKind::Interface,
                "implements" => TokenKind::Implements,
                "parent" => TokenKind::Parent,
                "int" => TokenKind::Int,
                "double" => TokenKind::Double,
                "bool" => TokenKind::Bool,
                "string" => TokenKind::String,
                "char" => TokenKind::CharType,
                "byte" => TokenKind::ByteType,
                "void" => TokenKind::Void,
                "print" => TokenKind::Print,
                "println" => TokenKind::PrintLine,
                "true" => TokenKind::Boolean(true),
                "false" => TokenKind::Boolean(false),
                "null" => TokenKind::NullLiteral,
                _ => TokenKind::Identifier(word),
            }
        } else if current.is_ascii_digit() {
            let start = cursor;
            while cursor < chars.len() && chars[cursor].is_ascii_digit() {
                cursor += 1;
                column += 1;
            }
            let mut is_double = false;
            if cursor < chars.len()
                && chars[cursor] == '.'
                && chars.get(cursor + 1).is_some_and(char::is_ascii_digit)
            {
                is_double = true;
                cursor += 1;
                column += 1;
                while cursor < chars.len() && chars[cursor].is_ascii_digit() {
                    cursor += 1;
                    column += 1;
                }
            }
            // An exponent (`e`/`E`, an optional sign, then digits) makes the
            // literal a double: 1e5, 2.5E-3. Without digits after it, the `e`
            // isn't part of the number.
            if matches!(chars.get(cursor), Some('e' | 'E')) {
                let digits_at = if matches!(chars.get(cursor + 1), Some('+' | '-')) { cursor + 2 } else { cursor + 1 };
                if chars.get(digits_at).is_some_and(char::is_ascii_digit) {
                    is_double = true;
                    column += digits_at - cursor;
                    cursor = digits_at;
                    while cursor < chars.len() && chars[cursor].is_ascii_digit() {
                        cursor += 1;
                        column += 1;
                    }
                }
            }
            if is_double {
                let literal: String = chars[start..cursor].iter().collect();
                let value = literal.parse::<f64>().map_err(|_| {
                    format!("{token_line}:{token_column}: invalid double literal")
                })?;
                if !value.is_finite() {
                    return Err(format!("{token_line}:{token_column}: double literal is out of range"));
                }
                TokenKind::Decimal(value)
            } else {
                let literal: String = chars[start..cursor].iter().collect();
                let value = literal
                    .parse::<u64>()
                    .ok()
                    .filter(|value| *value <= 1u64 << 63)
                    .ok_or_else(|| {
                        format!("{token_line}:{token_column}: integer literal is outside the int range")
                    })?;
                TokenKind::Number(value)
            }
        } else if current == '\'' {
            cursor += 1;
            column += 1;
            let first = chars.get(cursor).copied().ok_or_else(|| {
                format!("{token_line}:{token_column}: unterminated char literal")
            })?;
            let value = if first == '\\' {
                cursor += 1;
                column += 1;
                let escaped = chars.get(cursor).copied().ok_or_else(|| {
                    format!("{token_line}:{token_column}: unterminated char escape")
                })?;
                cursor += 1;
                column += 1;
                match escaped {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '\'' => '\'',
                    '"' => '"',
                    '\\' => '\\',
                    _ => {
                        return Err(format!(
                            "{token_line}:{token_column}: unsupported char escape \\{escaped}"
                        ));
                    }
                }
            } else if first == '\'' || first == '\n' {
                return Err(format!(
                    "{token_line}:{token_column}: char literal must contain exactly one character"
                ));
            } else {
                cursor += 1;
                column += 1;
                first
            };
            if chars.get(cursor) != Some(&'\'') {
                return Err(format!(
                    "{token_line}:{token_column}: char literal must contain exactly one character"
                ));
            }
            cursor += 1;
            column += 1;
            TokenKind::Character(value)
        } else if current == '"' {
            cursor += 1;
            column += 1;
            let mut value = String::new();
            let mut closed = false;
            while cursor < chars.len() {
                let character = chars[cursor];
                cursor += 1;
                column += 1;
                match character {
                    '"' => {
                        closed = true;
                        break;
                    }
                    '\\' => {
                        let escaped = chars.get(cursor).copied().ok_or_else(|| {
                            format!("{token_line}:{token_column}: unterminated string escape")
                        })?;
                        cursor += 1;
                        column += 1;
                        value.push(match escaped {
                            'n' => '\n',
                            'r' => '\r',
                            't' => '\t',
                            '"' => '"',
                            '\\' => '\\',
                            _ => {
                                return Err(format!(
                                    "{token_line}:{token_column}: unsupported string escape \\{escaped}"
                                ));
                            }
                        });
                    }
                    '\n' => {
                        return Err(format!("{token_line}:{token_column}: newline in string literal"));
                    }
                    _ => value.push(character),
                }
            }
            if !closed {
                return Err(format!("{token_line}:{token_column}: unterminated string literal"));
            }
            TokenKind::Text(value)
        } else if current == '`' {
            // A raw string: everything up to the next backtick, as it is. No
            // escapes, and it may span lines, for regular expressions, SQL,
            // and Windows paths.
            cursor += 1;
            column += 1;
            let start = cursor;
            while cursor < chars.len() && chars[cursor] != '`' {
                if chars[cursor] == '\n' {
                    line += 1;
                    column = 1;
                } else {
                    column += 1;
                }
                cursor += 1;
            }
            if cursor >= chars.len() {
                return Err(format!("{token_line}:{token_column}: unterminated raw string; add '`' to end it"));
            }
            let value: String = chars[start..cursor].iter().collect();
            cursor += 1;
            column += 1;
            TokenKind::Text(value)
        } else {
            cursor += 1;
            column += 1;
            match current {
                '=' if chars.get(cursor) == Some(&'=') => {
                    cursor += 1;
                    column += 1;
                    TokenKind::EqualEqual
                }
                '=' if chars.get(cursor) == Some(&'>') => {
                    cursor += 1;
                    column += 1;
                    TokenKind::FatArrow
                }
                '=' => TokenKind::Equals,
                '!' if chars.get(cursor) == Some(&'=') => {
                    cursor += 1;
                    column += 1;
                    TokenKind::BangEqual
                }
                '!' => TokenKind::Bang,
                '<' if chars.get(cursor) == Some(&'<') && chars.get(cursor + 1) == Some(&'=') => {
                    cursor += 2;
                    column += 2;
                    TokenKind::AssignOp(BinaryOp::ShiftLeft)
                }
                '>' if chars.get(cursor) == Some(&'>') && chars.get(cursor + 1) == Some(&'=') => {
                    cursor += 2;
                    column += 2;
                    TokenKind::AssignOp(BinaryOp::ShiftRight)
                }
                '<' if chars.get(cursor) == Some(&'<') => {
                    cursor += 1;
                    column += 1;
                    TokenKind::ShiftLeft
                }
                '>' if chars.get(cursor) == Some(&'>') => {
                    cursor += 1;
                    column += 1;
                    TokenKind::ShiftRight
                }
                '<' if chars.get(cursor) == Some(&'=') => {
                    cursor += 1;
                    column += 1;
                    TokenKind::LessEqual
                }
                '>' if chars.get(cursor) == Some(&'=') => {
                    cursor += 1;
                    column += 1;
                    TokenKind::GreaterEqual
                }
                '.' if chars.get(cursor) == Some(&'.') && chars.get(cursor + 1) == Some(&'.') => {
                    cursor += 2;
                    column += 2;
                    TokenKind::Ellipsis
                }
                '<' => TokenKind::Less,
                '>' => TokenKind::Greater,
                '&' if chars.get(cursor) == Some(&'&') => {
                    cursor += 1;
                    column += 1;
                    TokenKind::AndAnd
                }
                '|' if chars.get(cursor) == Some(&'|') => {
                    cursor += 1;
                    column += 1;
                    TokenKind::OrOr
                }
                '?' if chars.get(cursor) == Some(&'-') && chars.get(cursor + 1) == Some(&'>') => {
                    cursor += 2;
                    column += 2;
                    TokenKind::NullsafeArrow
                }
                '?' if chars.get(cursor) == Some(&'?') => {
                    cursor += 1;
                    column += 1;
                    TokenKind::QuestionQuestion
                }
                // `op=`: compound assignment.
                '+' | '-' | '*' | '/' | '%' | '&' | '|' | '^' if chars.get(cursor) == Some(&'=') => {
                    cursor += 1;
                    column += 1;
                    TokenKind::AssignOp(match current {
                        '+' => BinaryOp::Add,
                        '-' => BinaryOp::Subtract,
                        '*' => BinaryOp::Multiply,
                        '/' => BinaryOp::Divide,
                        '%' => BinaryOp::Modulo,
                        '&' => BinaryOp::BitAnd,
                        '|' => BinaryOp::BitOr,
                        _ => BinaryOp::BitXor,
                    })
                }
                '&' => TokenKind::Ampersand,
                '|' => TokenKind::Pipe,
                '^' => TokenKind::Caret,
                '~' => TokenKind::Tilde,
                '-' if chars.get(cursor) == Some(&'>') => {
                    cursor += 1;
                    column += 1;
                    TokenKind::Arrow
                }
                '+' if chars.get(cursor) == Some(&'+') => {
                    cursor += 1;
                    column += 1;
                    TokenKind::PlusPlus
                }
                '-' if chars.get(cursor) == Some(&'-') => {
                    cursor += 1;
                    column += 1;
                    TokenKind::MinusMinus
                }
                '+' => TokenKind::Plus,
                '-' => TokenKind::Minus,
                '*' if chars.get(cursor) == Some(&'*') => {
                    cursor += 1;
                    column += 1;
                    TokenKind::StarStar
                }
                '*' => TokenKind::Star,
                '/' => TokenKind::Slash,
                '%' => TokenKind::Percent,
                ',' => TokenKind::Comma,
                ';' => TokenKind::Semicolon,
                ':' if chars.get(cursor) == Some(&':') => {
                    cursor += 1;
                    column += 1;
                    TokenKind::DoubleColon
                }
                ':' => TokenKind::Colon,
                '(' => TokenKind::LeftParen,
                ')' => TokenKind::RightParen,
                '{' => TokenKind::LeftBrace,
                '}' => TokenKind::RightBrace,
                '[' => TokenKind::LeftBracket,
                ']' => TokenKind::RightBracket,
                _ => {
                    return Err(format!(
                        "{token_line}:{token_column}: unexpected character {current:?}"
                    ));
                }
            }
        };
        tokens.push(Token {
            kind,
            line: token_line,
            column: token_column,
        });
    }
    tokens.push(Token {
        kind: TokenKind::End,
        line,
        column,
    });
    Ok(tokens)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueType {
    Int,
    Double,
    Bool,
    String,
    Char,
    /// An unsigned 8-bit value, 0 to 255, for binary data.
    Byte,
    /// An array; the id indexes the array type table, which holds the
    /// element type (any type except void and null, including arrays).
    Array(u32),
    Map(u32),
    Class(u32),
    /// A function value; the id indexes the function type table, which holds
    /// the parameter and return types.
    Function(u32),
    /// The type of `null`: the value a `void` function call evaluates to.
    /// `Void` is only used as a function return type.
    Null,
    Void,
}

impl ValueType {
    fn llvm(self) -> &'static str {
        match self {
            Self::Int => "i64",
            Self::Double => "double",
            Self::Bool => "i1",
            Self::String => "ptr",
            Self::Char => "ptr",
            Self::Byte => "i8",
            Self::Array(_)
            | Self::Map(_)
            | Self::Class(_)
            | Self::Function(_)
            | Self::Null => "ptr",
            Self::Void => "void",
        }
    }

    /// Bytes an array element of this type occupies: values for the scalar
    /// types, pointers for strings, chars, arrays, maps, and objects.
    fn size_bytes(self) -> usize {
        match self {
            Self::Bool | Self::Byte => 1,
            _ => 8,
        }
    }

    fn alignment(self) -> usize {
        self.size_bytes()
    }

    /// A single value (not a reference to an array, map, or object).
    fn is_scalar(self) -> bool {
        self.is_scalar_key() || self == Self::Byte
    }

    /// Types that can be map keys.
    fn is_scalar_key(self) -> bool {
        matches!(self, Self::Int | Self::Double | Self::Bool | Self::String | Self::Char)
    }

    fn runtime_tag(self) -> Result<u32, String> {
        match self {
            Self::Int => Ok(0),
            Self::Double => Ok(1),
            Self::Bool => Ok(2),
            Self::String => Ok(3),
            Self::Char => Ok(4),
            _ => Err("map key must use a scalar type".to_string()),
        }
    }
}

/// The array type whose elements are `element`, registering it if needed.
fn array_type_of(array_types: &mut Vec<ValueType>, element: ValueType) -> ValueType {
    let id = match array_types.iter().position(|candidate| *candidate == element) {
        Some(index) => index,
        None => {
            array_types.push(element);
            array_types.len() - 1
        }
    };
    ValueType::Array(id as u32)
}

/// The element type of an array type, or None for other types.
fn element_type_of(array_types: &[ValueType], value_type: ValueType) -> Option<ValueType> {
    match value_type {
        ValueType::Array(id) => array_types.get(id as usize).copied(),
        _ => None,
    }
}

/// A type as it is written in qiq source, for error messages: `int`,
/// `string[]`, `map[string, int[]]`, a class name, `null`, or `void`.
fn type_name(
    value_type: ValueType,
    map_types: &[MapType],
    array_types: &[ValueType],
    function_types: &[FunctionType],
    classes: &[ClassDef],
) -> String {
    let name = |value_type: ValueType| type_name(value_type, map_types, array_types, function_types, classes);
    match value_type {
        ValueType::Int => "int".to_string(),
        ValueType::Double => "double".to_string(),
        ValueType::Bool => "bool".to_string(),
        ValueType::String => "string".to_string(),
        ValueType::Char => "char".to_string(),
        ValueType::Byte => "byte".to_string(),
        ValueType::Array(type_id) => match array_types.get(type_id as usize) {
            Some(element) => format!("{}[]", name(*element)),
            None => "array".to_string(),
        },
        ValueType::Map(type_id) => match map_types.get(type_id as usize) {
            Some(map) => format!(
                "map[{}, {}]",
                name(map.key_type),
                name(map.value_type)
            ),
            None => "map".to_string(),
        },
        ValueType::Class(class_id) => classes
            .get(class_id as usize)
            .map_or_else(|| "object".to_string(), |class| class.name.clone()),
        ValueType::Function(type_id) => match function_types.get(type_id as usize) {
            Some(function) => {
                let parameters: Vec<String> = function.parameters.iter().map(|parameter| name(*parameter)).collect();
                format!("function({}): {}", parameters.join(", "), name(function.return_type))
            }
            None => "function".to_string(),
        },
        ValueType::Null => "null".to_string(),
        ValueType::Void => "void".to_string(),
    }
}

/// What an anonymous function captures, for emitting its body: each captured
/// variable's cell (or `this`), in the order of the closure object's fields.
#[derive(Debug, Clone)]
struct ClosureContext {
    captures: Vec<Capture>,
    class: Option<u32>,
    method: Option<String>,
}

#[derive(Debug, Clone)]
struct Capture {
    name: String,
    value_type: ValueType,
    /// A foreach or catch variable, which can't be reassigned.
    read_only: bool,
}

/// The per-function state of the emitter, saved while an anonymous function
/// is emitted in the middle of another function.
struct FunctionState {
    next_register: usize,
    next_label: usize,
    entry_allocas: String,
    temporaries: Vec<Vec<String>>,
    owned_slots: Vec<String>,
    handlers: Vec<String>,
    handler_scopes: Vec<usize>,
    finally_stack: Vec<FinallyContext>,
    loop_targets: Vec<LoopTargets>,
    current_class: Option<u32>,
    current_method: Option<String>,
    current_method_is_static: bool,
    boxed: std::collections::BTreeSet<String>,
    in_closure: bool,
    closure_has_this: bool,
}

/// `function(int, string): bool`: the type of a function value.
#[derive(Debug, Clone, PartialEq)]
struct FunctionType {
    parameters: Vec<ValueType>,
    return_type: ValueType,
}

/// The function type with these parameters and return type, registering it
/// if needed.
fn function_type_of(function_types: &mut Vec<FunctionType>, signature: FunctionType) -> ValueType {
    let id = match function_types.iter().position(|candidate| *candidate == signature) {
        Some(id) => id,
        None => {
            function_types.push(signature);
            function_types.len() - 1
        }
    };
    ValueType::Function(id as u32)
}

/// An anonymous function: `function(int x): int { ... }`, or the arrow form
/// `fn(int x) => x * 2`, whose body is one return and whose return type is
/// inferred (`return_type` is None).
#[derive(Debug, Clone)]
struct Closure {
    parameters: Vec<Parameter>,
    return_type: Option<ValueType>,
    body: Vec<Statement>,
    /// Where `function` or `fn` is.
    line: usize,
    column: usize,
}

#[derive(Debug, Clone, Copy)]
struct MapType {
    key_type: ValueType,
    value_type: ValueType,
}

#[derive(Debug, Clone)]
struct Parameter {
    value_type: ValueType,
    name: String,
    /// Default value (a literal) that makes the parameter optional. Optional
    /// parameters always follow the required ones.
    default: Option<Expr>,
}

#[derive(Debug, Clone)]
struct Function {
    name: String,
    parameters: Vec<Parameter>,
    return_type: ValueType,
    body: Vec<Statement>,
    visibility: Visibility,
    is_abstract: bool,
    /// `final function`: subclasses can't override it.
    is_final: bool,
    /// `static function`: called as `Class::name()`, without an object.
    is_static: bool,
    /// Declared with the return type `static`: returns an instance of the
    /// class the call was made on (return_type holds the declaring class).
    returns_static: bool,
    /// Module that declares the function (for methods, the class's module).
    module: u32,
    /// LLVM symbol of a free function: its name in the entry module, or a
    /// module-qualified name in imported modules so names can repeat across files.
    /// For an `extern function`, the C function `qiq_stdlib_<name>`.
    symbol: String,
    /// Declared with `extern function`: implemented in the C runtime, no body.
    is_extern: bool,
    /// Declared with a final `...` (standard library only): a call can pass
    /// any number of extra values (int, double, bool, byte, string, char,
    /// byte[], or null) after the parameters, such as the values for a SQL
    /// query's `?` placeholders. They are handed to the C runtime with
    /// qiq_bind_values just before the call, which takes them from there.
    binds_values: bool,
    /// Where the function's name is (for an anonymous function, `function`
    /// or `fn`), for errors about the function as a whole.
    line: usize,
    column: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Visibility {
    Public,
    Private,
    Protected,
}

fn visibility_rank(visibility: Visibility) -> u8 {
    match visibility {
        Visibility::Private => 0,
        Visibility::Protected => 1,
        Visibility::Public => 2,
    }
}

#[derive(Debug, Clone)]
struct Property {
    name: String,
    value_type: ValueType,
    visibility: Visibility,
    /// `type name = value;`: a constant expression stored when an object is
    /// created, before the constructor runs; and where it's written.
    default: Option<Expr>,
    default_at: (usize, usize),
    /// `readonly`: set only by its class's constructor.
    readonly: bool,
}

/// A constructor parameter declared with a visibility or `readonly`
/// (`function __construct(public int x)`): it's also a property, set from
/// the argument before the constructor's body runs.
#[derive(Debug, Clone)]
struct PromotedParameter {
    name: String,
    value_type: ValueType,
    visibility: Visibility,
    readonly: bool,
    line: usize,
    column: usize,
}

/// `const [type] NAME = value;` in a class or interface: a value fixed at
/// compile time, read as `Class::NAME` or `self::NAME`.
#[derive(Debug, Clone)]
struct Constant {
    name: String,
    /// The declared type, or None to take it from the value.
    declared: Option<ValueType>,
    visibility: Visibility,
    value: Expr,
    line: usize,
    column: usize,
}

/// `static type name [= literal];`: one value per declaring class.
#[derive(Debug, Clone)]
struct StaticProperty {
    name: String,
    value_type: ValueType,
    visibility: Visibility,
    /// Initial value (a literal); zero or null when absent.
    default: Option<Expr>,
}

#[derive(Debug, Clone)]
struct ClassDef {
    name: String,
    parent_class: Option<u32>,
    /// Interfaces implemented by a class, or extended by an interface.
    interfaces: Vec<u32>,
    is_abstract: bool,
    /// `final class`: can't be extended.
    is_final: bool,
    is_interface: bool,
    /// Module that declares the class.
    module: u32,
    /// Name used in LLVM symbols; module-qualified outside the entry module.
    symbol: String,
    properties: Vec<Property>,
    static_properties: Vec<StaticProperty>,
    constants: Vec<Constant>,
    methods: Vec<Function>,
    /// Where the class's name is, for errors about the class as a whole.
    line: usize,
    column: usize,
}

#[derive(Debug)]
struct Program {
    functions: Vec<Function>,
    classes: Vec<ClassDef>,
    /// Top-level statements of the entry module; they become `main`.
    statements: Vec<Statement>,
    map_types: Vec<MapType>,
    array_types: Vec<ValueType>,
    function_types: Vec<FunctionType>,
    modules: Vec<Module>,
}

/// A source file. Module 0 is the entry file passed to the compiler.
#[derive(Debug, Default)]
struct Module {
    /// Path shown in error messages; empty when compiling a source string.
    path: String,
    /// Functions callable from this module (declared or imported): name -> symbol.
    functions: HashMap<String, String>,
    /// Classes and interfaces declared by this module, importable by name.
    exported_classes: HashMap<String, u32>,
    /// Functions declared by this module, importable by name: name -> symbol.
    exported_functions: HashMap<String, String>,
    /// Part of the standard library, which may declare `extern function`.
    is_stdlib: bool,
    /// The module (qiq.mod package) the file belongs to: an index into
    /// Loader::packages.
    package: u32,
    /// Top-level names declared `private`.
    private_names: HashSet<String>,
}

#[derive(Debug, Clone)]
enum Expr {
    /// `value instanceof Class [name]`: whether `value` is an object of that
    /// class (or a subclass, or one implementing that interface). A name
    /// binds the object as that class, and is only allowed in an if
    /// condition, which the parser rewrites (see parse_if).
    InstanceOf(Box<Expr>, u32, Option<String>),
    /// The value of an expression known to be an object of class `u32`
    /// (made by the parser after an instanceof check).
    Downcast(Box<Expr>, u32),
    /// `object?->rest`: null if `object` is null, otherwise `rest` (the
    /// member, call, and anything after them in the chain), evaluated with
    /// the hidden variable named by the String standing for the object.
    Nullsafe(Box<Expr>, String, Box<Expr>),
    Number(i64),
    /// The 1 that `x++` and `x--` add or subtract, in the type of `x` (an
    /// int or a double); only ever the right side of a Binary.
    One,
    Decimal(f64),
    Boolean(bool),
    Null,
    Text(String),
    Character(char),
    Variable(String),
    Call(String, Vec<Expr>),
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
    Negate(Box<Expr>),
    /// Bitwise complement `~` of an int.
    BitNot(Box<Expr>),
    /// Logical not `!` of a bool.
    Not(Box<Expr>),
    /// A comparison; produces a bool.
    Compare(CompareOp, Box<Expr>, Box<Expr>),
    /// `&&` or `||` on bools, short-circuiting.
    Logical(LogicalOp, Box<Expr>, Box<Expr>),
    /// `left ?? right`: `left` unless it's a missing map key, an array index
    /// out of range, or null; then `right`, evaluated only in that case.
    Coalesce(Box<Expr>, Box<Expr>),
    /// An anonymous function.
    Closure(Box<Closure>),
    /// Calling a function value: `makeAdder(2)(3)`, `(object->callback)(x)`.
    CallValue(Box<Expr>, Vec<Expr>),
    Index(Box<Expr>, Box<Expr>),
    Member(Box<Expr>, String),
    MethodCall(Box<Expr>, String, Vec<Expr>),
    /// `Class::method(args)`, `self::`, `parent::`, or `static::`: a static
    /// method call, or (for self:: and parent::) a direct instance call.
    StaticCall(StaticTarget, String, Vec<Expr>),
    /// `Class::name`, `self::name`, `parent::name`, or `static::name`.
    StaticProperty(StaticTarget, String),
    /// `X::class`, `self::class`, `parent::class`, or `static::class`: the
    /// class's name, a constant except for static::class.
    ClassName(StaticTarget),
    /// `value::class`: the name of the object's class at run time.
    ClassOf(Box<Expr>),
    /// `new static(args)`: an instance of the class the call was made on;
    /// the u32 is the class whose code contains it.
    NewStatic(u32, Vec<Expr>),
    NewObject(u32, Vec<Expr>),
    /// `new type[size]`: a zero-filled array; the type is the element type.
    NewArray(ValueType, Box<Expr>),
    /// `[a, b, ...]`; the element type comes from the elements, or from the
    /// expected type when the literal is empty.
    ArrayLiteral(Vec<Expr>),
    /// `{key => value, ...}`; typed like ArrayLiteral.
    MapLiteral(Vec<(Expr, Expr)>),
    /// `name: value` in a call's argument list; only valid there.
    NamedArgument(String, Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,
    BitAnd,
    BitOr,
    BitXor,
    ShiftLeft,
    ShiftRight,
}

impl BinaryOp {
    fn is_bitwise(self) -> bool {
        matches!(
            self,
            Self::BitAnd | Self::BitOr | Self::BitXor | Self::ShiftLeft | Self::ShiftRight
        )
    }

    fn symbol(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::Modulo => "%",
            Self::Power => "**",
            Self::BitAnd => "&",
            Self::BitOr => "|",
            Self::BitXor => "^",
            Self::ShiftLeft => "<<",
            Self::ShiftRight => ">>",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogicalOp {
    And,
    Or,
}

#[derive(Debug, Clone, Copy)]
enum CompareOp {
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    Equal,
    NotEqual,
}

/// A statement and where it starts, so errors found while compiling it can
/// say where they are.
#[derive(Debug, Clone)]
struct Statement {
    kind: StatementKind,
    line: usize,
    column: usize,
}

impl Statement {
    /// A statement the compiler made itself, positioned like `like`.
    fn like(like: &Statement, kind: StatementKind) -> Self {
        Self { kind, line: like.line, column: like.column }
    }
}

#[derive(Debug, Clone)]
enum StatementKind {
    Declare(ValueType, String, Expr),
    DeclareArray(ValueType, String, ArrayInitializer),
    DeclareMap(ValueType, String, MapInitializer),
    Assign(String, Expr),
    AssignIndex(Expr, Expr, Expr),
    AssignMember(Expr, String, Expr),
    /// `Class::name = value;` (any StaticTarget).
    AssignStatic(StaticTarget, String, Expr),
    /// `print(a, b, ...)`: the values separated by spaces, then a newline.
    /// `print(values)`, or with true, `println(values)`: the same except that
    /// println writes each value on its own line instead of separating them
    /// with spaces.
    Print(Vec<Expr>, bool),
    Expression(Expr),
    Return(Option<Expr>),
    Break,
    Continue,
    /// `throw value;` where value is an Exception.
    Throw(Expr),
    /// `try { ... } catch (Type name) { ... } ... finally { ... }`.
    Try {
        body: Vec<Statement>,
        catches: Vec<CatchClause>,
        finally: Option<Vec<Statement>>,
    },
    If {
        /// Any bool expression.
        condition: Expr,
        then_body: Vec<Statement>,
        else_body: Vec<Statement>,
    },
    /// `switch (subject) { case a, b: ... default: ... }`. Cases don't fall
    /// through; `break` leaves the switch.
    Switch {
        subject: Expr,
        cases: Vec<SwitchCase>,
        default: Option<Vec<Statement>>,
    },
    /// `while (condition) { body }`
    While {
        condition: Expr,
        body: Vec<Statement>,
    },
    Foreach {
        iterable: Expr,
        key_name: Option<String>,
        value_name: String,
        body: Vec<Statement>,
    },
}

/// What the left side of `::` names.
#[derive(Debug, Clone, Copy)]
enum StaticTarget {
    /// `Class::` (`forwards` false) or `self::` (`forwards` true, with the
    /// enclosing class). self:: passes the called class along to static calls.
    Class { class_id: u32, forwards: bool },
    /// `parent::`, inside `enclosing`.
    Parent { parent: u32, enclosing: u32 },
    /// `static::`: the class the call was made on (late static binding),
    /// checked against `enclosing`, the class whose code contains it.
    Late { enclosing: u32 },
}

#[derive(Debug, Clone)]
struct SwitchCase {
    /// The values this case matches; any one of them selects it.
    values: Vec<Expr>,
    body: Vec<Statement>,
}

#[derive(Debug, Clone)]
struct CatchClause {
    /// The caught class; subclasses match too.
    class_id: u32,
    /// Variable bound to the exception inside the catch body.
    name: String,
    body: Vec<Statement>,
}

#[derive(Debug, Clone)]
enum ArrayInitializer {
    Uninitialized,
    /// Any array-typed expression: a literal `[...]`, `new type[size]`, a call.
    Existing(Expr),
}

#[derive(Debug, Clone)]
enum MapInitializer {
    Empty,
    /// Any map-typed expression: a literal `{...}`, a call, a variable.
    Existing(Expr),
}

/// Parses one module. `classes` and `map_types` are shared by every module so
/// class ids and map type ids are global; `class_names` holds the classes
/// visible in this module (imported plus declared).
struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
    map_types: Vec<MapType>,
    array_types: Vec<ValueType>,
    function_types: Vec<FunctionType>,
    class_names: HashMap<String, u32>,
    classes: Vec<ClassDef>,
    module: u32,
    module_tag: String,
    /// Whether `extern function` is allowed (standard library modules only).
    allow_extern: bool,
    /// The class whose body is being parsed; `self::` refers to it.
    current_class: Option<u32>,
    /// Its parent; `parent::` refers to it.
    current_parent: Option<u32>,
    /// Top-level functions, classes, and interfaces declared `private`:
    /// importable only within the same module (qiq.mod).
    private_names: HashSet<String>,
    /// Numbers the hidden variables that hold the parts of a compound
    /// assignment's target (see parse_update).
    next_hidden: usize,
    /// The promoted parameters of the constructor just parsed.
    promoted: Vec<PromotedParameter>,
}

const INCREMENT_IS_A_STATEMENT: &str = "'++' and '--' are statements in qiq: write `i++;` on its own, then use i";

impl Parser {
    fn array_of(&mut self, element: ValueType) -> ValueType {
        array_type_of(&mut self.array_types, element)
    }

    fn element_of(&self, value_type: ValueType) -> Option<ValueType> {
        element_type_of(&self.array_types, value_type)
    }

    /// A type as written in qiq source, for error messages.
    fn type_name(&self, value_type: ValueType) -> String {
        type_name(value_type, &self.map_types, &self.array_types, &self.function_types, &self.classes)
    }

    fn qualified_symbol(&self, name: &str, prefix: &str) -> String {
        if self.module_tag.is_empty() {
            name.to_string()
        } else {
            format!("{prefix}{}.{name}", self.module_tag)
        }
    }

    /// Parses the declarations and statements after the module's imports.
    fn parse_module(&mut self) -> Result<(Vec<Function>, Vec<Statement>), String> {
        let mut functions = Vec::new();
        let mut statements = Vec::new();
        while !matches!(self.current().kind, TokenKind::End) {
            if is_import_at(&self.tokens, self.cursor) {
                return Err(self.error("imports must appear before other declarations"));
            }
            if matches!(&self.current().kind, TokenKind::Identifier(word) if word == "extern")
                && matches!(
                    self.tokens.get(self.cursor + 1).map(|token| &token.kind),
                    Some(TokenKind::Function)
                )
            {
                functions.push(self.parse_extern_function()?);
                continue;
            }
            let is_private = matches!(self.current().kind, TokenKind::Private);
            if is_private {
                let next = self.tokens.get(self.cursor + 1).map(|token| &token.kind);
                let declaration = matches!(
                    next,
                    Some(TokenKind::Class | TokenKind::Abstract | TokenKind::Final | TokenKind::Interface | TokenKind::Function)
                );
                if !declaration {
                    return Err(self.error("expected a function, class, or interface after 'private'"));
                }
                self.advance();
                // The declared name: the first identifier after the keywords.
                let name = self.tokens[self.cursor..]
                    .iter()
                    .find_map(|token| match &token.kind {
                        TokenKind::Identifier(name) => Some(name.clone()),
                        _ => None,
                    })
                    .unwrap_or_default();
                self.private_names.insert(name);
                if matches!(self.current().kind, TokenKind::Function) && self.at_function_type() {
                    return Err(self.error("expected a function, class, or interface after 'private'"));
                }
            }
            if matches!(self.current().kind, TokenKind::Class | TokenKind::Abstract | TokenKind::Final) {
                let mut is_abstract = false;
                let mut is_final = false;
                while matches!(self.current().kind, TokenKind::Abstract | TokenKind::Final) {
                    let (flag, word) = if matches!(self.current().kind, TokenKind::Abstract) {
                        (&mut is_abstract, "abstract")
                    } else {
                        (&mut is_final, "final")
                    };
                    if *flag {
                        return Err(self.error(&format!("'{word}' is repeated")));
                    }
                    *flag = true;
                    self.advance();
                }
                if is_abstract && is_final {
                    return Err(self.error(
                        "a class can't be both abstract and final: abstract classes exist to be extended",
                    ));
                }
                if !matches!(self.current().kind, TokenKind::Class) {
                    return Err(self.error("expected 'class' after the class modifiers"));
                }
                self.parse_class(is_abstract, is_final)?;
            } else if matches!(self.current().kind, TokenKind::Interface) {
                self.parse_interface()?;
            } else if matches!(self.current().kind, TokenKind::Function) && !self.at_function_type() {
                functions.push(self.parse_function()?);
            } else {
                statements.extend(self.parse_statement()?);
            }
        }
        Ok((functions, statements))
    }

    /// Parses `extern function name(params): type;`, a function implemented in C as
    /// `qiq_stdlib_<name>`. The return type defaults to void.
    fn parse_extern_function(&mut self) -> Result<Function, String> {
        if !self.allow_extern {
            return Err(self.error("extern functions can only be declared in the standard library"));
        }
        self.advance();
        let start = self.cursor;
        let mut function = self.parse_function_member(None, Visibility::Public, true)?;
        let unsupported = function
            .parameters
            .iter()
            .map(|parameter| parameter.value_type)
            .chain(std::iter::once(function.return_type))
            .find(|value_type| {
                let scalar_array = self.element_of(*value_type).is_some_and(ValueType::is_scalar);
                !value_type.is_scalar() && !scalar_array && *value_type != ValueType::Void
            });
        if let Some(value_type) = unsupported {
            let token = &self.tokens[start];
            return Err(format!(
                "{}:{}: extern function '{}' uses {}; extern functions support only int, double, bool, byte, string, char, and one-dimensional arrays of these",
                token.line,
                token.column,
                function.name,
                self.type_name(value_type)
            ));
        }
        function.is_abstract = false;
        function.is_extern = true;
        function.symbol = format!("qiq_stdlib_{}", function.name);
        Ok(function)
    }

    /// `const [type] NAME = value;`, from the word `const`. The value is
    /// checked when the program is compiled (see constant_value).
    fn parse_constant(&mut self, visibility: Visibility) -> Result<Constant, String> {
        self.advance(); // const
        // A type, unless the name comes straight after `const`.
        let typed = !matches!(
            (self.tokens.get(self.cursor).map(|token| &token.kind), self.tokens.get(self.cursor + 1).map(|token| &token.kind)),
            (Some(TokenKind::Identifier(_)), Some(TokenKind::Equals))
        );
        let declared = if typed { Some(self.parse_value_type()?) } else { None };
        let (line, column) = (self.current().line, self.current().column);
        let name = self.expect_identifier("expected the constant's name")?;
        self.expect(TokenKind::Equals, "expected '=' and the constant's value")?;
        let value = self.parse_expression()?;
        self.expect(TokenKind::Semicolon, "expected ';' after the constant")?;
        Ok(Constant { name, declared, visibility, value, line, column })
    }

    /// Parses a parameter's default value: a literal of the parameter's type.
    fn parse_parameter_default(&mut self, value_type: ValueType, name: &str) -> Result<Expr, String> {
        let start = self.cursor;
        let error_at = |parser: &Self, message: String| {
            let token = &parser.tokens[start];
            format!("{}:{}: {message}", token.line, token.column)
        };
        let default = match self.parse_expression()? {
            Expr::Negate(operand) => match *operand {
                Expr::Decimal(value) => Expr::Decimal(-value),
                _ => return Err(error_at(self, format!("default for '{name}' must be a literal"))),
            },
            other => other,
        };
        let literal_type = match default {
            Expr::Number(_) => ValueType::Int,
            Expr::Decimal(_) => ValueType::Double,
            Expr::Boolean(_) => ValueType::Bool,
            Expr::Text(_) => ValueType::String,
            Expr::Character(_) => ValueType::Char,
            Expr::Null => ValueType::Null,
            _ => return Err(error_at(self, format!("default for '{name}' must be a literal"))),
        };
        if let (ValueType::Byte, Expr::Number(number)) = (value_type, &default) {
            if !(0..=255).contains(number) {
                return Err(error_at(self, format!("default for '{name}' must be a byte (0 to 255), found {number}")));
            }
            return Ok(default);
        }
        if literal_type == ValueType::Null && is_reference(value_type) {
            return Ok(default);
        }
        if literal_type != value_type {
            return Err(error_at(
                self,
                format!(
                    "default for '{name}' must be {}, found {}",
                    self.type_name(value_type),
                    self.type_name(literal_type)
                ),
            ));
        }
        Ok(default)
    }

    /// Points code still using the old `fn` keyword at `function`.
    fn reject_fn_keyword(&self) -> Result<(), String> {
        if matches!(&self.current().kind, TokenKind::Identifier(word) if word == "fn") {
            return Err(self.error("declare functions with 'function', not 'fn'"));
        }
        Ok(())
    }

    fn parse_function(&mut self) -> Result<Function, String> {
        self.parse_function_member(None, Visibility::Public, false)
    }

    fn parse_function_member(
        &mut self,
        owner_class: Option<u32>,
        visibility: Visibility,
        is_abstract: bool,
    ) -> Result<Function, String> {
        self.advance();
        let (line, column) = (self.current().line, self.current().column);
        let name = self.expect_identifier("expected function name")?;
        let name_for_symbol = self.qualified_symbol(&name, "qiq.");
        self.expect(TokenKind::LeftParen, "expected '(' after function name")?;
        let mut parameters = Vec::new();
        self.promoted.clear();
        let is_constructor = owner_class.is_some() && name == "__construct";
        let mut binds_values = false;
        if !matches!(self.current().kind, TokenKind::RightParen) {
            loop {
                if matches!(self.current().kind, TokenKind::Ellipsis) {
                    if !self.allow_extern || owner_class.is_none() || is_abstract || is_constructor {
                        return Err(self.error("'...' parameters are only for standard library methods"));
                    }
                    self.advance();
                    binds_values = true;
                    if !matches!(self.current().kind, TokenKind::RightParen) {
                        return Err(self.error("'...' must be the last parameter"));
                    }
                    break;
                }
                // `public`, `protected`, `private`, and `readonly` make a
                // constructor parameter a property too.
                let (modifier_line, modifier_column) = (self.current().line, self.current().column);
                let mut promoted_visibility = None;
                let mut promoted_readonly = false;
                loop {
                    match &self.current().kind {
                        TokenKind::Public | TokenKind::Protected | TokenKind::Private if promoted_visibility.is_none() => {
                            promoted_visibility = Some(match self.current().kind {
                                TokenKind::Public => Visibility::Public,
                                TokenKind::Protected => Visibility::Protected,
                                _ => Visibility::Private,
                            });
                        }
                        TokenKind::Identifier(word) if word == "readonly" && !promoted_readonly => promoted_readonly = true,
                        _ => break,
                    }
                    self.advance();
                }
                let promotes = promoted_visibility.is_some() || promoted_readonly;
                if promotes && !is_constructor {
                    return Err(format!(
                        "{modifier_line}:{modifier_column}: only a constructor's parameters can be properties too (with public, protected, private, or readonly)"
                    ));
                }
                if promotes && is_abstract {
                    return Err(format!("{modifier_line}:{modifier_column}: an abstract constructor's parameters can't be properties"));
                }
                let value_type = self.parse_value_type()?;
                let (parameter_line, parameter_column) = (self.current().line, self.current().column);
                let parameter_name = self.expect_identifier("expected parameter name")?;
                if promotes {
                    self.promoted.push(PromotedParameter {
                        name: parameter_name.clone(),
                        value_type,
                        visibility: promoted_visibility.unwrap_or(Visibility::Public),
                        readonly: promoted_readonly,
                        line: parameter_line,
                        column: parameter_column,
                    });
                }
                let default = if matches!(self.current().kind, TokenKind::Equals) {
                    self.advance();
                    Some(self.parse_parameter_default(value_type, &parameter_name)?)
                } else {
                    if let Some(optional) = parameters.iter().find(|p: &&Parameter| p.default.is_some()) {
                        return Err(self.error(&format!(
                            "required parameter '{parameter_name}' cannot follow optional parameter '{}'",
                            optional.name
                        )));
                    }
                    None
                };
                parameters.push(Parameter {
                    value_type,
                    name: parameter_name,
                    default,
                });
                if !matches!(self.current().kind, TokenKind::Comma) {
                    break;
                }
                self.advance();
            }
        }
        self.expect(TokenKind::RightParen, "expected ')' after parameters")?;
        let mut returns_static = false;
        let return_type = if matches!(self.current().kind, TokenKind::Colon) {
            self.advance();
            if matches!(self.current().kind, TokenKind::Static) {
                // `: static` returns the class the call was made on.
                let class_id = self
                    .current_class
                    .ok_or_else(|| self.error("the return type static is only valid in a class"))?;
                self.advance();
                returns_static = true;
                ValueType::Class(class_id)
            } else {
                self.parse_return_type()?
            }
        } else if is_abstract || (owner_class.is_some() && name == "__construct") {
            ValueType::Void
        } else {
            return Err(self.error("expected ':' before function return type"));
        };
        let mut body = Vec::new();
        if is_abstract {
            if name == "__construct" {
                return Err(self.error("constructors cannot be abstract"));
            }
            self.expect(TokenKind::Semicolon, "expected ';' after abstract method signature")?;
        } else {
            self.expect(TokenKind::LeftBrace, "expected '{' before function body")?;
            while !matches!(self.current().kind, TokenKind::RightBrace | TokenKind::End) {
                body.extend(self.parse_statement()?);
            }
            self.expect(TokenKind::RightBrace, "expected '}' after function body")?;
        }
        Ok(Function {
            name,
            parameters,
            return_type,
            body,
            visibility,
            is_abstract,
            is_final: false,
            is_static: false,
            returns_static,
            module: self.module,
            symbol: name_for_symbol,
            is_extern: false,
            binds_values,
            line,
            column,
        })
    }

    fn parse_class(&mut self, is_abstract: bool, is_final: bool) -> Result<(), String> {
        self.expect(TokenKind::Class, "expected 'class'")?;
        let (line, column) = (self.current().line, self.current().column);
        let name = self.expect_identifier("expected class name")?;
        self.check_class_name_is_free(&name)?;
        let parent_class = if matches!(self.current().kind, TokenKind::Extends) {
            self.advance();
            let parent_name = self.expect_identifier("expected parent class name after extends")?;
            let parent_id = *self.class_names.get(&parent_name).ok_or_else(|| {
                self.error(&format!("parent class '{parent_name}' must be declared first"))
            })?;
            if self.classes[parent_id as usize].is_interface {
                return Err(self.error(&format!(
                    "class '{name}' cannot extend interface '{parent_name}'; use implements"
                )));
            }
            if self.classes[parent_id as usize].is_final {
                return Err(self.error(&format!("class '{name}' cannot extend final class '{parent_name}'")));
            }
            Some(parent_id)
        } else {
            None
        };
        let interfaces = if matches!(self.current().kind, TokenKind::Implements) {
            self.advance();
            self.parse_interface_list(&name, "implements")?
        } else {
            Vec::new()
        };
        let class_id = self.classes.len() as u32;
        self.class_names.insert(name.clone(), class_id);
        self.current_class = Some(class_id);
        self.current_parent = parent_class;
        self.expect(TokenKind::LeftBrace, "expected '{' after class name")?;

        let mut properties = Vec::new();
        let mut static_properties = Vec::new();
        let mut constants: Vec<Constant> = Vec::new();
        let mut methods = Vec::new();
        while !matches!(self.current().kind, TokenKind::RightBrace | TokenKind::End) {
            // Modifiers, in any order: abstract, final, static, and one visibility.
            let mut is_abstract_method = false;
            let mut is_final_method = false;
            let mut is_static_method = false;
            let mut is_readonly = false;
            let mut explicit_visibility = None;
            loop {
                let repeated = match &self.current().kind {
                    TokenKind::Identifier(word) if word == "readonly" => std::mem::replace(&mut is_readonly, true),
                    TokenKind::Abstract => std::mem::replace(&mut is_abstract_method, true),
                    TokenKind::Final => std::mem::replace(&mut is_final_method, true),
                    TokenKind::Static => std::mem::replace(&mut is_static_method, true),
                    TokenKind::Public | TokenKind::Private | TokenKind::Protected => {
                        let visibility = match self.current().kind {
                            TokenKind::Public => Visibility::Public,
                            TokenKind::Private => Visibility::Private,
                            _ => Visibility::Protected,
                        };
                        explicit_visibility.replace(visibility).is_some()
                    }
                    _ => break,
                };
                if repeated {
                    return Err(self.error("a member modifier is repeated (or given two visibilities)"));
                }
                self.advance();
            }
            if is_readonly && (matches!(self.current().kind, TokenKind::Function) || matches!(&self.current().kind, TokenKind::Identifier(word) if word == "const")) {
                return Err(self.error("readonly can only modify a property"));
            }
            if matches!(&self.current().kind, TokenKind::Identifier(word) if word == "const") {
                if is_static_method || is_abstract_method || is_final_method {
                    return Err(self.error("a constant can't be static, abstract, or final: it's already fixed"));
                }
                let constant = self.parse_constant(explicit_visibility.unwrap_or(Visibility::Public))?;
                if constants.iter().any(|existing| existing.name == constant.name)
                    || static_properties.iter().any(|property: &StaticProperty| property.name == constant.name)
                {
                    return Err(format!("{}:{}: '{}' is already declared in class '{name}'", constant.line, constant.column, constant.name));
                }
                constants.push(constant);
                continue;
            }
            if matches!(self.current().kind, TokenKind::Function) && !self.at_function_type() {
                let start = self.error("");
                let mut method = self.parse_function_member(
                    Some(class_id),
                    explicit_visibility.unwrap_or(Visibility::Public),
                    is_abstract_method,
                )?;
                if is_final_method {
                    if is_abstract_method {
                        return Err(format!(
                            "{start}method '{}' can't be both abstract and final: abstract methods exist to be overridden",
                            method.name
                        ));
                    }
                    if method.visibility == Visibility::Private {
                        return Err(format!(
                            "{start}private method '{}' can't be final: private methods are never overridden",
                            method.name
                        ));
                    }
                }
                if is_static_method {
                    if is_abstract_method {
                        return Err(format!(
                            "{start}method '{}' can't be both abstract and static",
                            method.name
                        ));
                    }
                    if method.name == "__construct" {
                        return Err(format!("{start}a constructor can't be static"));
                    }
                }
                // One name, one method: no overloading, and a static and an
                // instance method can't share a name either.
                if methods.iter().any(|existing: &Function| existing.name == method.name) {
                    return Err(format!(
                        "{}:{}: class '{name}' declares method '{}' more than once",
                        method.line, method.column, method.name
                    ));
                }
                method.is_final = is_final_method;
                method.is_static = is_static_method;
                // Promoted parameters: properties, set first thing.
                let promoted = std::mem::take(&mut self.promoted);
                for (index, parameter) in promoted.iter().enumerate() {
                    if properties.iter().any(|property: &Property| property.name == parameter.name)
                        || static_properties.iter().any(|property: &StaticProperty| property.name == parameter.name)
                    {
                        return Err(format!(
                            "{}:{}: property '{}' is already declared",
                            parameter.line, parameter.column, parameter.name
                        ));
                    }
                    properties.push(Property {
                        name: parameter.name.clone(),
                        value_type: parameter.value_type,
                        visibility: parameter.visibility,
                        default: None,
                        default_at: (parameter.line, parameter.column),
                        readonly: parameter.readonly,
                    });
                    method.body.insert(
                        index,
                        Statement {
                            kind: StatementKind::AssignMember(
                                Expr::Variable("this".to_string()),
                                parameter.name.clone(),
                                Expr::Variable(parameter.name.clone()),
                            ),
                            line: parameter.line,
                            column: parameter.column,
                        },
                    );
                }
                methods.push(method);
            } else {
                self.reject_fn_keyword()?;
                if is_abstract_method {
                    return Err(self.error("abstract can only modify a method"));
                }
                if is_final_method {
                    return Err(self.error("final can only modify a class or a method"));
                }
                let value_type = self.parse_value_type()?;
                if value_type == ValueType::Void {
                    return Err(self.error("class properties cannot have type void"));
                }
                let property_name = self.expect_identifier("expected property name")?;
                if properties.iter().any(|property: &Property| property.name == property_name)
                    || static_properties.iter().any(|property: &StaticProperty| property.name == property_name)
                    || (is_static_method && constants.iter().any(|constant| constant.name == property_name))
                {
                    return Err(self.error(&format!("property '{property_name}' is already declared")));
                }
                if is_readonly && is_static_method {
                    return Err(self.error("a static property can't be readonly"));
                }
                if is_static_method {
                    let default = if matches!(self.current().kind, TokenKind::Equals) {
                        self.advance();
                        Some(self.parse_parameter_default(value_type, &property_name)?)
                    } else {
                        None
                    };
                    self.expect(TokenKind::Semicolon, "expected ';' after static property")?;
                    static_properties.push(StaticProperty {
                        name: property_name,
                        value_type,
                        visibility: explicit_visibility.unwrap_or(Visibility::Private),
                        default,
                    });
                    continue;
                }
                // A default: checked to be a constant of the right type when
                // the program is compiled (see check_constant_value).
                let default_at = (self.current().line, self.current().column);
                let default = if matches!(self.current().kind, TokenKind::Equals) {
                    if is_readonly {
                        return Err(self.error("a readonly property can't have a default: set it in the constructor"));
                    }
                    self.advance();
                    Some(self.parse_expression()?)
                } else {
                    None
                };
                self.expect(TokenKind::Semicolon, "expected ';' after class property")?;
                properties.push(Property {
                    name: property_name,
                    value_type,
                    visibility: explicit_visibility.unwrap_or(Visibility::Private),
                    default,
                    default_at,
                    readonly: is_readonly,
                });
            }
        }
        self.expect(TokenKind::RightBrace, "expected '}' after class body")?;
        self.current_class = None;
        self.current_parent = None;
        self.classes.push(ClassDef {
            parent_class,
            interfaces,
            is_abstract,
            is_final,
            is_interface: false,
            module: self.module,
            symbol: self.qualified_symbol(&name, ""),
            name,
            properties,
            static_properties,
            constants,
            methods,
            line,
            column,
        });
        Ok(())
    }

    /// A class or interface name must not repeat a declared, imported, or
    /// built-in one.
    fn check_class_name_is_free(&self, name: &str) -> Result<(), String> {
        match self.class_names.get(name) {
            None => Ok(()),
            Some(&existing) if self.classes[existing as usize].module == self.module => {
                Err(self.error(&format!("class '{name}' is already declared")))
            }
            Some(_) => Err(self.error(&format!(
                "class '{name}' is already imported or built in"
            ))),
        }
    }

    fn parse_interface_list(&mut self, owner: &str, keyword: &str) -> Result<Vec<u32>, String> {
        let mut interfaces = Vec::new();
        loop {
            let interface_name =
                self.expect_identifier(&format!("expected interface name after {keyword}"))?;
            let interface_id = *self.class_names.get(&interface_name).ok_or_else(|| {
                self.error(&format!("interface '{interface_name}' must be declared first"))
            })?;
            if !self.classes[interface_id as usize].is_interface {
                return Err(self.error(&format!("'{interface_name}' is a class, not an interface")));
            }
            if interfaces.contains(&interface_id) {
                return Err(self.error(&format!(
                    "'{owner}' lists interface '{interface_name}' more than once"
                )));
            }
            interfaces.push(interface_id);
            if !matches!(self.current().kind, TokenKind::Comma) {
                return Ok(interfaces);
            }
            self.advance();
        }
    }

    fn parse_interface(&mut self) -> Result<(), String> {
        self.expect(TokenKind::Interface, "expected 'interface'")?;
        let (line, column) = (self.current().line, self.current().column);
        let name = self.expect_identifier("expected interface name")?;
        self.check_class_name_is_free(&name)?;
        let interfaces = if matches!(self.current().kind, TokenKind::Extends) {
            self.advance();
            self.parse_interface_list(&name, "extends")?
        } else {
            Vec::new()
        };
        let interface_id = self.classes.len() as u32;
        self.class_names.insert(name.clone(), interface_id);
        self.expect(TokenKind::LeftBrace, "expected '{' after interface name")?;

        let mut methods: Vec<Function> = Vec::new();
        let mut constants: Vec<Constant> = Vec::new();
        while !matches!(self.current().kind, TokenKind::RightBrace | TokenKind::End) {
            let at_const = |parser: &Self| matches!(&parser.current().kind, TokenKind::Identifier(word) if word == "const");
            if at_const(self) || (matches!(self.current().kind, TokenKind::Public) && matches!(self.tokens.get(self.cursor + 1).map(|token| &token.kind), Some(TokenKind::Identifier(word)) if word == "const")) {
                if matches!(self.current().kind, TokenKind::Public) {
                    self.advance();
                }
                let constant = self.parse_constant(Visibility::Public)?;
                if constants.iter().any(|existing| existing.name == constant.name) {
                    return Err(format!("{}:{}: '{}' is already declared in interface '{name}'", constant.line, constant.column, constant.name));
                }
                constants.push(constant);
                continue;
            }
            if matches!(self.current().kind, TokenKind::Private | TokenKind::Protected)
                && matches!(self.tokens.get(self.cursor + 1).map(|token| &token.kind), Some(TokenKind::Identifier(word)) if word == "const")
            {
                return Err(self.error("interface constants must be public"));
            }
            match self.current().kind {
                TokenKind::Public => self.advance(),
                TokenKind::Private | TokenKind::Protected => {
                    return Err(self.error("interface methods must be public"));
                }
                TokenKind::Abstract => {
                    return Err(self.error("interface methods are implicitly abstract"));
                }
                TokenKind::Final => {
                    return Err(self.error("interface methods can't be final: they exist to be implemented"));
                }
                TokenKind::Static => {
                    return Err(self.error("interface methods can't be static"));
                }
                _ => {}
            }
            self.reject_fn_keyword()?;
            if !matches!(self.current().kind, TokenKind::Function) {
                return Err(self.error("interfaces can only declare method signatures and constants"));
            }
            if matches!(
                self.tokens.get(self.cursor + 1).map(|token| &token.kind),
                Some(TokenKind::Identifier(method)) if method == "__construct"
            ) {
                return Err(self.error("interfaces cannot declare constructors"));
            }
            let method = self.parse_function_member(Some(interface_id), Visibility::Public, true)?;
            if methods.iter().any(|existing| existing.name == method.name) {
                return Err(self.error(&format!(
                    "interface '{name}' declares method '{}' more than once",
                    method.name
                )));
            }
            methods.push(method);
        }
        self.expect(TokenKind::RightBrace, "expected '}' after interface body")?;
        self.classes.push(ClassDef {
            symbol: self.qualified_symbol(&name, ""),
            name,
            parent_class: None,
            interfaces,
            is_abstract: true,
            is_final: false,
            is_interface: true,
            module: self.module,
            properties: Vec::new(),
            static_properties: Vec::new(),
            constants,
            methods,
            line,
            column,
        });
        Ok(())
    }

    /// One statement as written, as the statements it compiles to: usually
    /// one, but a compound assignment can first save parts of its target in
    /// hidden variables (see parse_update).
    fn parse_statement(&mut self) -> Result<Vec<Statement>, String> {
        let (line, column) = (self.current().line, self.current().column);
        let mut kinds = Vec::new();
        if matches!(self.current().kind, TokenKind::PlusPlus | TokenKind::MinusMinus) {
            // `++count;`: the same as `count++;`.
            let op = if matches!(self.current().kind, TokenKind::PlusPlus) { BinaryOp::Add } else { BinaryOp::Subtract };
            self.advance();
            let target = self.parse_expression()?;
            self.expect(TokenKind::Semicolon, "expected ';' after the incremented value")?;
            self.parse_update(target, op, Expr::One, (line, column), &mut kinds)?;
        } else {
            let kind = self.parse_statement_kind(&mut kinds)?;
            kinds.push(kind);
        }
        Ok(kinds.into_iter().map(|kind| Statement { kind, line, column }).collect())
    }

    /// `target op= value` (and `target++`, with Expr::One) as a plain
    /// assignment, `target = target op value`, so it's checked and compiled
    /// like one. Each part of the target is evaluated once: parts that could
    /// run code (`items[next()]`) are saved in hidden variables first, and so
    /// are all of them when `value` could, so that the target is read, then
    /// `value` evaluated, then the result stored. The statements that save
    /// parts go into `before`; the result is the assignment.
    fn update_target(&mut self, target: Expr, op: BinaryOp, value: Expr, at: (usize, usize), before: &mut Vec<StatementKind>) -> Result<StatementKind, String> {
        let force = !is_pure(&value);
        let increments = matches!(value, Expr::One);
        let hoist = |this: &mut Self, part: Expr, before: &mut Vec<StatementKind>| -> Expr {
            let literal = matches!(part, Expr::Number(_) | Expr::Decimal(_) | Expr::Boolean(_) | Expr::Null | Expr::Text(_) | Expr::Character(_));
            if literal || (is_pure(&part) && !force) {
                return part;
            }
            this.next_hidden += 1;
            // Not a name the program can write.
            let name = format!("#{}", this.next_hidden);
            before.push(StatementKind::Assign(name.clone(), part));
            Expr::Variable(name)
        };
        let combined = |current: Expr| Expr::Binary(op, Box::new(current), Box::new(value));
        match target {
            Expr::Variable(name) => Ok(StatementKind::Assign(name.clone(), combined(Expr::Variable(name)))),
            Expr::Index(container, key) => {
                let container = hoist(self, *container, before);
                let key = hoist(self, *key, before);
                let current = Expr::Index(Box::new(container.clone()), Box::new(key.clone()));
                Ok(StatementKind::AssignIndex(container, key, combined(current)))
            }
            Expr::Member(object, property_name) => {
                let object = hoist(self, *object, before);
                let current = Expr::Member(Box::new(object.clone()), property_name.clone());
                Ok(StatementKind::AssignMember(object, property_name, combined(current)))
            }
            Expr::StaticProperty(target, property_name) => {
                let current = Expr::StaticProperty(target.clone(), property_name.clone());
                Ok(StatementKind::AssignStatic(target, property_name, combined(current)))
            }
            _ => Err(format!(
                "{}:{}: {}",
                at.0,
                at.1,
                if increments {
                    "only variables, indexed elements, or properties can be incremented or decremented"
                } else {
                    "only variables, indexed elements, or properties can be assigned"
                }
            )),
        }
    }

    /// parse_update's work, pushing every statement into `kinds`.
    fn parse_update(&mut self, target: Expr, op: BinaryOp, value: Expr, at: (usize, usize), kinds: &mut Vec<StatementKind>) -> Result<(), String> {
        let assignment = self.update_target(target, op, value, at, kinds)?;
        kinds.push(assignment);
        Ok(())
    }

    fn parse_statement_kind(&mut self, before: &mut Vec<StatementKind>) -> Result<StatementKind, String> {
        if self.at_function_type() || self.at_grouped_type_declaration() {
            // A variable of function type: function(int): int twice = ...;
            let value_type = self.parse_value_type()?;
            return self.parse_declaration(value_type);
        }
        match self.current().kind.clone() {
            TokenKind::Int
            | TokenKind::Double
            | TokenKind::Bool
            | TokenKind::String
            | TokenKind::CharType
            | TokenKind::ByteType
            | TokenKind::Map => {
                let value_type = self.parse_value_type()?;
                self.parse_declaration(value_type)
            }
            TokenKind::Identifier(ref type_name)
                if self.class_names.contains_key(type_name)
                    && !matches!(
                        self.tokens.get(self.cursor + 1).map(|token| &token.kind),
                        Some(TokenKind::DoubleColon)
                    ) =>
            {
                let value_type = self.parse_value_type()?;
                self.parse_declaration(value_type)
            }
            TokenKind::Print | TokenKind::PrintLine => {
                let lines = matches!(self.current().kind, TokenKind::PrintLine);
                let name = if lines { "println" } else { "print" };
                self.advance();
                self.expect(TokenKind::LeftParen, &format!("expected '(' after '{name}'"))?;
                let mut values = Vec::new();
                if !matches!(self.current().kind, TokenKind::RightParen) {
                    loop {
                        values.push(self.parse_expression()?);
                        if !matches!(self.current().kind, TokenKind::Comma) {
                            break;
                        }
                        self.advance();
                    }
                }
                self.expect(TokenKind::RightParen, &format!("expected ')' after {name} arguments"))?;
                self.expect(TokenKind::Semicolon, &format!("expected ';' after {name} statement"))?;
                Ok(StatementKind::Print(values, lines))
            }
            TokenKind::Throw => {
                self.advance();
                let value = self.parse_expression()?;
                self.expect(TokenKind::Semicolon, "expected ';' after throw")?;
                Ok(StatementKind::Throw(value))
            }
            TokenKind::Try => self.parse_try(),
            TokenKind::Return => {
                self.advance();
                let value = if matches!(self.current().kind, TokenKind::Semicolon) {
                    None
                } else {
                    Some(self.parse_expression()?)
                };
                self.expect(TokenKind::Semicolon, "expected ';' after return")?;
                Ok(StatementKind::Return(value))
            }
            TokenKind::Break => {
                self.advance();
                self.expect(TokenKind::Semicolon, "expected ';' after break")?;
                Ok(StatementKind::Break)
            }
            TokenKind::Continue => {
                self.advance();
                self.expect(TokenKind::Semicolon, "expected ';' after continue")?;
                Ok(StatementKind::Continue)
            }
            TokenKind::If => self.parse_if(before),
            TokenKind::Foreach => self.parse_foreach(),
            TokenKind::While => self.parse_while(),
            TokenKind::Switch => self.parse_switch(),
            // `Name name ...` where Name is not a known class: report the type,
            // which is usually a missing import.
            TokenKind::Identifier(ref type_name)
                if matches!(
                    self.tokens.get(self.cursor + 1).map(|token| &token.kind),
                    Some(TokenKind::Identifier(_))
                ) =>
            {
                if type_name == "fn" {
                    return Err(self.error("declare functions with 'function', not 'fn'"));
                }
                Err(self.error(&format!("type '{type_name}' is not declared or imported{}", stdlib_hint(type_name))))
            }
            TokenKind::Identifier(_) | TokenKind::Parent | TokenKind::Static | TokenKind::LeftParen => {
                let at = (self.current().line, self.current().column);
                let expression = self.parse_expression()?;
                if matches!(expression, Expr::Nullsafe(..))
                    && matches!(self.current().kind, TokenKind::AssignOp(_) | TokenKind::Equals | TokenKind::PlusPlus | TokenKind::MinusMinus)
                {
                    return Err(format!("{}:{}: can't assign through '?->': check for null with if, then assign", at.0, at.1));
                }
                if let TokenKind::AssignOp(op) = self.current().kind {
                    // `total += price;`
                    self.advance();
                    let value = self.parse_expression()?;
                    if matches!(self.current().kind, TokenKind::PlusPlus | TokenKind::MinusMinus) {
                        return Err(self.error(INCREMENT_IS_A_STATEMENT));
                    }
                    self.expect(TokenKind::Semicolon, "expected ';' after assignment")?;
                    return self.update_target(expression, op, value, at, before);
                }
                if matches!(self.current().kind, TokenKind::PlusPlus | TokenKind::MinusMinus) {
                    // `count++;`
                    let op = if matches!(self.current().kind, TokenKind::PlusPlus) { BinaryOp::Add } else { BinaryOp::Subtract };
                    self.advance();
                    self.expect(TokenKind::Semicolon, "expected ';' after '++' or '--'")?;
                    return self.update_target(expression, op, Expr::One, at, before);
                }
                if matches!(self.current().kind, TokenKind::Equals) {
                    self.advance();
                    let value = self.parse_expression()?;
                    if matches!(self.current().kind, TokenKind::PlusPlus | TokenKind::MinusMinus) {
                        return Err(self.error(INCREMENT_IS_A_STATEMENT));
                    }
                    self.expect(TokenKind::Semicolon, "expected ';' after assignment")?;
                    if let Expr::Variable(name) = expression {
                        Ok(StatementKind::Assign(name, value))
                    } else if let Expr::Index(array, index) = expression {
                        Ok(StatementKind::AssignIndex(*array, *index, value))
                    } else if let Expr::Member(object, property_name) = expression {
                        Ok(StatementKind::AssignMember(*object, property_name, value))
                    } else if let Expr::StaticProperty(target, property_name) = expression {
                        Ok(StatementKind::AssignStatic(target, property_name, value))
                    } else {
                        Err(self.error("only variables, indexed elements, or properties can be assigned"))
                    }
                } else {
                    self.expect(TokenKind::Semicolon, "expected ';' after expression")?;
                    Ok(StatementKind::Expression(expression))
                }
            }
            _ => Err(self.error("expected a declaration, print, function call, or return")),
        }
    }

    fn parse_if(&mut self, before: &mut Vec<StatementKind>) -> Result<StatementKind, String> {
        let at = (self.current().line, self.current().column);
        self.advance();
        self.expect(TokenKind::LeftParen, "expected '(' after 'if'")?;
        let condition = self.parse_expression()?;
        self.expect(TokenKind::RightParen, "expected ')' after condition")?;
        let then_body = self.parse_statement_block("expected '{' before if body")?;
        let else_body = if matches!(self.current().kind, TokenKind::Else) {
            self.advance();
            // `else if (...) { ... }` chains: the if is the else branch.
            if matches!(self.current().kind, TokenKind::If) {
                self.parse_statement()?
            } else {
                self.parse_statement_block("expected '{' before else body")?
            }
        } else {
            Vec::new()
        };
        self.bind_instanceof(condition, then_body, else_body, at, before)
    }

    /// `if (... && value instanceof Class name && ...) { then } else { else }`
    /// as statements without bindings. On its own, a binding just declares
    /// `Class name` from the checked value at the start of the then branch.
    /// Joined with other conditions by &&, the condition is worked out first,
    /// in nested ifs (so && still stops at the first false part): each
    /// binding is declared for the parts after it and saved in a hidden
    /// variable, and a hidden flag records whether every part held. Then
    /// `if (flag) { Class name = saved; then } else { else }`, an ordinary
    /// if-else, so that a function returning in both branches is seen to
    /// return. The value checked is evaluated once. Statements that go
    /// before the result are added to `before`.
    fn bind_instanceof(
        &mut self,
        condition: Expr,
        then_body: Vec<Statement>,
        else_body: Vec<Statement>,
        at: (usize, usize),
        before: &mut Vec<StatementKind>,
    ) -> Result<StatementKind, String> {
        fn flatten(condition: Expr, parts: &mut Vec<Expr>) {
            match condition {
                Expr::Logical(LogicalOp::And, left, right) => {
                    flatten(*left, parts);
                    flatten(*right, parts);
                }
                other => parts.push(other),
            }
        }
        let statement = |kind: StatementKind| Statement { kind, line: at.0, column: at.1 };
        let mut parts = Vec::new();
        flatten(condition, &mut parts);
        if !parts.iter().any(|part| matches!(part, Expr::InstanceOf(_, _, Some(_)))) {
            let condition = parts
                .into_iter()
                .reduce(|left, right| Expr::Logical(LogicalOp::And, Box::new(left), Box::new(right)))
                .expect("a condition has a part");
            return Ok(StatementKind::If { condition, then_body, else_body });
        }
        // Only a binding: check, and declare the name in the then branch.
        if parts.len() == 1 {
            let Some(Expr::InstanceOf(value, class_id, Some(name))) = parts.pop() else { unreachable!() };
            let checked = self.evaluate_once(*value, before);
            let mut then_with_binding = vec![statement(StatementKind::Declare(
                ValueType::Class(class_id),
                name,
                Expr::Downcast(Box::new(checked.clone()), class_id),
            ))];
            then_with_binding.extend(then_body);
            return Ok(StatementKind::If {
                condition: Expr::InstanceOf(Box::new(checked), class_id, None),
                then_body: then_with_binding,
                else_body,
            });
        }
        // The flag, and a saved object for each binding.
        self.next_hidden += 1;
        let flag = format!("#{}", self.next_hidden);
        before.push(StatementKind::Declare(ValueType::Bool, flag.clone(), Expr::Boolean(false)));
        let mut bindings: Vec<(String, u32, String)> = Vec::new();   // name, class, saved in
        for part in &parts {
            if let Expr::InstanceOf(_, class_id, Some(name)) = part {
                self.next_hidden += 1;
                let saved = format!("#{}", self.next_hidden);
                before.push(StatementKind::Declare(ValueType::Class(*class_id), saved.clone(), Expr::Null));
                bindings.push((name.clone(), *class_id, saved));
            }
        }
        // Innermost first: when every part held, set the flag.
        let mut inner = vec![statement(StatementKind::Assign(flag.clone(), Expr::Boolean(true)))];
        let mut binding_index = bindings.len();
        for part in parts.into_iter().rev() {
            let kind = match part {
                Expr::InstanceOf(value, class_id, Some(name)) => {
                    binding_index -= 1;
                    let saved = bindings[binding_index].2.clone();
                    let mut steps = Vec::new();
                    let checked = self.evaluate_once(*value, &mut steps);
                    let mut bound = vec![
                        statement(StatementKind::Declare(
                            ValueType::Class(class_id),
                            name.clone(),
                            Expr::Downcast(Box::new(checked.clone()), class_id),
                        )),
                        statement(StatementKind::Assign(saved, Expr::Variable(name))),
                    ];
                    bound.extend(inner);
                    steps.push(StatementKind::If {
                        condition: Expr::InstanceOf(Box::new(checked), class_id, None),
                        then_body: bound,
                        else_body: Vec::new(),
                    });
                    inner = steps.into_iter().map(statement).collect();
                    continue;
                }
                condition => StatementKind::If { condition, then_body: inner, else_body: Vec::new() },
            };
            inner = vec![statement(kind)];
        }
        before.extend(inner.into_iter().map(|statement| statement.kind));
        // The branches, with the names bound from the saved objects.
        let mut then_with_bindings: Vec<Statement> = bindings
            .into_iter()
            .map(|(name, class_id, saved)| statement(StatementKind::Declare(ValueType::Class(class_id), name, Expr::Variable(saved))))
            .collect();
        then_with_bindings.extend(then_body);
        Ok(StatementKind::If { condition: Expr::Variable(flag), then_body: then_with_bindings, else_body })
    }

    /// `value` as an expression that can be evaluated more than once: itself
    /// if evaluating it has no effects, otherwise a hidden variable it's
    /// saved in first (by a statement added to `steps`).
    fn evaluate_once(&mut self, value: Expr, steps: &mut Vec<StatementKind>) -> Expr {
        if is_pure(&value) {
            return value;
        }
        self.next_hidden += 1;
        let hidden = format!("#{}", self.next_hidden);
        steps.push(StatementKind::Assign(hidden.clone(), value));
        Expr::Variable(hidden)
    }

    fn parse_try(&mut self) -> Result<StatementKind, String> {
        self.advance();
        let body = self.parse_statement_block("expected '{' after 'try'")?;
        let mut catches = Vec::new();
        while matches!(self.current().kind, TokenKind::Catch) {
            self.advance();
            self.expect(TokenKind::LeftParen, "expected '(' after 'catch'")?;
            let class_name = self.expect_identifier("expected an exception class in catch")?;
            let class_id = *self.class_names.get(&class_name).ok_or_else(|| {
                self.error(&format!("class '{class_name}' is not declared or imported{}", stdlib_hint(&class_name)))
            })?;
            let name = self.expect_identifier("expected a variable name after the exception class")?;
            self.expect(TokenKind::RightParen, "expected ')' after catch variable")?;
            let body = self.parse_statement_block("expected '{' before catch body")?;
            catches.push(CatchClause { class_id, name, body });
        }
        let finally = if matches!(self.current().kind, TokenKind::Finally) {
            self.advance();
            Some(self.parse_statement_block("expected '{' after 'finally'")?)
        } else {
            None
        };
        if catches.is_empty() && finally.is_none() {
            return Err(self.error("try needs at least one catch or a finally block"));
        }
        Ok(StatementKind::Try { body, catches, finally })
    }

    /// The rest of a declaration after its type: `name;` (arrays and maps
    /// only) or `name = value;`.
    fn parse_declaration(&mut self, value_type: ValueType) -> Result<StatementKind, String> {
        if self.element_of(value_type).is_some() {
            let name = self.expect_identifier("expected array variable name")?;
            let initializer = if matches!(self.current().kind, TokenKind::Semicolon) {
                self.advance();
                ArrayInitializer::Uninitialized
            } else {
                self.expect(TokenKind::Equals, "expected '=' or ';' after array name")?;
                if matches!(self.current().kind, TokenKind::LeftBrace) {
                    return Err(self.error("array literals use [...], e.g. int[] values = [1, 2];"));
                }
                let value = self.parse_expression()?;
                self.expect(TokenKind::Semicolon, "expected ';' after array declaration")?;
                ArrayInitializer::Existing(value)
            };
            return Ok(StatementKind::DeclareArray(value_type, name, initializer));
        }
        if matches!(value_type, ValueType::Map(_)) {
            let name = self.expect_identifier("expected map variable name")?;
            let initializer = if matches!(self.current().kind, TokenKind::Semicolon) {
                self.advance();
                MapInitializer::Empty
            } else {
                self.expect(TokenKind::Equals, "expected '=' or ';' after map name")?;
                let value = self.parse_expression()?;
                self.expect(TokenKind::Semicolon, "expected ';' after map declaration")?;
                MapInitializer::Existing(value)
            };
            return Ok(StatementKind::DeclareMap(value_type, name, initializer));
        }
        let name = self.expect_identifier("expected variable name")?;
        self.expect(TokenKind::Equals, "expected '=' after variable name")?;
        let value = self.parse_expression()?;
        if matches!(self.current().kind, TokenKind::PlusPlus | TokenKind::MinusMinus) {
            return Err(self.error(INCREMENT_IS_A_STATEMENT));
        }
        self.expect(TokenKind::Semicolon, "expected ';' after declaration")?;
        Ok(StatementKind::Declare(value_type, name, value))
    }

    fn parse_switch(&mut self) -> Result<StatementKind, String> {
        self.advance();
        self.expect(TokenKind::LeftParen, "expected '(' after 'switch'")?;
        let subject = self.parse_expression()?;
        self.expect(TokenKind::RightParen, "expected ')' after switch value")?;
        self.expect(TokenKind::LeftBrace, "expected '{' before switch cases")?;
        let mut cases = Vec::new();
        let mut default = None;
        while !matches!(self.current().kind, TokenKind::RightBrace | TokenKind::End) {
            let label_error = self.error("");
            let values = match self.current().kind {
                TokenKind::Case => {
                    self.advance();
                    let mut values = vec![self.parse_expression()?];
                    while matches!(self.current().kind, TokenKind::Comma) {
                        self.advance();
                        values.push(self.parse_expression()?);
                    }
                    Some(values)
                }
                TokenKind::Default => {
                    if default.is_some() {
                        return Err(self.error("a switch can have only one default"));
                    }
                    self.advance();
                    None
                }
                _ => return Err(self.error("expected 'case' or 'default' in switch")),
            };
            self.expect(TokenKind::Colon, "expected ':' after case values")?;
            let mut body = Vec::new();
            while !matches!(
                self.current().kind,
                TokenKind::Case | TokenKind::Default | TokenKind::RightBrace | TokenKind::End
            ) {
                body.extend(self.parse_statement()?);
            }
            match values {
                Some(values) => {
                    if body.is_empty() {
                        // Usually PHP-style stacked cases, which would silently do nothing.
                        return Err(format!(
                            "{}an empty case does nothing because cases don't fall through; list several values in one case (`case 1, 2:`), or write `break;` to do nothing",
                            label_error
                        ));
                    }
                    cases.push(SwitchCase { values, body });
                }
                None => default = Some(body),
            }
        }
        self.expect(TokenKind::RightBrace, "expected '}' after switch cases")?;
        if cases.is_empty() && default.is_none() {
            return Err(self.error("a switch needs at least one case"));
        }
        Ok(StatementKind::Switch { subject, cases, default })
    }

    fn parse_while(&mut self) -> Result<StatementKind, String> {
        self.advance();
        self.expect(TokenKind::LeftParen, "expected '(' after 'while'")?;
        let condition = self.parse_expression()?;
        self.expect(TokenKind::RightParen, "expected ')' after while condition")?;
        let body = self.parse_statement_block("expected '{' before while body")?;
        Ok(StatementKind::While { condition, body })
    }

    fn parse_foreach(&mut self) -> Result<StatementKind, String> {
        self.advance();
        self.expect(TokenKind::LeftParen, "expected '(' after 'foreach'")?;
        let iterable = self.parse_expression()?;
        self.expect(TokenKind::As, "expected 'as' after foreach collection")?;
        let first = self.expect_identifier("expected foreach variable")?;
        let (key_name, value_name) = if matches!(self.current().kind, TokenKind::FatArrow) {
            self.advance();
            let second = self.expect_identifier("expected foreach value variable")?;
            (Some(first), second)
        } else {
            (None, first)
        };
        self.expect(TokenKind::RightParen, "expected ')' after foreach bindings")?;
        let body = self.parse_statement_block("expected '{' before foreach body")?;
        Ok(StatementKind::Foreach {
            iterable,
            key_name,
            value_name,
            body,
        })
    }

    fn parse_statement_block(&mut self, message: &str) -> Result<Vec<Statement>, String> {
        self.expect(TokenKind::LeftBrace, message)?;
        let mut statements = Vec::new();
        while !matches!(self.current().kind, TokenKind::RightBrace | TokenKind::End) {
            statements.extend(self.parse_statement()?);
        }
        self.expect(TokenKind::RightBrace, "expected '}' after block")?;
        Ok(statements)
    }

    /// Parses an expression. Precedence, lowest first: `??` (right-
    /// associative, as in PHP), `||`, `&&`, `== !=`,
    /// `< > <= >=`, `|`, `^`, `&`, `<< >>`, `+ -`, `* / %`, unary `-` `~` `!`,
    /// then `**` (right-associative), so `-2 ** 2` is `-4`. Bitwise operators
    /// bind tighter than comparisons (as in Rust and Python, unlike C and PHP),
    /// so `x & 1 == 0` means `(x & 1) == 0`.
    fn parse_expression(&mut self) -> Result<Expr, String> {
        let left = self.parse_or()?;
        if matches!(self.current().kind, TokenKind::QuestionQuestion) {
            self.advance();
            let right = self.parse_expression()?;
            return Ok(Expr::Coalesce(Box::new(left), Box::new(right)));
        }
        Ok(left)
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut expression = self.parse_and()?;
        while matches!(self.current().kind, TokenKind::OrOr) {
            self.advance();
            let right = self.parse_and()?;
            expression = Expr::Logical(LogicalOp::Or, Box::new(expression), Box::new(right));
        }
        Ok(expression)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut expression = self.parse_equality()?;
        while matches!(self.current().kind, TokenKind::AndAnd) {
            self.advance();
            let right = self.parse_equality()?;
            expression = Expr::Logical(LogicalOp::And, Box::new(expression), Box::new(right));
        }
        Ok(expression)
    }

    fn parse_equality(&mut self) -> Result<Expr, String> {
        let mut expression = self.parse_relational()?;
        loop {
            let operator = match self.current().kind {
                TokenKind::EqualEqual => CompareOp::Equal,
                TokenKind::BangEqual => CompareOp::NotEqual,
                _ => return Ok(expression),
            };
            self.advance();
            let right = self.parse_relational()?;
            expression = Expr::Compare(operator, Box::new(expression), Box::new(right));
        }
    }

    /// `a < b < c` is rejected: relational comparisons don't chain.
    fn parse_relational(&mut self) -> Result<Expr, String> {
        let expression = self.parse_bit_or()?;
        let operator = match self.current().kind {
            TokenKind::Less => CompareOp::Less,
            TokenKind::Greater => CompareOp::Greater,
            TokenKind::LessEqual => CompareOp::LessEqual,
            TokenKind::GreaterEqual => CompareOp::GreaterEqual,
            _ => return Ok(expression),
        };
        self.advance();
        let right = self.parse_bit_or()?;
        if matches!(
            self.current().kind,
            TokenKind::Less | TokenKind::Greater | TokenKind::LessEqual | TokenKind::GreaterEqual
        ) {
            return Err(self.error("comparisons cannot be chained; combine them with && or ||"));
        }
        Ok(Expr::Compare(operator, Box::new(expression), Box::new(right)))
    }

    /// Parses one left-associative binary level: operands come from `next`,
    /// and `operator_for` maps the current token to this level's operator.
    fn parse_binary_level(
        &mut self,
        next: fn(&mut Self) -> Result<Expr, String>,
        operator_for: fn(&TokenKind) -> Option<BinaryOp>,
    ) -> Result<Expr, String> {
        let mut expression = next(self)?;
        while let Some(operator) = operator_for(&self.current().kind) {
            self.advance();
            let right = next(self)?;
            expression = Expr::Binary(operator, Box::new(expression), Box::new(right));
        }
        Ok(expression)
    }

    fn parse_bit_or(&mut self) -> Result<Expr, String> {
        self.parse_binary_level(Self::parse_bit_xor, |kind| {
            matches!(kind, TokenKind::Pipe).then_some(BinaryOp::BitOr)
        })
    }

    fn parse_bit_xor(&mut self) -> Result<Expr, String> {
        self.parse_binary_level(Self::parse_bit_and, |kind| {
            matches!(kind, TokenKind::Caret).then_some(BinaryOp::BitXor)
        })
    }

    fn parse_bit_and(&mut self) -> Result<Expr, String> {
        self.parse_binary_level(Self::parse_shift, |kind| {
            matches!(kind, TokenKind::Ampersand).then_some(BinaryOp::BitAnd)
        })
    }

    fn parse_shift(&mut self) -> Result<Expr, String> {
        self.parse_binary_level(Self::parse_additive, |kind| match kind {
            TokenKind::ShiftLeft => Some(BinaryOp::ShiftLeft),
            TokenKind::ShiftRight => Some(BinaryOp::ShiftRight),
            _ => None,
        })
    }

    fn parse_additive(&mut self) -> Result<Expr, String> {
        let mut expression = self.parse_multiplicative()?;
        loop {
            let operator = match self.current().kind {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Subtract,
                _ => return Ok(expression),
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            expression = Expr::Binary(operator, Box::new(expression), Box::new(right));
        }
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, String> {
        let mut expression = self.parse_unary()?;
        loop {
            let operator = match self.current().kind {
                TokenKind::Star => BinaryOp::Multiply,
                TokenKind::Slash => BinaryOp::Divide,
                TokenKind::Percent => BinaryOp::Modulo,
                _ => return Ok(expression),
            };
            self.advance();
            let right = self.parse_unary()?;
            expression = Expr::Binary(operator, Box::new(expression), Box::new(right));
        }
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        if matches!(self.current().kind, TokenKind::PlusPlus | TokenKind::MinusMinus) {
            return Err(self.error(INCREMENT_IS_A_STATEMENT));
        }
        if matches!(self.current().kind, TokenKind::Tilde) {
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(Expr::BitNot(Box::new(operand)));
        }
        if matches!(self.current().kind, TokenKind::Bang) {
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(Expr::Not(Box::new(operand)));
        }
        if !matches!(self.current().kind, TokenKind::Minus) {
            let operand = self.parse_power()?;
            return self.parse_instanceof(operand);
        }
        self.advance();
        // A negative literal is folded into the number, which is how the
        // smallest int (-2147483648) can be written. `-2 ** 2` is not folded.
        if let TokenKind::Number(value) = self.current().kind {
            let next = self.tokens.get(self.cursor + 1).map(|token| &token.kind);
            if !matches!(next, Some(TokenKind::StarStar | TokenKind::LeftBracket | TokenKind::Arrow)) {
                let value = i64::try_from(-(value as i128))
                    .map_err(|_| self.error("integer literal is outside the int range"))?;
                self.advance();
                return Ok(Expr::Number(value));
            }
        }
        let operand = self.parse_unary()?;
        Ok(Expr::Negate(Box::new(operand)))
    }

    /// `operand instanceof Class [name]`, if that follows.
    fn parse_instanceof(&mut self, operand: Expr) -> Result<Expr, String> {
        if !matches!(&self.current().kind, TokenKind::Identifier(word) if word == "instanceof") {
            return Ok(operand);
        }
        self.advance();
        let class_name = self.expect_identifier("expected a class or interface name after instanceof")?;
        let class_id = *self.class_names.get(&class_name).ok_or_else(|| {
            self.error(&format!("type '{class_name}' is not declared or imported{}", stdlib_hint(&class_name)))
        })?;
        let binding = match &self.current().kind {
            TokenKind::Identifier(name) if name != "instanceof" => {
                let name = name.clone();
                self.advance();
                Some(name)
            }
            _ => None,
        };
        Ok(Expr::InstanceOf(Box::new(operand), class_id, binding))
    }

    fn parse_power(&mut self) -> Result<Expr, String> {
        let base = self.parse_primary()?;
        if !matches!(self.current().kind, TokenKind::StarStar) {
            return Ok(base);
        }
        self.advance();
        // Right-associative, and the exponent may be negated: 2 ** -1.
        let exponent = self.parse_unary()?;
        Ok(Expr::Binary(BinaryOp::Power, Box::new(base), Box::new(exponent)))
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        let (line, column) = (self.current().line, self.current().column);
        if self.at_function_type() {
            // function(int x): int { ... }
            self.advance();
            let parameters = self.parse_closure_parameters()?;
            self.expect(TokenKind::Colon, "expected ':' and a return type after the parameters, such as function(int x): int { ... }")?;
            let return_type = self.parse_return_type()?;
            let body = self.parse_statement_block("expected '{' before the function body")?;
            return self.parse_postfix(Expr::Closure(Box::new(Closure { parameters, return_type: Some(return_type), body, line, column })));
        }
        if self.at_arrow_function() {
            // fn(int x) => x * 2
            self.advance();
            let parameters = self.parse_closure_parameters()?;
            self.expect(TokenKind::FatArrow, "expected '=>' after the parameters")?;
            let (value_line, value_column) = (self.current().line, self.current().column);
            let value = self.parse_expression()?;
            return Ok(Expr::Closure(Box::new(Closure {
                parameters,
                return_type: None,
                body: vec![Statement { kind: StatementKind::Return(Some(value)), line: value_line, column: value_column }],
                line,
                column,
            })));
        }
        let expression = match self.current().kind.clone() {
            TokenKind::New
                if matches!(
                    (
                        self.tokens.get(self.cursor + 1).map(|token| &token.kind),
                        self.tokens.get(self.cursor + 2).map(|token| &token.kind)
                    ),
                    (Some(TokenKind::Static), Some(TokenKind::LeftParen))
                ) =>
            {
                let enclosing = self
                    .current_class
                    .ok_or_else(|| self.error("new static can only be used inside a class"))?;
                self.advance();
                self.advance();
                self.advance();
                let arguments = self.parse_arguments()?;
                self.expect(TokenKind::RightParen, "expected ')' after constructor arguments")?;
                Expr::NewStatic(enclosing, arguments)
            }
            TokenKind::New
                if !matches!(
                    (
                        self.tokens.get(self.cursor + 1).map(|token| &token.kind),
                        self.tokens.get(self.cursor + 2).map(|token| &token.kind)
                    ),
                    (Some(TokenKind::Identifier(_)), Some(TokenKind::LeftParen))
                ) =>
            {
                // `new type[size]`: the element type may itself be an array
                // (`new int[][3]`), a map, or a class (`new Page[3]`).
                self.advance();
                let element_type = self.parse_value_type()?;
                self.expect(TokenKind::LeftBracket, "expected '[size]' after the array element type")?;
                let size = self.parse_expression()?;
                self.expect(TokenKind::RightBracket, "expected ']' after array size")?;
                Expr::NewArray(element_type, Box::new(size))
            }
            TokenKind::New => {
                self.advance();
                let class_name = self.expect_identifier("expected class name after 'new'")?;
                let class_id = *self.class_names.get(&class_name).ok_or_else(|| {
                    self.error(&format!("class '{class_name}' is not declared{}", stdlib_hint(&class_name)))
                })?;
                self.expect(TokenKind::LeftParen, "expected '(' after class name")?;
                let arguments = self.parse_arguments()?;
                self.expect(TokenKind::RightParen, "expected ')' after constructor arguments")?;
                Expr::NewObject(class_id, arguments)
            }
            TokenKind::Number(value) => {
                let value = i64::try_from(value)
                    .map_err(|_| self.error("integer literal is outside the int range"))?;
                self.advance();
                Expr::Number(value)
            }
            TokenKind::LeftParen => {
                self.advance();
                let expression = self.parse_expression()?;
                self.expect(TokenKind::RightParen, "expected ')' after expression")?;
                expression
            }
            TokenKind::LeftBracket => {
                self.advance();
                let mut elements = Vec::new();
                if !matches!(self.current().kind, TokenKind::RightBracket) {
                    loop {
                        elements.push(self.parse_expression()?);
                        if !matches!(self.current().kind, TokenKind::Comma) {
                            break;
                        }
                        self.advance();
                    }
                }
                self.expect(TokenKind::RightBracket, "expected ']' after array elements")?;
                Expr::ArrayLiteral(elements)
            }
            TokenKind::LeftBrace => {
                self.advance();
                let mut entries = Vec::new();
                if !matches!(self.current().kind, TokenKind::RightBrace) {
                    loop {
                        let key = self.parse_expression()?;
                        if !matches!(self.current().kind, TokenKind::FatArrow) {
                            return Err(self.error(
                                "expected '=>' between map key and value (array literals use [...])",
                            ));
                        }
                        self.advance();
                        let value = self.parse_expression()?;
                        entries.push((key, value));
                        if !matches!(self.current().kind, TokenKind::Comma) {
                            break;
                        }
                        self.advance();
                    }
                }
                self.expect(TokenKind::RightBrace, "expected '}' after map entries")?;
                Expr::MapLiteral(entries)
            }
            TokenKind::Decimal(value) => {
                self.advance();
                Expr::Decimal(value)
            }
            TokenKind::Boolean(value) => {
                self.advance();
                Expr::Boolean(value)
            }
            TokenKind::NullLiteral => {
                self.advance();
                Expr::Null
            }
            TokenKind::Text(value) => {
                self.advance();
                Expr::Text(value)
            }
            TokenKind::Character(value) => {
                self.advance();
                Expr::Character(value)
            }
            // A class name before `::`; a variable before `::class` (that
            // isn't also a class's name) is the object whose class it asks.
            TokenKind::Identifier(name)
                if matches!(
                    self.tokens.get(self.cursor + 1).map(|token| &token.kind),
                    Some(TokenKind::DoubleColon)
                ) && (name == "self"
                    || self.class_names.contains_key(&name)
                    || !matches!(self.tokens.get(self.cursor + 2).map(|token| &token.kind), Some(TokenKind::Class))) =>
            {
                let target = if name == "self" {
                    let class_id = self
                        .current_class
                        .ok_or_else(|| self.error("self:: can only be used inside a class"))?;
                    StaticTarget::Class { class_id, forwards: true }
                } else {
                    let class_id = *self.class_names.get(&name).ok_or_else(|| {
                        self.error(&format!("class '{name}' is not declared or imported{}", stdlib_hint(&name)))
                    })?;
                    StaticTarget::Class { class_id, forwards: false }
                };
                self.parse_static_member(target)?
            }
            TokenKind::Parent => {
                if !matches!(
                    self.tokens.get(self.cursor + 1).map(|token| &token.kind),
                    Some(TokenKind::DoubleColon)
                ) {
                    return Err(self.error(
                        "write parent::__construct(...) to call the parent constructor, or parent::method(...) for another method",
                    ));
                }
                let enclosing = self
                    .current_class
                    .ok_or_else(|| self.error("parent:: can only be used inside a class"))?;
                let parent = self
                    .current_parent
                    .ok_or_else(|| self.error("parent:: requires a class that extends another"))?;
                self.parse_static_member(StaticTarget::Parent { parent, enclosing })?
            }
            TokenKind::Static
                if matches!(
                    self.tokens.get(self.cursor + 1).map(|token| &token.kind),
                    Some(TokenKind::DoubleColon)
                ) =>
            {
                let enclosing = self
                    .current_class
                    .ok_or_else(|| self.error("static:: can only be used inside a class"))?;
                self.parse_static_member(StaticTarget::Late { enclosing })?
            }
            TokenKind::Identifier(name) => {
                self.advance();
                if matches!(self.current().kind, TokenKind::LeftParen) {
                    self.advance();
                    let arguments = self.parse_arguments()?;
                    self.expect(TokenKind::RightParen, "expected ')' after arguments")?;
                    Expr::Call(name, arguments)
                } else {
                    Expr::Variable(name)
                }
            }
            _ => return Err(self.error("expected a literal, variable, or function call")),
        };
        self.parse_postfix(expression)
    }

    /// Indexing, members, method calls, and calls of function values after
    /// an expression.
    fn parse_postfix(&mut self, mut expression: Expr) -> Result<Expr, String> {
        loop {
            if matches!(self.current().kind, TokenKind::PlusPlus | TokenKind::MinusMinus)
                && !matches!(self.tokens.get(self.cursor + 1).map(|token| &token.kind), Some(TokenKind::Semicolon))
            {
                // `print(i++)`, `a[i++]`: only `i++;` on its own is allowed.
                return Err(self.error(INCREMENT_IS_A_STATEMENT));
            }
            if matches!(self.current().kind, TokenKind::DoubleColon)
                && matches!(self.tokens.get(self.cursor + 1).map(|token| &token.kind), Some(TokenKind::Class))
            {
                // `shape::class`: the object's class's name.
                self.advance();
                self.advance();
                expression = Expr::ClassOf(Box::new(expression));
                continue;
            }
            if matches!(self.current().kind, TokenKind::LeftParen) {
                // Calling a function value: makeAdder(2)(3), (object->callback)(x).
                self.advance();
                let arguments = self.parse_arguments()?;
                self.expect(TokenKind::RightParen, "expected ')' after arguments")?;
                expression = Expr::CallValue(Box::new(expression), arguments);
            } else if matches!(self.current().kind, TokenKind::LeftBracket) {
                self.advance();
                let index = self.parse_expression()?;
                self.expect(TokenKind::RightBracket, "expected ']' after index")?;
                expression = Expr::Index(Box::new(expression), Box::new(index));
            } else if matches!(self.current().kind, TokenKind::Arrow) {
                self.advance();
                let member = self.expect_identifier("expected member name after '.'")?;
                if matches!(self.current().kind, TokenKind::LeftParen) {
                    self.advance();
                    let arguments = self.parse_arguments()?;
                    self.expect(TokenKind::RightParen, "expected ')' after method arguments")?;
                    expression = Expr::MethodCall(Box::new(expression), member, arguments);
                } else {
                    expression = Expr::Member(Box::new(expression), member);
                }
            } else if matches!(self.current().kind, TokenKind::NullsafeArrow) {
                // `object?->member...`: the rest of the chain runs only when
                // the object isn't null, on a hidden variable standing for it.
                self.advance();
                self.next_hidden += 1;
                let placeholder = format!("#{}", self.next_hidden);
                let member = self.expect_identifier("expected a property or method name after '?->'")?;
                let first = if matches!(self.current().kind, TokenKind::LeftParen) {
                    self.advance();
                    let arguments = self.parse_arguments()?;
                    self.expect(TokenKind::RightParen, "expected ')' after method arguments")?;
                    Expr::MethodCall(Box::new(Expr::Variable(placeholder.clone())), member, arguments)
                } else {
                    Expr::Member(Box::new(Expr::Variable(placeholder.clone())), member)
                };
                let rest = self.parse_postfix(first)?;
                return Ok(Expr::Nullsafe(Box::new(expression), placeholder, Box::new(rest)));
            } else {
                break;
            }
        }
        Ok(expression)
    }

    /// Parses call arguments up to (not including) `)`. An argument is either
    /// positional or named (`name: value`); named arguments come last.
    fn parse_arguments(&mut self) -> Result<Vec<Expr>, String> {
        let mut arguments = Vec::new();
        if matches!(self.current().kind, TokenKind::RightParen) {
            return Ok(arguments);
        }
        loop {
            let is_named = matches!(self.current().kind, TokenKind::Identifier(_))
                && matches!(
                    self.tokens.get(self.cursor + 1).map(|token| &token.kind),
                    Some(TokenKind::Colon)
                );
            if is_named {
                let name = self.expect_identifier("expected parameter name")?;
                if arguments
                    .iter()
                    .any(|argument| matches!(argument, Expr::NamedArgument(existing, _) if *existing == name))
                {
                    return Err(self.error(&format!("argument '{name}' is named more than once")));
                }
                self.advance();
                let value = self.parse_expression()?;
                arguments.push(Expr::NamedArgument(name, Box::new(value)));
            } else {
                if arguments.iter().any(|argument| matches!(argument, Expr::NamedArgument(..))) {
                    return Err(self.error("positional arguments cannot follow named arguments"));
                }
                arguments.push(self.parse_expression()?);
            }
            if !matches!(self.current().kind, TokenKind::Comma) {
                return Ok(arguments);
            }
            self.advance();
        }
    }

    /// After `X` in `X::member`: a method call (with parentheses) or a static
    /// property.
    fn parse_static_member(&mut self, target: StaticTarget) -> Result<Expr, String> {
        self.advance();
        self.expect(TokenKind::DoubleColon, "expected '::'")?;
        if matches!(self.current().kind, TokenKind::Class) {
            self.advance();
            return Ok(Expr::ClassName(target));
        }
        let member = self.expect_identifier("expected a method or property name after '::'")?;
        if !matches!(self.current().kind, TokenKind::LeftParen) {
            return Ok(Expr::StaticProperty(target, member));
        }
        self.advance();
        let arguments = self.parse_arguments()?;
        self.expect(TokenKind::RightParen, "expected ')' after arguments")?;
        Ok(Expr::StaticCall(target, member, arguments))
    }

    fn parse_value_type(&mut self) -> Result<ValueType, String> {
        if matches!(self.current().kind, TokenKind::LeftParen) {
            // Grouping, for arrays of functions: (function(int): int)[]
            self.advance();
            let inner = self.parse_value_type()?;
            self.expect(TokenKind::RightParen, "expected ')' after the type")?;
            return Ok(self.parse_array_suffixes(inner));
        }
        if self.at_function_type() {
            // function(int, string): bool
            self.advance();
            self.advance();
            let mut parameters = Vec::new();
            if !matches!(self.current().kind, TokenKind::RightParen) {
                loop {
                    let parameter = self.parse_value_type()?;
                    if parameter == ValueType::Void {
                        return Err(self.error("function parameters can't be void"));
                    }
                    parameters.push(parameter);
                    if !matches!(self.current().kind, TokenKind::Comma) {
                        break;
                    }
                    self.advance();
                }
            }
            self.expect(TokenKind::RightParen, "expected ')' after the function type's parameter types")?;
            self.expect(TokenKind::Colon, "expected ':' and a return type after function(...), such as function(int): void")?;
            let return_type = self.parse_return_type()?;
            return Ok(function_type_of(&mut self.function_types, FunctionType { parameters, return_type }));
        }
        if matches!(self.current().kind, TokenKind::Map) {
            self.advance();
            self.expect(TokenKind::LeftBracket, "expected '[' after 'map'")?;
            let key_type = self.parse_value_type()?;
            if !key_type.is_scalar_key() {
                return Err(self.error("map keys must use a scalar type"));
            }
            self.expect(TokenKind::Comma, "expected ',' between map key and value types")?;
            let value_type = self.parse_value_type()?;
            if value_type == ValueType::Void {
                return Err(self.error("map values cannot be void"));
            }
            self.expect(TokenKind::RightBracket, "expected ']' after map types")?;
            let descriptor = MapType {
                key_type,
                value_type,
            };
            let id = if let Some(existing) = self.map_types.iter().position(|candidate| {
                candidate.key_type == descriptor.key_type
                    && candidate.value_type == descriptor.value_type
            }) {
                existing as u32
            } else {
                let id = self.map_types.len() as u32;
                self.map_types.push(descriptor);
                id
            };
            return Ok(self.parse_array_suffixes(ValueType::Map(id)));
        }
        let value_type = self.parse_scalar_type()?;
        Ok(self.parse_array_suffixes(value_type))
    }

    /// Whether the tokens start a function type or anonymous function:
    /// `function(` (a declaration has a name after `function`).
    fn at_function_type(&self) -> bool {
        matches!(self.current().kind, TokenKind::Function)
            && matches!(self.tokens.get(self.cursor + 1).map(|token| &token.kind), Some(TokenKind::LeftParen))
    }

    /// Whether a statement starting with `(` declares a variable of a
    /// grouped type, `(function(): int)[] makers = [];`, rather than being an
    /// expression such as `(object->callback)(x);`.
    fn at_grouped_type_declaration(&self) -> bool {
        if !matches!(self.current().kind, TokenKind::LeftParen)
            || !matches!(self.tokens.get(self.cursor + 1).map(|token| &token.kind), Some(TokenKind::Function))
        {
            return false;
        }
        let mut depth = 0;
        for (offset, token) in self.tokens[self.cursor..].iter().enumerate() {
            match token.kind {
                TokenKind::LeftParen => depth += 1,
                TokenKind::RightParen => {
                    depth -= 1;
                    if depth == 0 {
                        let next = self.tokens.get(self.cursor + offset + 1).map(|token| &token.kind);
                        let after = self.tokens.get(self.cursor + offset + 2).map(|token| &token.kind);
                        return matches!(next, Some(TokenKind::Identifier(_)))
                            || matches!((next, after), (Some(TokenKind::LeftBracket), Some(TokenKind::RightBracket)));
                    }
                }
                TokenKind::End => return false,
                _ => {}
            }
        }
        false
    }

    /// Whether the tokens start an arrow function: `fn(` ... `) =>`.
    fn at_arrow_function(&self) -> bool {
        if !matches!(&self.current().kind, TokenKind::Identifier(name) if name == "fn")
            || !matches!(self.tokens.get(self.cursor + 1).map(|token| &token.kind), Some(TokenKind::LeftParen))
        {
            return false;
        }
        let mut depth = 0;
        for (offset, token) in self.tokens[self.cursor + 1..].iter().enumerate() {
            match token.kind {
                TokenKind::LeftParen => depth += 1,
                TokenKind::RightParen => {
                    depth -= 1;
                    if depth == 0 {
                        return matches!(
                            self.tokens.get(self.cursor + 2 + offset).map(|token| &token.kind),
                            Some(TokenKind::FatArrow)
                        );
                    }
                }
                TokenKind::End => return false,
                _ => {}
            }
        }
        false
    }

    /// An anonymous function's parameters, `(type name, ...)`, after `(`.
    fn parse_closure_parameters(&mut self) -> Result<Vec<Parameter>, String> {
        self.expect(TokenKind::LeftParen, "expected '(' before the parameters")?;
        let mut parameters: Vec<Parameter> = Vec::new();
        if !matches!(self.current().kind, TokenKind::RightParen) {
            loop {
                let value_type = self.parse_value_type()?;
                let name = self.expect_identifier("expected parameter name")?;
                if matches!(self.current().kind, TokenKind::Equals) {
                    return Err(self.error("anonymous function parameters can't have defaults"));
                }
                if parameters.iter().any(|parameter| parameter.name == name) {
                    return Err(self.error(&format!("parameter '{name}' is declared more than once")));
                }
                parameters.push(Parameter { value_type, name, default: None });
                if !matches!(self.current().kind, TokenKind::Comma) {
                    break;
                }
                self.advance();
            }
        }
        self.expect(TokenKind::RightParen, "expected ')' after parameters")?;
        Ok(parameters)
    }

    /// Applies each `[]` after a type: `int[][]` is an array of int arrays.
    /// A `[` followed by anything but `]` (such as `new int[5]`'s size) ends it.
    fn parse_array_suffixes(&mut self, mut value_type: ValueType) -> ValueType {
        while matches!(self.current().kind, TokenKind::LeftBracket)
            && matches!(
                self.tokens.get(self.cursor + 1).map(|token| &token.kind),
                Some(TokenKind::RightBracket)
            )
        {
            self.advance();
            self.advance();
            value_type = self.array_of(value_type);
        }
        value_type
    }

    fn parse_scalar_type(&mut self) -> Result<ValueType, String> {
        let value_type = match self.current().kind {
            TokenKind::Int => ValueType::Int,
            TokenKind::Double => ValueType::Double,
            TokenKind::Bool => ValueType::Bool,
            TokenKind::String => ValueType::String,
            TokenKind::CharType => ValueType::Char,
            TokenKind::ByteType => ValueType::Byte,
            TokenKind::Identifier(ref name) if self.class_names.contains_key(name) => {
                ValueType::Class(*self.class_names.get(name).unwrap())
            }
            _ => return Err(self.error("expected a built-in or declared class type")),
        };
        self.advance();
        Ok(value_type)
    }

    fn parse_return_type(&mut self) -> Result<ValueType, String> {
        if matches!(self.current().kind, TokenKind::Void) {
            self.advance();
            Ok(ValueType::Void)
        } else {
            self.parse_value_type()
        }
    }

    fn expect_identifier(&mut self, message: &str) -> Result<String, String> {
        match self.current().kind.clone() {
            TokenKind::Identifier(name) => {
                self.advance();
                Ok(name)
            }
            _ => Err(self.error(message)),
        }
    }

    fn expect(&mut self, expected: TokenKind, message: &str) -> Result<(), String> {
        if self.current().kind == expected {
            self.advance();
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    fn current(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    fn advance(&mut self) {
        if !matches!(self.current().kind, TokenKind::End) {
            self.cursor += 1;
        }
    }

    fn error(&self, message: &str) -> String {
        let token = self.current();
        format!("{}:{}: {message}", token.line, token.column)
    }
}

#[derive(Debug, Clone)]
struct Value {
    value_type: ValueType,
    operand: String,
    /// Stack slot holding a reassignable scalar variable. When set, `operand` is
    /// unused: reads load from the slot and assignments store into it.
    slot: Option<String>,
}

#[derive(Debug)]
struct FunctionSignature {
    parameters: Vec<Parameter>,
    return_type: ValueType,
}

/// A `finally` block whose try statement encloses the code being emitted.
/// Leaving the try by return, break, or continue runs it first.
#[derive(Clone)]
struct FinallyContext {
    body: Vec<Statement>,
    /// Variables in scope at the try statement.
    variables: HashMap<String, Value>,
    /// Length of the handler stack outside the try.
    handler_depth: usize,
    /// Number of enclosing loops at the try statement.
    loop_depth: usize,
}

/// For a call to a std/regex function: checks a pattern written as a
/// literal, and a preg_replace replacement written as one, so mistakes are
/// compile errors rather than a RegexException when the line runs.
fn check_regex_literals(function: &str, arguments: &[(usize, &Expr)]) -> Result<(), String> {
    const TAKE_PATTERNS: &[&str] =
        &["preg_match", "preg_match_all", "preg_test", "preg_replace", "preg_replace_callback", "preg_split"];
    if !TAKE_PATTERNS.contains(&function) {
        return Ok(());
    }
    let literal = |parameter: usize| {
        arguments.iter().find(|(index, _)| *index == parameter).and_then(|(_, argument)| match argument {
            Expr::Text(text) => Some(text.as_str()),
            _ => None,
        })
    };
    let Some(pattern) = literal(0) else { return Ok(()) };
    let checked = crate::regex_check::check_pattern(pattern)?;
    if function == "preg_replace" {
        if let Some(replacement) = literal(1) {
            crate::regex_check::check_replacement(replacement, &checked)?;
        }
    }
    Ok(())
}

/// Exception classes the runtime raises, indexed by QiqExceptionKind in
/// stdlib/runtime/qiq_runtime.h; keep the two lists in the same order.
const RUNTIME_EXCEPTION_CLASSES: [&str; 17] = [
    "RuntimeException",
    "ArithmeticException",
    "DivisionByZeroException",
    "KeyNotFoundException",
    "InvalidUtf8Exception",
    "InvalidArgumentException",
    "IOException",
    "FileNotFoundException",
    "PermissionDeniedException",
    "OutOfRangeException",
    "JsonException",
    "NetworkException",
    "DatabaseException",
    "ConstraintViolationException",
    "UniqueViolationException",
    "ProcessException",
    "RegexException",
];

/// Where `break` and `continue` go. Loops have both; a switch only has a
/// break target, so `continue` inside a switch goes to the enclosing loop.
struct LoopTargets {
    continue_label: Option<String>,
    break_label: String,
}

struct Emitter {
    /// Function signatures keyed by LLVM symbol.
    signatures: HashMap<String, FunctionSignature>,
    /// Per module: callable function name -> symbol.
    module_functions: Vec<HashMap<String, String>>,
    module_paths: Vec<String>,
    /// Module whose code is being emitted; decides how function calls resolve.
    current_module: usize,
    /// Where the statement being compiled starts (line, column), for errors.
    position: Option<(usize, usize)>,
    /// Symbols of every function called so far; standard library functions are
    /// emitted only when called.
    called_functions: Vec<String>,
    /// Classes the program creates or calls methods on (with their
    /// ancestors); standard library classes' methods are emitted only when
    /// used. Exception classes are always emitted, since the runtime raises them.
    used_classes: Vec<u32>,
    /// Variables of the current function that its anonymous functions
    /// capture: they live in cells shared with the closures.
    boxed: std::collections::BTreeSet<String>,
    /// Set while starting to emit an anonymous function: what it captures.
    closure_context: Option<ClosureContext>,
    /// Whether the current function is an anonymous function, and whether it
    /// captured `this`.
    in_closure: bool,
    closure_has_this: bool,
    /// The IR of anonymous functions and of adapters for named functions used
    /// as values, emitted as they're met.
    closure_functions: String,
    next_closure: usize,
    /// The synthetic class of the cell for each captured value type.
    cell_classes: Vec<(ValueType, u32)>,
    /// Named functions used as values, by symbol: their adapter and class.
    function_references: Vec<(String, u32)>,
    map_types: Vec<MapType>,
    array_types: Vec<ValueType>,
    function_types: Vec<FunctionType>,
    classes: Vec<ClassDef>,
    string_globals: Vec<String>,
    next_register: usize,
    next_string: usize,
    next_label: usize,
    /// Owned temporaries (operands holding one reference) per open scope:
    /// one scope per statement being emitted, plus one for the right side of
    /// `&&` and `||`. A scope's temporaries are released when it closes.
    temporaries: Vec<Vec<String>>,
    /// Set by emit_borrowed for the element or property read it emits next:
    /// that read skips the retain (see emit_borrowed).
    borrow_read: bool,
    /// Stack slots in the current function that own a reference, released
    /// before every `ret`.
    owned_slots: Vec<String>,
    next_release: usize,
    /// For each entry of `handlers`, the first scope of `temporaries` that
    /// belongs to code inside it: an exception going to that handler
    /// releases those scopes' temporaries first.
    handler_scopes: Vec<usize>,
    loop_targets: Vec<LoopTargets>,
    current_class: Option<u32>,
    current_method: Option<String>,
    /// Whether the method being emitted is static (has no `this`).
    current_method_is_static: bool,
    /// `static::method` dispatch tables needed, as (enclosing class, method).
    late_method_tables: Vec<(u32, String)>,
    /// `static::property` slot tables needed, as (enclosing class, property).
    late_property_tables: Vec<(u32, String)>,
    /// Classes containing `new static`; each needs a factory table.
    new_static_tables: Vec<u32>,
    /// Labels exceptions branch to, innermost last. The bottom entry is the
    /// function's `unwind` block, which returns to the caller (or, in main,
    /// reports the uncaught exception).
    handlers: Vec<String>,
    /// Enclosing `finally` blocks, innermost last.
    finally_stack: Vec<FinallyContext>,
    module_is_stdlib: Vec<bool>,
    /// Abstract methods called through dynamic dispatch, as (declaring type, method name).
    /// Each one gets a dispatch table indexed by class id.
    dispatch_tables: Vec<(u32, String)>,
    /// Classes `instanceof` tests for, each with a table of which class ids are it.
    instanceof_tables: Vec<u32>,
    /// Dispatched calls that omit optional arguments, as (declaring type,
    /// method name, indices of the supplied parameters). Each gets a dispatch
    /// table of adapters that fill in the defaults of the class whose method runs.
    adapter_tables: Vec<(u32, String, Vec<usize>)>,
    /// Instructions placed at the top of the current function's entry block:
    /// variable slot allocations, so a slot declared inside a loop is allocated once.
    entry_allocas: String,
    /// The type the result of the method call being emitted is used as, for
    /// Database::queryAs and queryOneAs (see emit_query_as).
    call_expectation: Option<ValueType>,
    /// Functions made for queryAs and queryOneAs: (class, whether it reads
    /// all the rows, symbol).
    row_readers: Vec<(u32, bool, String)>,
    /// The network databases the program's Database::open calls can open
    /// (see note_database_open): only their drivers are linked in.
    uses_postgres: bool,
    uses_mysql: bool,
    /// Whether `value::class` or `static::class` needs the table of class names.
    uses_class_name_table: bool,
}

/// Built-in conversion functions (see Emitter::emit_conversion).
/// Functions built into the compiler rather than declared in the standard
/// library, because each accepts several types.
const CONVERSION_FUNCTIONS: [&str; 20] = [
    // An f-string's expression (see lex_fstring).
    "__interpolate",
    "to_int",
    "to_double",
    "to_byte",
    "to_string",
    "len",
    "sprintf",
    "json_encode",
    "json_decode",
    "map_key_exists",
    "map_keys",
    "map_values",
    "in_map",
    "unset",
    "array_push",
    "array_pop",
    "array_shift",
    "array_unshift",
    "array_splice",
    "range",
];

/// Every array starts with its length as an i64, then a pointer to the
/// elements. Offset of the elements pointer in an array's header (QIQ_ARRAY_DATA in
/// stdlib/runtime/qiq_runtime.h). Arrays can grow, so the elements are found
/// through this pointer.
const ARRAY_DATA_OFFSET: usize = 8;

/// Every object starts with one i64 slot holding its class id, used for dynamic dispatch.
const OBJECT_HEADER_FIELDS: usize = 1;
/// QIQ_KIND_STRING in the runtime's QiqHeader.
const STRING_KIND: u32 = 0x7169_0001;

impl Emitter {
    fn new(
        signatures: HashMap<String, FunctionSignature>,
        map_types: Vec<MapType>,
        array_types: Vec<ValueType>,
        function_types: Vec<FunctionType>,
        classes: Vec<ClassDef>,
        modules: &[Module],
    ) -> Self {
        Self {
            signatures,
            module_functions: modules.iter().map(|module| module.functions.clone()).collect(),
            module_paths: modules.iter().map(|module| module.path.clone()).collect(),
            current_module: 0,
            position: None,
            called_functions: Vec::new(),
            used_classes: Vec::new(),
            boxed: std::collections::BTreeSet::new(),
            closure_context: None,
            in_closure: false,
            closure_has_this: false,
            closure_functions: String::new(),
            next_closure: 0,
            cell_classes: Vec::new(),
            function_references: Vec::new(),
            map_types,
            array_types,
            function_types,
            classes,
            string_globals: Vec::new(),
            next_register: 0,
            next_string: 0,
            next_label: 0,
            temporaries: vec![Vec::new()],
            borrow_read: false,
            owned_slots: Vec::new(),
            next_release: 0,
            handler_scopes: Vec::new(),
            loop_targets: Vec::new(),
            current_class: None,
            current_method: None,
            current_method_is_static: false,
            late_method_tables: Vec::new(),
            late_property_tables: Vec::new(),
            new_static_tables: Vec::new(),
            handlers: Vec::new(),
            finally_stack: Vec::new(),
            module_is_stdlib: modules.iter().map(|module| module.is_stdlib).collect(),
            dispatch_tables: Vec::new(),
            instanceof_tables: Vec::new(),
            adapter_tables: Vec::new(),
            entry_allocas: String::new(),
            call_expectation: None,
            row_readers: Vec::new(),
            uses_postgres: false,
            uses_mysql: false,
            uses_class_name_table: false,
        }
    }

    fn emit_program(mut self, program: &Program) -> Result<String, String> {
        self.validate_class_contracts()?;
        self.validate_constants_and_defaults()?;
        let is_stdlib = |function: &Function| program.modules[function.module as usize].is_stdlib;
        let mut functions_ir = String::new();
        for function in program.functions.iter().filter(|function| !is_stdlib(function)) {
            self.next_register = 0;
            self.current_module = function.module as usize;
            let ir = self
                .emit_function(function, None)
                .map_err(|error| self.in_module(function.module, error))?;
            functions_ir.push_str(&ir);
            functions_ir.push('\n');
        }
        // Standard library classes other than exceptions wait until used.
        let exception = self.exception_class()?;
        let deferred = |this: &Self, class_id: u32| {
            program.modules[program.classes[class_id as usize].module as usize].is_stdlib
                && class_id != exception
                && !this.is_subclass(class_id, exception)
        };
        let mut emitted_classes: Vec<u32> = Vec::new();
        for class_id in 0..program.classes.len() as u32 {
            if deferred(&self, class_id) {
                continue;
            }
            emitted_classes.push(class_id);
            functions_ir.push_str(&self.emit_class_methods(&program.classes[class_id as usize], class_id)?);
        }
        self.next_register = 0;
        self.next_label = 0;
        self.entry_allocas.clear();
        // Main's slots aren't released: the program ends after it.
        self.temporaries = vec![Vec::new()];
        self.owned_slots.clear();
        self.boxed = captured_names(&program.statements);
        self.in_closure = false;
        self.current_module = 0;
        self.handlers = vec!["unwind".to_string()];
        self.handler_scopes = vec![0];
        self.finally_stack.clear();
        let (main_body, main_terminated) = self
            .emit_block(&program.statements, &HashMap::new(), ValueType::Int, true)
            .map_err(|error| self.in_module(0, error))?;
        // Taken now: emitting the functions below reuses `entry_allocas`.
        let main_allocas = std::mem::take(&mut self.entry_allocas);

        // Emit the standard library functions the program calls, including ones
        // they call in turn. Extern functions only need a declaration.
        let mut extern_declarations = String::new();
        let mut emitted: Vec<&str> = Vec::new();
        loop {
            let pending: Vec<&Function> = program
                .functions
                .iter()
                .filter(|function| {
                    is_stdlib(function)
                        && self.called_functions.contains(&function.symbol)
                        && !emitted.contains(&function.symbol.as_str())
                })
                .collect();
            let pending_classes: Vec<u32> = self
                .used_classes
                .iter()
                .copied()
                .filter(|class_id| !emitted_classes.contains(class_id))
                .collect();
            if pending.is_empty() && pending_classes.is_empty() {
                break;
            }
            for class_id in pending_classes {
                emitted_classes.push(class_id);
                functions_ir.push_str(&self.emit_class_methods(&program.classes[class_id as usize], class_id)?);
            }
            for function in pending {
                emitted.push(&function.symbol);
                if function.is_extern {
                    extern_declarations.push_str(&extern_declaration(function));
                    continue;
                }
                self.next_register = 0;
                self.current_module = function.module as usize;
                let ir = self
                    .emit_function(function, None)
                    .map_err(|error| self.in_module(function.module, error))?;
                functions_ir.push_str(&ir);
                functions_ir.push('\n');
            }
        }

        // Adapters for dispatched calls that omit optional arguments: one per
        // class that can receive the call.
        let mut adapters: Vec<(u32, String, Vec<usize>)> = Vec::new();
        for (owner, method_name, supplied) in self.adapter_tables.clone() {
            for class_id in 0..self.classes.len() as u32 {
                let class = &self.classes[class_id as usize];
                if class.is_abstract || !(class_id == owner || self.is_subclass(class_id, owner)) {
                    continue;
                }
                let Some(implementer) = self.find_implementation(class_id, &method_name) else {
                    continue;
                };
                let key = (implementer, method_name.clone(), supplied.clone());
                if adapters.contains(&key) {
                    continue;
                }
                adapters.push(key);
                let module = self.classes[implementer as usize].module;
                let ir = self
                    .emit_default_adapter(implementer, &method_name, &supplied)
                    .map_err(|error| self.in_module(module, error))?;
                functions_ir.push_str(&ir);
                functions_ir.push('\n');
            }
        }

        // new static: one factory per class that can be the called class.
        let mut factories: Vec<u32> = Vec::new();
        for enclosing in self.new_static_tables.clone() {
            for class_id in self.subtypes_of(enclosing) {
                if factories.contains(&class_id) {
                    continue;
                }
                factories.push(class_id);
                let ir = self.emit_static_factory(class_id)?;
                functions_ir.push_str(&ir);
                functions_ir.push('\n');
            }
        }
        for class_id in 0..self.classes.len() as u32 {
            if !self.classes[class_id as usize].is_interface && self.has_property_defaults(class_id) {
                let ir = self.emit_defaults_function(class_id)?;
                functions_ir.push_str(&ir);
                functions_ir.push('\n');
            }
        }
        functions_ir.push_str(&std::mem::take(&mut self.closure_functions));
        let static_globals = self.emit_static_property_globals()?;

        let raise_function = self.emit_raise_function()?;
        let exception_symbol = self.classes[self.exception_class()? as usize].symbol.clone();

        let mut ir = String::from(
            "@.fmt.int = private unnamed_addr constant [6 x i8] c\"%lld\\0A\\00\", align 1\n\
             @.fmt.bool = private unnamed_addr constant [4 x i8] c\"%s\\0A\\00\", align 1\n\
             @.str.true = private unnamed_addr constant [5 x i8] c\"true\\00\", align 1\n\
             @.str.false = private unnamed_addr constant [6 x i8] c\"false\\00\", align 1\n\
             @.fmt.double = private unnamed_addr constant [4 x i8] c\"%g\\0A\\00\", align 1\n\
             @.fmt.string = private unnamed_addr constant [4 x i8] c\"%s\\0A\\00\", align 1\n\
             @.str.null = private unnamed_addr constant [5 x i8] c\"null\\00\", align 1\n",
        );
        for global in &self.string_globals {
            ir.push_str(global);
            ir.push('\n');
        }
        ir.push_str(&self.emit_class_names());
        ir.push_str(&static_globals);
        ir.push_str(&self.emit_late_tables());
        ir.push_str("@qiq_pending_exception = external global ptr\n");
        // std/sql's network drivers, or null for those no Database::open
        // call can reach, which are then left out of the executable.
        for (used, name) in [(self.uses_postgres, "postgres"), (self.uses_mysql, "mysql")] {
            if used {
                ir.push_str(&format!(
                    "@qiq_{name}_driver = external constant i8\n@qiq_sql_{name} = constant ptr @qiq_{name}_driver\n"
                ));
            } else {
                ir.push_str(&format!("@qiq_sql_{name} = constant ptr null\n"));
            }
        }
        for (owner, method_name) in &self.dispatch_tables {
            ir.push_str(&self.emit_dispatch_table(*owner, method_name, None));
        }
        // instanceof: for each class tested, a byte per class id, 1 for the
        // classes that are it (it, its subclasses, or implementers).
        if !self.instanceof_tables.is_empty() {
            ir.push_str(&format!("@qiq_no_object = private unnamed_addr constant i64 {}\n", self.classes.len()));
        }
        for &target in &self.instanceof_tables {
            let members = self.subtypes_of(target);
            // One more entry than there are classes, for null (qiq_no_object).
            let entries: Vec<String> = (0..=self.classes.len() as u32)
                .map(|class_id| format!("i8 {}", u8::from(members.contains(&class_id))))
                .collect();
            ir.push_str(&format!(
                "@qiq_is.{} = private unnamed_addr constant [{} x i8] [{}]\n",
                self.classes[target as usize].symbol,
                entries.len(),
                entries.join(", ")
            ));
        }
        for (owner, method_name, supplied) in &self.adapter_tables {
            ir.push_str(&self.emit_dispatch_table(*owner, method_name, Some(supplied)));
        }
        ir.push_str(
            "\ndeclare i32 @printf(ptr, ...)\ndeclare i32 @strcmp(ptr, ptr)\ndeclare ptr @calloc(i64, i64)\ndeclare ptr @qiq_map_new(i32, i32)\ndeclare void @qiq_retain(ptr)\ndeclare void @qiq_release(ptr)\ndeclare ptr @qiq_object_new(i64, i64)\ndeclare void @qiq_map_set(ptr, i64, i64)\ndeclare i64 @qiq_map_get(ptr, i64)\ndeclare i64 @qiq_map_iteration_end(ptr)\ndeclare i64 @qiq_map_version(ptr)\ndeclare zeroext i1 @qiq_map_entry_live(ptr, i64, i64)\ndeclare zeroext i1 @qiq_map_find(ptr, i64, ptr)\ndeclare zeroext i1 @qiq_map_has(ptr, i64)\ndeclare void @qiq_map_remove(ptr, i64)\ndeclare ptr @qiq_map_keys(ptr, i64, i32)\ndeclare ptr @qiq_map_values(ptr, i64, i32)\ndeclare zeroext i1 @qiq_map_contains_value(ptr, i64, ptr)\ndeclare zeroext i1 @qiq_values_equal(ptr, ptr, ptr)\ndeclare i64 @qiq_map_key_at(ptr, i64)\ndeclare i64 @qiq_map_value_at(ptr, i64)\ndeclare i64 @qiq_string_len(ptr)\ndeclare ptr @qiq_sprintf(ptr, ptr, ptr, i64)\ndeclare void @qiq_bind_values(ptr, ptr, i64)\ndeclare ptr @qiq_json_encode(ptr, i64, i1 zeroext)\ndeclare i64 @qiq_json_decode(ptr, ptr)\ndeclare i32 @qiq_string_equality(ptr, ptr)\ndeclare i32 @qiq_string_compare(ptr, ptr)\ndeclare ptr @qiq_string_char_at(ptr, i64)\ndeclare ptr @qiq_string_concat(ptr, ptr)\ndeclare i64 @qiq_int_divide(i64, i64)\ndeclare i64 @qiq_int_modulo(i64, i64)\ndeclare i64 @qiq_int_power(i64, i64)\ndeclare i64 @qiq_int_power_unchecked(i64, i64)\ndeclare i64 @qiq_int_shift_left(i64, i64)\ndeclare i64 @qiq_int_shift_right(i64, i64)\ndeclare double @llvm.pow.f64(double, double)\ndeclare double @llvm.fabs.f64(double)\ndeclare i64 @qiq_math_min_int(ptr)\ndeclare i64 @qiq_math_max_int(ptr)\ndeclare double @qiq_math_min_double(ptr)\ndeclare double @qiq_math_max_double(ptr)\ndeclare i64 @qiq_math_clamp_int(i64, i64, i64)\ndeclare double @qiq_math_clamp_double(double, double, double)\ndeclare void @qiq_uncaught(ptr, ptr)\ndeclare ptr @qiq_array_new(i64, i64, i32)\ndeclare void @qiq_set_arguments(i32, ptr)\ndeclare void @qiq_check_callable(ptr)\ndeclare void @qiq_array_push(ptr, i64, i64)\ndeclare i64 @qiq_range_plan(i64, i64, i64, i1 zeroext, ptr)\ndeclare ptr @qiq_range_int(i64, i64, i64, i1 zeroext)\ndeclare ptr @qiq_range_count(i64)\ndeclare ptr @qiq_range_double(double, double, double, i1 zeroext)\ndeclare ptr @qiq_range_char(ptr, ptr, i64, i1 zeroext)\ndeclare i64 @qiq_array_pop(ptr, i64)\ndeclare i64 @qiq_array_shift(ptr, i64)\ndeclare void @qiq_array_insert(ptr, i64, i64, i64)\ndeclare ptr @qiq_array_splice(ptr, i64, i64, i64, i1 zeroext, ptr)\ndeclare i64 @qiq_array_length(ptr)\ndeclare i64 @qiq_len_string(ptr)\ndeclare i64 @qiq_len_array(ptr)\ndeclare i64 @qiq_len_map(ptr)\ndeclare void @qiq_array_check_index(ptr, i64)\ndeclare void @qiq_print_value(ptr, i64)\ndeclare void @qiq_print_space()\ndeclare void @qiq_print_newline()\ndeclare i64 @qiq_double_to_int(double)\ndeclare zeroext i8 @qiq_int_to_byte(i64)\ndeclare i64 @qiq_string_to_int(ptr)\ndeclare double @qiq_string_to_double(ptr)\ndeclare ptr @qiq_int_to_string(i64)\ndeclare ptr @qiq_double_to_string(double)\n",
        );
        ir.push_str(&extern_declarations);
        ir.push('\n');
        ir.push_str(&functions_ir);
        ir.push_str(&raise_function);
        // The command line goes to the runtime first, for argv().
        ir.push_str("define i32 @main(i32 %argc, ptr %argv) {\nentry:\n  call void @qiq_set_arguments(i32 %argc, ptr %argv)\n");
        ir.push_str(&main_allocas);
        ir.push_str(&main_body);
        if !main_terminated {
            ir.push_str("  ret i32 0\n");
        }
        // An exception reached the top level: report it and exit.
        ir.push_str(&format!(
            "unwind:\n  %uncaught = load ptr, ptr @qiq_pending_exception, align 8\n  %uncaught.class = load i64, ptr %uncaught, align 8\n  %uncaught.slot = getelementptr inbounds ptr, ptr @qiq_class_names, i64 %uncaught.class\n  %uncaught.name = load ptr, ptr %uncaught.slot, align 8\n  %uncaught.message = call ptr @qiq_{}_getMessage(ptr %uncaught)\n  call void @qiq_uncaught(ptr %uncaught.name, ptr %uncaught.message)\n  unreachable\n}}\n",
            exception_symbol
        ));
        // Each class's name as a string literal, by class id, for
        // `value::class` and `static::class`.
        if self.uses_class_name_table {
            let mut entries = Vec::new();
            for (class_id, class) in self.classes.iter().enumerate() {
                let length = class.name.len() + 1;
                let layout = format!("{{ i64, i32, i32, [{length} x i8] }}");
                ir.push_str(&format!(
                    "@.classname.{class_id} = private unnamed_addr constant {layout} {{ i64 -1, i32 {STRING_KIND}, i32 0, [{length} x i8] c\"{}\\00\" }}, align 8\n",
                    llvm_escape(class.name.as_bytes())
                ));
                entries.push(format!("ptr getelementptr inbounds ({layout}, ptr @.classname.{class_id}, i32 0, i32 3)"));
            }
            ir.push_str(&format!(
                "@qiq_class_name_strings = private unnamed_addr constant [{} x ptr] [{}], align 8\n",
                entries.len(),
                entries.join(", ")
            ));
        }
        // DateTime's formatter for json.c, when the program has DateTimes,
        // so that programs without them leave out the time zone code.
        let datetime_used = self.stdlib_class("DateTime").is_some_and(|class_id| self.used_classes.contains(&class_id));
        if datetime_used {
            if !ir.contains("declare ptr @qiq_stdlib___datetime_format(") {
                ir.push_str("declare ptr @qiq_stdlib___datetime_format(ptr, i64, ptr)\n");
            }
            ir.push_str("@qiq_json_datetime_format = constant ptr @qiq_stdlib___datetime_format\n");
        } else {
            ir.push_str("@qiq_json_datetime_format = constant ptr null\n");
        }
        Ok(ir)
    }

    /// The concrete methods of a class.
    fn emit_class_methods(&mut self, class: &ClassDef, class_id: u32) -> Result<String, String> {
        let mut ir = String::new();
        for method in &class.methods {
            if method.is_abstract {
                continue;
            }
            self.next_register = 0;
            self.current_module = class.module as usize;
            let function = self
                .emit_function(method, Some(class_id))
                .map_err(|error| self.in_module(class.module, error))?;
            ir.push_str(&function);
            ir.push('\n');
        }
        Ok(ir)
    }

    /// Emits a function. Its errors say which function they're in, after
    /// their position: the statement's, or the function's for errors about
    /// the function as a whole.
    fn emit_function(&mut self, function: &Function, owner_class: Option<u32>) -> Result<String, String> {
        self.emit_function_body(function, owner_class).map_err(|error| {
            let context = format!("function '{}': ", function.name);
            match split_position(&error) {
                Some((position, message)) => format!("{position}{context}{message}"),
                None => format!("{}:{}: {context}{error}", function.line, function.column),
            }
        })
    }

    fn emit_function_body(&mut self, function: &Function, owner_class: Option<u32>) -> Result<String, String> {
        self.next_register = 0;
        self.next_label = 0;
        self.current_class = owner_class;
        self.current_method = Some(function.name.clone());
        self.boxed = captured_names(&function.body);
        let closure = self.closure_context.take();
        self.in_closure = closure.is_some();
        self.closure_has_this = false;
        self.entry_allocas.clear();
        self.handlers = vec!["unwind".to_string()];
        self.handler_scopes = vec![0];
        self.finally_stack.clear();
        self.temporaries = vec![Vec::new()];
        self.owned_slots.clear();
        // Stores that copy scalar arguments into their slots, run after the allocas.
        let mut parameter_stores = String::new();
        let mut variables = HashMap::new();
        let mut parameters = Vec::new();
        self.current_method_is_static = function.is_static;
        if let Some(context) = &closure {
            // An anonymous function gets its closure object first; captured
            // cells (and `this`) are read from its fields.
            self.current_class = context.class;
            self.current_method = context.method.clone();
            self.current_method_is_static = false;
            parameters.push("ptr %env".to_string());
            for (index, capture) in context.captures.iter().enumerate() {
                let field = OBJECT_HEADER_FIELDS + 1 + index;
                let at = format!("%capture{index}.at");
                let bits = format!("%capture{index}.bits");
                let register = if capture.name == "this" { "%this".to_string() } else { format!("%capture{index}") };
                parameter_stores.push_str(&format!(
                    "  {at} = getelementptr inbounds i64, ptr %env, i64 {field}\n  {bits} = load i64, ptr {at}, align 8\n  {register} = inttoptr i64 {bits} to ptr\n"
                ));
                if capture.name == "this" {
                    self.closure_has_this = true;
                    variables.insert(
                        "this".to_string(),
                        Value { value_type: capture.value_type, operand: register, slot: None },
                    );
                } else {
                    let prefix = if capture.read_only { "cellptrro" } else { "cellptr" };
                    variables.insert(
                        capture.name.clone(),
                        Value { value_type: capture.value_type, operand: String::new(), slot: Some(format!("{prefix}:{register}")) },
                    );
                }
            }
        }
        if owner_class.is_some() && function.is_static {
            // The class the call was made on, for static:: (late static binding).
            parameters.push("i64 %called".to_string());
        }
        if let Some(class_id) = owner_class.filter(|_| !function.is_static) {
            parameters.push("ptr %this".to_string());
            variables.insert(
                "this".to_string(),
                Value {
                    value_type: ValueType::Class(class_id),
                    operand: "%this".to_string(),
                    slot: None,
                },
            );
        }
        for (index, parameter) in function.parameters.iter().enumerate() {
            if variables.contains_key(&parameter.name) {
                return Err(format!("parameter '{}' is declared more than once", parameter.name));
            }
            let operand = format!("%arg{index}");
            let argument = Value {
                value_type: parameter.value_type,
                operand: operand.clone(),
                slot: None,
            };
            // The caller keeps its arguments alive for the call, so a
            // reference parameter is borrowed unless the function reassigns
            // it; then it gets a slot holding its own reference.
            let borrowed = is_reference(parameter.value_type) && !assigns(&function.body, &parameter.name);
            let bound = if self.boxed.contains(&parameter.name) {
                // Captured by an anonymous function: shared through a cell.
                self.new_cell(argument, false, &mut parameter_stores)
            } else if borrowed {
                argument
            } else {
                self.bind_variable(argument, &mut parameter_stores)
            };
            variables.insert(parameter.name.clone(), bound);
            parameters.push(format!("{} {operand}", parameter.value_type.llvm()));
        }

        self.entry_allocas.push_str(&parameter_stores);
        let (body, terminated) = self
            .emit_block(&function.body, &variables, function.return_type, false)?;
        let symbol = if let Some(class_id) = owner_class {
            format!("qiq_{}_{}", self.classes[class_id as usize].symbol, function.name)
        } else {
            function.symbol.clone()
        };
        let mut output = format!(
            "define {} @{}({}) {{\nentry:\n",
            function.return_type.llvm(),
            symbol,
            parameters.join(", ")
        );
        output.push_str(&self.entry_allocas);
        output.push_str(&body);
        if function.return_type == ValueType::Void && !terminated {
            output.push_str("  ret void\n");
        } else if function.return_type != ValueType::Void && !terminated {
            return Err("non-void function must return a value on every path".to_string());
        }
        output.push_str(&Self::unwind_block(function.return_type));
        output.push_str("}\n");
        let output = self.release_slots_before_returns(&output);
        self.owned_slots.clear();
        self.current_class = None;
        self.current_method = None;
        Ok(output)
    }

    /// Emits a block of statements. An error found while compiling one of
    /// them is prefixed with its position (`line:column: `), unless a block
    /// nested inside it already did that.
    fn emit_block(
        &mut self,
        statements: &[Statement],
        initial_variables: &HashMap<String, Value>,
        return_type: ValueType,
        is_main: bool,
    ) -> Result<(String, bool), String> {
        let outer = self.position;
        let result = self.emit_statements(statements, initial_variables, return_type, is_main);
        let at = self.position;
        self.position = outer;
        result.map_err(|error| match at {
            Some((line, column)) if !has_position(&error) => format!("{line}:{column}: {error}"),
            _ => error,
        })
    }

    fn emit_statements(
        &mut self,
        statements: &[Statement],
        initial_variables: &HashMap<String, Value>,
        return_type: ValueType,
        is_main: bool,
    ) -> Result<(String, bool), String> {
        let mut body = String::new();
        let mut variables = initial_variables.clone();
        let mut terminated = false;
        // Each statement's temporaries are released when the next statement
        // starts (or after the last one), after any labels it ended with.
        let mut scope_open = false;

        for statement in statements {
            self.position = Some((statement.line, statement.column));
            if terminated {
                return Err("unreachable statement after return, throw, break, or continue".to_string());
            }
            if scope_open {
                self.close_scope(&mut body);
            }
            self.open_scope();
            scope_open = true;
            match &statement.kind {
                StatementKind::Declare(declared_type, name, expression) => {
                    if variables.contains_key(name) {
                        return Err(format!("variable '{name}' is already declared"));
                    }
                    let value =
                        self.emit_expecting(expression, Some(*declared_type), &variables, &mut body)?;
                    if !self.is_assignable(*declared_type, value.value_type) {
                        return Err(format!(
                            "cannot initialize {} variable '{name}' with {}",
                            self.type_name(*declared_type),
                            self.type_name(value.value_type)
                        ));
                    }
                    let value = Value {
                        value_type: *declared_type,
                        ..value
                    };
                    let bound = self.bind_name(name, value, &mut body);
                    variables.insert(name.clone(), bound);
                }
                StatementKind::Assign(name, value_expression) => {
                    if name == "this" {
                        return Err("cannot assign to 'this'".to_string());
                    }
                    // Assigning to an undeclared name declares it, inferring the
                    // variable's type from the value (`num = 123;` declares an int).
                    let Some(target) = variables.get(name).cloned() else {
                        let value = self.emit_expression(value_expression, &variables, &mut body)?;
                        let bound = self.bind_name(name, value, &mut body);
                        variables.insert(name.clone(), bound);
                        continue;
                    };
                    if let Some((cell, read_only)) = target.slot.as_deref().and_then(|slot| self.cell_of(slot, &mut body)) {
                        if read_only {
                            return Err(format!("foreach and catch variable '{name}' cannot be reassigned"));
                        }
                        let value =
                            self.emit_expecting(value_expression, Some(target.value_type), &variables, &mut body)?;
                        if !self.is_assignable(target.value_type, value.value_type) {
                            return Err(format!(
                                "cannot assign {} to {} variable '{name}'",
                                self.type_name(value.value_type),
                                self.type_name(target.value_type)
                            ));
                        }
                        self.store_in_cell(&cell, target.value_type, value, &mut body)?;
                        continue;
                    }
                    let slot = match target.slot {
                        Some(slot) => slot,
                        None if name == "this" => return Err("cannot assign to 'this'".to_string()),
                        // Foreach variables and caught exceptions are bound to
                        // their value for the loop iteration or catch block.
                        None => return Err(format!("foreach and catch variable '{name}' cannot be reassigned")),
                    };
                    let value =
                        self.emit_expecting(value_expression, Some(target.value_type), &variables, &mut body)?;
                    if !self.is_assignable(target.value_type, value.value_type) {
                        return Err(format!(
                            "cannot assign {} to {} variable '{name}'",
                            self.type_name(value.value_type),
                            self.type_name(target.value_type)
                        ));
                    }
                    if is_reference(target.value_type) {
                        let operand = self.take(&value, &mut body);
                        self.store_owned(&slot, &operand, &mut body);
                    } else {
                        body.push_str(&format!(
                            "  store {} {}, ptr {slot}, align 8\n",
                            target.value_type.llvm(),
                            value.operand
                        ));
                    }
                }
                StatementKind::DeclareArray(array_type, name, initializer) => {
                    if variables.contains_key(name) {
                        return Err(format!("variable '{name}' is already declared"));
                    }
                    let value = match initializer {
                        ArrayInitializer::Uninitialized => {
                            // An empty array, ready to grow (like maps declared
                            // without a value); `= null` gives no array.
                            let element = self.element_of(*array_type).expect("array type");
                            let array = self.new_register();
                            body.push_str(&format!(
                                "  {array} = call ptr @qiq_array_new(i64 0, i64 {}, i32 {})\n",
                                element.size_bytes(),
                                self.contents_flags(element)
                            ));
                            self.own(Value {
                                value_type: *array_type,
                                operand: array,
                                slot: None,
                            })
                        }
                        ArrayInitializer::Existing(expression) => {
                            let value =
                                self.emit_expecting(expression, Some(*array_type), &variables, &mut body)?;
                            if value.value_type != *array_type {
                                return Err(format!(
                                    "cannot initialize {} variable '{name}' with {}",
                                    self.type_name(*array_type),
                                    self.type_name(value.value_type)
                                ));
                            }
                            value
                        }
                    };
                    let bound = self.bind_name(name, value, &mut body);
                    variables.insert(name.clone(), bound);
                }
                StatementKind::DeclareMap(map_type, name, initializer) => {
                    if variables.contains_key(name) {
                        return Err(format!("variable '{name}' is already declared"));
                    }
                    let ValueType::Map(type_id) = map_type else {
                        return Err("invalid map declaration type".to_string());
                    };
                    let signature = *self
                        .map_types
                        .get(*type_id as usize)
                        .ok_or_else(|| "invalid map type descriptor".to_string())?;
                    let value = match initializer {
                        MapInitializer::Existing(expression) => {
                            let existing =
                                self.emit_expecting(expression, Some(*map_type), &variables, &mut body)?;
                            if existing.value_type != *map_type {
                                return Err(format!(
                                    "cannot initialize {} variable '{name}' with {}",
                                    self.type_name(*map_type),
                                    self.type_name(existing.value_type)
                                ));
                            }
                            existing
                        }
                        MapInitializer::Empty => {
                            let map = self.new_register();
                            body.push_str(&format!(
                                "  {map} = call ptr @qiq_map_new(i32 {}, i32 {})\n",
                                signature.key_type.runtime_tag()?,
                                self.contents_flags(signature.value_type)
                            ));
                            self.own(Value {
                                value_type: *map_type,
                                operand: map,
                                slot: None,
                            })
                        }
                    };
                    let bound = self.bind_name(name, value, &mut body);
                    variables.insert(name.clone(), bound);
                }
                StatementKind::AssignIndex(array_expression, index_expression, value_expression) => {
                    let array = self.emit_expression(array_expression, &variables, &mut body)?;
                    let index = self.emit_expression(index_expression, &variables, &mut body)?;
                    let expected = match array.value_type {
                        ValueType::Map(type_id) => self.map_types.get(type_id as usize).map(|map| map.value_type),
                        other => self.element_of(other),
                    };
                    let value = self.emit_expecting(value_expression, expected, &variables, &mut body)?;
                    if let Some(element_type) = self.element_of(array.value_type) {
                        if index.value_type != ValueType::Int {
                            return Err("array index must be an int".to_string());
                        }
                        if !self.is_assignable(element_type, value.value_type) {
                            return Err(format!(
                                "array element assignment requires {}, found {}",
                                self.type_name(element_type),
                                self.type_name(value.value_type)
                            ));
                        }
                        let address = self.emit_element_address(&array, element_type, &index.operand, &mut body);
                        if is_reference(element_type) {
                            let operand = self.take(&value, &mut body);
                            self.store_owned(&address, &operand, &mut body);
                        } else {
                            body.push_str(&format!(
                                "  store {} {}, ptr {address}, align {}\n",
                                element_type.llvm(),
                                value.operand,
                                element_type.alignment()
                            ));
                        }
                    } else if let ValueType::Map(type_id) = array.value_type {
                        let signature = *self
                            .map_types
                            .get(type_id as usize)
                            .ok_or_else(|| "invalid map type descriptor".to_string())?;
                        if index.value_type != signature.key_type {
                            return Err(format!(
                                "map key requires {}, found {}",
                                self.type_name(signature.key_type),
                                self.type_name(index.value_type)
                            ));
                        }
                        if !self.is_assignable(signature.value_type, value.value_type) {
                            return Err(format!(
                                "map value requires {}, found {}",
                                self.type_name(signature.value_type),
                                self.type_name(value.value_type)
                            ));
                        }
                        self.take(&index, &mut body);
                        self.take(&value, &mut body);
                        let key_bits = self.encode_payload(&index, &mut body)?;
                        let value_bits = self.encode_payload(&value, &mut body)?;
                        body.push_str(&format!(
                            "  call void @qiq_map_set(ptr {}, i64 {key_bits}, i64 {value_bits})\n",
                            array.operand
                        ));
                        self.emit_exception_check(&mut body);
                    } else {
                        return Err("indexed assignment target is not an array or map".to_string());
                    }
                }
                StatementKind::AssignMember(object_expression, property_name, value_expression) => {
                    let object = self.emit_expression(object_expression, &variables, &mut body)?;
                    let class_id = match object.value_type {
                        ValueType::Class(class_id) => class_id,
                        _ => return Err("property assignment requires a class instance".to_string()),
                    };
                    self.emit_null_check(&object, object_expression, &format!("cannot assign property '{property_name}' of null"), &mut body);
                    let (property_index, property_type, visibility, owner_class) =
                        self.find_property(class_id, property_name)?;
                    self.check_visibility(visibility, owner_class, property_name)?;
                    let readonly = self.classes[owner_class as usize]
                        .properties
                        .iter()
                        .any(|property| property.name == *property_name && property.readonly);
                    if readonly {
                        let in_constructor = self.current_method.as_deref() == Some("__construct")
                            && self.current_class == Some(owner_class)
                            && matches!(object_expression, Expr::Variable(name) if name == "this");
                        if !in_constructor {
                            return Err(format!(
                                "property '{property_name}' is readonly: only {}'s constructor can set it, as this->{property_name}",
                                self.classes[owner_class as usize].name
                            ));
                        }
                    }
                    let value =
                        self.emit_expecting(value_expression, Some(property_type), &variables, &mut body)?;
                    if !self.is_assignable(property_type, value.value_type) {
                        return Err(format!(
                            "property '{property_name}' requires {}, found {}",
                            self.type_name(property_type),
                            self.type_name(value.value_type)
                        ));
                    }
                    if is_reference(property_type) {
                        self.take(&value, &mut body);
                    }
                    let encoded = self.encode_payload(&value, &mut body)?;
                    let address = self.new_register();
                    body.push_str(&format!(
                        "  {address} = getelementptr inbounds i64, ptr {}, i64 {property_index}\n",
                        object.operand
                    ));
                    if is_reference(property_type) {
                        let old = self.new_register();
                        body.push_str(&format!(
                            "  {old} = load ptr, ptr {address}, align 8\n  store i64 {encoded}, ptr {address}, align 8\n  call void @qiq_release(ptr {old})\n"
                        ));
                    } else {
                        body.push_str(&format!("  store i64 {encoded}, ptr {address}, align 8\n"));
                    }
                }
                StatementKind::Print(expressions, lines) => {
                    let mut values = Vec::new();
                    for expression in expressions {
                        let value = self.emit_expression(expression, &variables, &mut body)?;
                        if value.value_type == ValueType::Void {
                            return Err("cannot print the result of a void function".to_string());
                        }
                        // An object with __toString prints as its text, or as
                        // "null" when it's null: `object?->__toString() ?? "null"`.
                        let value = match value.value_type {
                            ValueType::Class(class_id) if self.find_method(class_id, "__toString").is_ok() => {
                                let mut scope = variables.clone();
                                scope.insert("#printed".to_string(), Value { slot: None, ..value.clone() });
                                let call = Expr::Nullsafe(
                                    Box::new(Expr::Variable("#printed".to_string())),
                                    "#printed.object".to_string(),
                                    Box::new(Expr::MethodCall(Box::new(Expr::Variable("#printed.object".to_string())), "__toString".to_string(), Vec::new())),
                                );
                                self.emit_coalesce(&call, &Expr::Text("null".to_string()), &scope, &mut body)?
                            }
                            _ => value,
                        };
                        values.push(value);
                    }
                    let scalar = |value: &Value| {
                        value.value_type.is_scalar() || value.value_type == ValueType::Null
                    };
                    if let [value] = values.as_slice() {
                        if scalar(value) {
                            // One scalar: a single printf, as before.
                            self.emit_print_scalar(value, &mut body);
                            continue;
                        }
                    }
                    // Several values, or arrays, maps, and objects: written by the
                    // runtime from a type descriptor, separated by spaces.
                    for (index, value) in values.iter().enumerate() {
                        if index > 0 {
                            // println puts each value on its own line.
                            body.push_str(if *lines { "  call void @qiq_print_newline()\n" } else { "  call void @qiq_print_space()\n" });
                        }
                        let descriptor = self.type_descriptor(value.value_type);
                        let descriptor = self.emit_expression(&Expr::Text(descriptor), &variables, &mut body)?;
                        let bits = if value.value_type == ValueType::Null {
                            "0".to_string()
                        } else {
                            self.encode_payload(value, &mut body)?
                        };
                        body.push_str(&format!(
                            "  call void @qiq_print_value(ptr {}, i64 {bits})\n",
                            descriptor.operand
                        ));
                    }
                    body.push_str("  call void @qiq_print_newline()\n");
                }
                StatementKind::Expression(expression) => {
                    // A call can stand alone; any result it returns is discarded
                    // (and freed at the end of the statement).
                    fn ends_in_call(expression: &Expr) -> bool {
                        match expression {
                            Expr::Call(..) | Expr::MethodCall(..) | Expr::StaticCall(..) | Expr::CallValue(..) => true,
                            // `logger?->log(message);`
                            Expr::Nullsafe(_, _, rest) => ends_in_call(rest),
                            _ => false,
                        }
                    }
                    let is_call = ends_in_call(expression);
                    if !is_call {
                        return Err("only a function call can be used as a statement".to_string());
                    }
                    self.emit_expression(expression, &variables, &mut body)?;
                }
                StatementKind::Return(expression) => {
                    if is_main {
                        return Err("return is not allowed at the top level".to_string());
                    }
                    // The value is computed first, then enclosing finally blocks run.
                    let instruction = match (return_type, expression) {
                        (ValueType::Void, None) => {
                            self.release_all(&mut body);
                            "  ret void\n".to_string()
                        }
                        (ValueType::Void, Some(expression)) => {
                            // `return null;` (or returning another void call) is allowed.
                            let value = self.emit_expression(expression, &variables, &mut body)?;
                            if value.value_type != ValueType::Null {
                                return Err("void function cannot return a value".to_string());
                            }
                            self.release_all(&mut body);
                            "  ret void\n".to_string()
                        }
                        (_, None) => {
                            return Err("non-void function must return a value".to_string());
                        }
                        (expected, Some(expression)) => {
                            let value = self.emit_expecting(expression, Some(expected), &variables, &mut body)?;
                            if !self.is_assignable(expected, value.value_type) {
                                return Err(format!(
                                    "return type mismatch: expected {}, found {}",
                                    self.type_name(expected),
                                    self.type_name(value.value_type)
                                ));
                            }
                            // The caller gets one reference to the result.
                            let operand = self.take(&value, &mut body);
                            self.release_all(&mut body);
                            format!("  ret {} {operand}\n", expected.llvm())
                        }
                    };
                    // The temporaries are released on this path, so the finally
                    // blocks (and exceptions inside them) must not see them.
                    let open_scopes = self.temporaries.len();
                    let released = std::mem::replace(&mut self.temporaries, vec![Vec::new(); open_scopes]);
                    let chain = self.emit_finally_chain(0, return_type, is_main);
                    self.temporaries = released;
                    let (finally_ir, finally_terminated) = chain?;
                    body.push_str(&finally_ir);
                    if !finally_terminated {
                        body.push_str(&instruction);
                    }
                    terminated = true;
                }
                StatementKind::AssignStatic(target, name, value_expression) => {
                    if let Some((owner, _)) = self.find_constant(Self::static_target_class(*target), name) {
                        return Err(format!("can't assign to constant {}::{name}", self.classes[owner as usize].name));
                    }
                    let (address, property) = self.emit_static_property_address(*target, name, &mut body)?;
                    let value = self.emit_expecting(value_expression, Some(property.value_type), &variables, &mut body)?;
                    if !self.is_assignable(property.value_type, value.value_type) {
                        return Err(format!(
                            "static property '{name}' requires {}, found {}",
                            self.type_name(property.value_type),
                            self.type_name(value.value_type)
                        ));
                    }
                    if is_reference(property.value_type) {
                        let operand = self.take(&value, &mut body);
                        self.store_owned(&address, &operand, &mut body);
                    } else {
                        body.push_str(&format!(
                            "  store {} {}, ptr {address}, align 8\n",
                            property.value_type.llvm(),
                            value.operand
                        ));
                    }
                }
                StatementKind::Break => {
                    // The innermost loop or switch.
                    let target_index = self
                        .loop_targets
                        .len()
                        .checked_sub(1)
                        .ok_or_else(|| "break is only valid inside a loop or switch".to_string())?;
                    let target = self.loop_targets[target_index].break_label.clone();
                    // Run the finally blocks of try statements inside the target.
                    let down_to = self
                        .finally_stack
                        .iter()
                        .position(|context| context.loop_depth > target_index)
                        .unwrap_or(self.finally_stack.len());
                    let (finally_ir, finally_terminated) =
                        self.emit_finally_chain(down_to, return_type, is_main)?;
                    body.push_str(&finally_ir);
                    if !finally_terminated {
                        body.push_str(&format!("  br label %{target}\n"));
                    }
                    terminated = true;
                }
                StatementKind::Continue => {
                    // The innermost loop, skipping any switches inside it.
                    let target_index = self
                        .loop_targets
                        .iter()
                        .rposition(|targets| targets.continue_label.is_some())
                        .ok_or_else(|| "continue is only valid inside a loop".to_string())?;
                    let target = self.loop_targets[target_index]
                        .continue_label
                        .clone()
                        .expect("found by rposition");
                    // Run the finally blocks of try statements inside the target loop.
                    let down_to = self
                        .finally_stack
                        .iter()
                        .position(|context| context.loop_depth > target_index)
                        .unwrap_or(self.finally_stack.len());
                    let (finally_ir, finally_terminated) =
                        self.emit_finally_chain(down_to, return_type, is_main)?;
                    body.push_str(&finally_ir);
                    if !finally_terminated {
                        body.push_str(&format!("  br label %{target}\n"));
                    }
                    terminated = true;
                }
                StatementKind::Throw(expression) => {
                    let value = self.emit_expression(expression, &variables, &mut body)?;
                    let exception = self.exception_class()?;
                    match value.value_type {
                        ValueType::Class(class_id)
                            if class_id == exception || self.is_subclass(class_id, exception) => {}
                        ValueType::Class(class_id) => {
                            return Err(format!(
                                "throw requires an Exception, found '{}'",
                                self.classes[class_id as usize].name
                            ));
                        }
                        other => {
                            return Err(format!("throw requires an Exception, found {}", self.type_name(other)));
                        }
                    }
                    let operand = self.take(&value, &mut body);
                    let (handler, cleanup) = self.handler_cleanup();
                    body.push_str(&cleanup);
                    body.push_str(&format!(
                        "  store ptr {operand}, ptr @qiq_pending_exception, align 8\n  br label %{handler}\n"
                    ));
                    terminated = true;
                }
                StatementKind::Try {
                    body: try_body,
                    catches,
                    finally,
                } => {
                    let exception = self.exception_class()?;
                    let id = self.next_label;
                    self.next_label += 1;
                    let dispatch_label = format!("try.dispatch{id}");
                    let rethrow_label = format!("try.rethrow{id}");
                    let end_label = format!("try.end{id}");
                    let outer_handler = self.handlers.last().cloned().unwrap_or_else(|| "unwind".to_string());
                    let handler_depth = self.handlers.len();
                    if let Some(finally_body) = finally {
                        self.finally_stack.push(FinallyContext {
                            body: finally_body.clone(),
                            variables: variables.clone(),
                            handler_depth,
                            loop_depth: self.loop_targets.len(),
                        });
                    }
                    let finally_index = self.finally_stack.len().wrapping_sub(1);
                    let mut reaches_end = false;

                    // The try body; its exceptions go to the catch dispatch.
                    self.handlers.push(dispatch_label.clone());
                    self.handler_scopes.push(self.temporaries.len());
                    let (try_ir, try_terminated) =
                        self.emit_block(try_body, &variables, return_type, is_main)?;
                    self.handlers.pop();
                    self.handler_scopes.pop();
                    body.push_str(&try_ir);
                    if !try_terminated {
                        let mut finally_terminated = false;
                        if finally.is_some() {
                            let (finally_ir, terminated) = self.emit_finally(finally_index, return_type, is_main)?;
                            body.push_str(&finally_ir);
                            finally_terminated = terminated;
                        }
                        if !finally_terminated {
                            body.push_str(&format!("  br label %{end_label}\n"));
                            reaches_end = true;
                        }
                    }

                    // Dispatch: test the exception's class id against each catch.
                    let exception_register = self.new_register();
                    let class_register = self.new_register();
                    body.push_str(&format!(
                        "{dispatch_label}:\n  {exception_register} = load ptr, ptr @qiq_pending_exception, align 8\n  {class_register} = load i64, ptr {exception_register}, align 8\n"
                    ));
                    let unmatched = if finally.is_some() { rethrow_label.clone() } else { outer_handler.clone() };
                    for (index, clause) in catches.iter().enumerate() {
                        let caught = clause.class_id;
                        if caught != exception && !self.is_subclass(caught, exception) {
                            return Err(format!(
                                "catch requires Exception or a subclass, found '{}'",
                                self.classes[caught as usize].name
                            ));
                        }
                        let next = if index + 1 < catches.len() {
                            format!("try.test{id}.{}", index + 1)
                        } else {
                            unmatched.clone()
                        };
                        let matching: Vec<String> = (0..self.classes.len() as u32)
                            .filter(|class_id| *class_id == caught || self.is_subclass(*class_id, caught))
                            .map(|class_id| format!(" i64 {class_id}, label %try.catch{id}.{index}"))
                            .collect();
                        if index > 0 {
                            body.push_str(&format!("try.test{id}.{index}:\n"));
                        }
                        body.push_str(&format!(
                            "  switch i64 {class_register}, label %{next} [{} ]\n",
                            matching.join("")
                        ));
                    }
                    if catches.is_empty() {
                        body.push_str(&format!("  br label %{unmatched}\n"));
                    }

                    // Catch bodies; their exceptions run finally, then propagate.
                    for (index, clause) in catches.iter().enumerate() {
                        if variables.contains_key(&clause.name) {
                            return Err(format!("variable '{}' is already declared", clause.name));
                        }
                        let mut catch_variables = variables.clone();
                        body.push_str(&format!(
                            "try.catch{id}.{index}:\n  store ptr null, ptr @qiq_pending_exception, align 8\n"
                        ));
                        let caught = self.own(Value {
                            value_type: ValueType::Class(clause.class_id),
                            operand: exception_register.clone(),
                            slot: None,
                        });
                        let caught = self.hold(caught, &mut body);
                        let caught = self.bind_read_only(&clause.name, caught, &mut body);
                        catch_variables.insert(clause.name.clone(), caught);
                        if finally.is_some() {
                            self.handlers.push(rethrow_label.clone());
                            self.handler_scopes.push(self.temporaries.len());
                        }
                        let (catch_ir, catch_terminated) =
                            self.emit_block(&clause.body, &catch_variables, return_type, is_main)?;
                        if finally.is_some() {
                            self.handlers.pop();
                            self.handler_scopes.pop();
                        }
                        body.push_str(&catch_ir);
                        if !catch_terminated {
                            let mut finally_terminated = false;
                            if finally.is_some() {
                                let (finally_ir, terminated) = self.emit_finally(finally_index, return_type, is_main)?;
                                body.push_str(&finally_ir);
                                finally_terminated = terminated;
                            }
                            if !finally_terminated {
                                body.push_str(&format!("  br label %{end_label}\n"));
                                reaches_end = true;
                            }
                        }
                    }

                    // Uncaught here: run finally with the exception set aside,
                    // then keep propagating it.
                    if finally.is_some() {
                        let saved = self.new_register();
                        body.push_str(&format!(
                            "{rethrow_label}:\n  {saved} = load ptr, ptr @qiq_pending_exception, align 8\n  store ptr null, ptr @qiq_pending_exception, align 8\n"
                        ));
                        let (finally_ir, finally_terminated) = self.emit_finally(finally_index, return_type, is_main)?;
                        body.push_str(&finally_ir);
                        if !finally_terminated {
                            body.push_str(&format!(
                                "  store ptr {saved}, ptr @qiq_pending_exception, align 8\n  br label %{outer_handler}\n"
                            ));
                        }
                        self.finally_stack.pop();
                    }

                    if reaches_end {
                        body.push_str(&format!("{end_label}:\n"));
                    } else {
                        terminated = true;
                    }
                }
                StatementKind::If {
                    condition,
                    then_body,
                    else_body,
                } => {
                    let condition = self.emit_expression(condition, &variables, &mut body)?;
                    if condition.value_type != ValueType::Bool {
                        return Err(format!(
                            "if condition must be a bool, found {}",
                            self.type_name(condition.value_type)
                        ));
                    }
                    self.release_current(&mut body);
                    let label_id = self.next_label;
                    self.next_label += 1;
                    let then_label = format!("if.then{label_id}");
                    let else_label = format!("if.else{label_id}");
                    let merge_label = format!("if.end{label_id}");
                    body.push_str(&format!(
                        "  br i1 {}, label %{then_label}, label %{else_label}\n{then_label}:\n",
                        condition.operand
                    ));

                    let (mut then_ir, then_terminated) =
                        self.emit_block(then_body, &variables, return_type, is_main)?;
                    if !then_terminated {
                        then_ir.push_str(&format!("  br label %{merge_label}\n"));
                    }
                    body.push_str(&then_ir);
                    body.push_str(&format!("{else_label}:\n"));

                    let (mut else_ir, else_terminated) =
                        self.emit_block(else_body, &variables, return_type, is_main)?;
                    if !else_terminated {
                        else_ir.push_str(&format!("  br label %{merge_label}\n"));
                    }
                    body.push_str(&else_ir);

                    if then_terminated && else_terminated {
                        terminated = true;
                    } else {
                        body.push_str(&format!("{merge_label}:\n"));
                    }
                }
                StatementKind::Switch { subject, cases, default } => {
                    let subject = self.emit_expression(subject, &variables, &mut body)?;
                    if !subject.value_type.is_scalar() {
                        return Err(format!(
                            "switch requires an int, double, bool, byte, string, or char, found {}",
                            self.type_name(subject.value_type)
                        ));
                    }
                    let id = self.next_label;
                    self.next_label += 1;
                    let end_label = format!("switch.end{id}");
                    let default_label = format!("switch.default{id}");
                    let unmatched = if default.is_some() { default_label.clone() } else { end_label.clone() };

                    // Duplicate literal values can never be reached.
                    let mut seen: Vec<String> = Vec::new();
                    for case in cases {
                        for value in &case.values {
                            let key = match value {
                                Expr::Number(n) => Some(format!("int {n}")),
                                Expr::Decimal(d) => Some(format!("double {}", d.to_bits())),
                                Expr::Text(t) => Some(format!("string {t:?}")),
                                Expr::Character(c) => Some(format!("char {c:?}")),
                                Expr::Boolean(b) => Some(format!("bool {b}")),
                                _ => None,
                            };
                            if let Some(key) = key {
                                if seen.contains(&key) {
                                    return Err(format!(
                                        "duplicate case value {}",
                                        key.split_once(' ').map_or(key.as_str(), |(_, value)| value)
                                    ));
                                }
                                seen.push(key);
                            }
                        }
                    }

                    // Tests, in order: each value is compared with the subject
                    // (evaluated once), and the first match picks its case.
                    for (case_index, case) in cases.iter().enumerate() {
                        for (value_index, value_expression) in case.values.iter().enumerate() {
                            self.open_scope();
                            let value = self.emit_expecting(
                                value_expression,
                                Some(subject.value_type),
                                &variables,
                                &mut body,
                            )?;
                            if value.value_type != subject.value_type {
                                return Err(format!(
                                    "case value must be {}, found {}",
                                    self.type_name(subject.value_type),
                                    self.type_name(value.value_type)
                                ));
                            }
                            let matched = self.emit_comparison(subject.clone(), CompareOp::Equal, value, &mut body)?;
                            self.close_scope(&mut body);
                            let last_test = case_index + 1 == cases.len() && value_index + 1 == case.values.len();
                            let next = if last_test {
                                unmatched.clone()
                            } else {
                                format!("switch.test{id}.{case_index}.{}", value_index + 1)
                            };
                            body.push_str(&format!(
                                "  br i1 {}, label %switch.case{id}.{case_index}, label %{next}\n",
                                matched.operand
                            ));
                            if !last_test {
                                body.push_str(&format!("{next}:\n"));
                            }
                        }
                    }
                    if cases.is_empty() {
                        body.push_str(&format!("  br label %{unmatched}\n"));
                    }

                    // Bodies: each ends by leaving the switch (no fallthrough).
                    let mut reaches_end = default.is_none();
                    let blocks = cases
                        .iter()
                        .enumerate()
                        .map(|(index, case)| (format!("switch.case{id}.{index}"), &case.body))
                        .chain(default.iter().map(|statements| (default_label.clone(), statements)));
                    for (label, statements) in blocks.collect::<Vec<_>>() {
                        body.push_str(&format!("{label}:\n"));
                        self.loop_targets.push(LoopTargets {
                            continue_label: None,
                            break_label: end_label.clone(),
                        });
                        let (case_ir, case_terminated) =
                            self.emit_block(statements, &variables, return_type, is_main)?;
                        self.loop_targets.pop();
                        body.push_str(&case_ir);
                        if !case_terminated {
                            body.push_str(&format!("  br label %{end_label}\n"));
                        }
                        if !case_terminated || contains_break(statements) {
                            reaches_end = true;
                        }
                    }
                    if reaches_end {
                        body.push_str(&format!("{end_label}:\n"));
                    } else {
                        // Every case and the default return or throw.
                        terminated = true;
                    }
                }
                StatementKind::While { condition, body: loop_body } => {
                    let label_id = self.next_label;
                    self.next_label += 1;
                    let test_label = format!("while.test{label_id}");
                    let body_label = format!("while.body{label_id}");
                    let end_label = format!("while.end{label_id}");
                    body.push_str(&format!("  br label %{test_label}\n{test_label}:\n"));
                    let tested = self.emit_expression(condition, &variables, &mut body)?;
                    if tested.value_type != ValueType::Bool {
                        return Err(format!(
                            "while condition must be a bool, found {}",
                            self.type_name(tested.value_type)
                        ));
                    }
                    self.release_current(&mut body);
                    body.push_str(&format!(
                        "  br i1 {}, label %{body_label}, label %{end_label}\n{body_label}:\n",
                        tested.operand
                    ));
                    self.loop_targets.push(LoopTargets {
                        continue_label: Some(test_label.clone()),
                        break_label: end_label.clone(),
                    });
                    let (loop_ir, body_terminated) =
                        self.emit_block(loop_body, &variables, return_type, is_main)?;
                    self.loop_targets.pop();
                    body.push_str(&loop_ir);
                    if !body_terminated {
                        body.push_str(&format!("  br label %{test_label}\n"));
                    }
                    body.push_str(&format!("{end_label}:\n"));
                    // `while (true)` without a break never falls through, so a
                    // function can end with it instead of a return.
                    if matches!(condition, Expr::Boolean(true)) && !contains_break(loop_body) {
                        body.push_str("  unreachable\n");
                        terminated = true;
                    }
                }
                StatementKind::Foreach {
                    iterable,
                    key_name,
                    value_name,
                    body: loop_body,
                } => {
                    if key_name.as_ref() == Some(value_name) {
                        return Err("foreach key and value variables must be different".to_string());
                    }
                    // `foreach (range(...) as ...)` over ints counts without
                    // building an array; other ranges build one.
                    let range = match iterable {
                        Expr::Call(name, arguments) if name == "range" => {
                            Some(self.emit_range_arguments(arguments, &variables, &mut body)?)
                        }
                        _ => None,
                    };
                    if let Some((ValueType::Int, values)) = &range {
                        let (step, has_step) = match values.get(2) {
                            Some(step) => (step.operand.clone(), "true"),
                            None => ("1".to_string(), "false"),
                        };
                        let counting = values.len() == 1;
                        let start = if counting { "0".to_string() } else { values[0].operand.clone() };
                        let label_id = self.next_label;
                        self.next_label += 1;
                        let step_slot = format!("%range.step{label_id}");
                        let slot = format!("%foreach.slot{label_id}");
                        self.entry_allocas.push_str(&format!(
                            "  {step_slot} = alloca i64, align 8\n  {slot} = alloca i64, align 8\n"
                        ));
                        let count = self.new_register();
                        if counting {
                            // range(count): count times from 0, or not at all.
                            let positive = self.new_register();
                            body.push_str(&format!(
                                "  {positive} = icmp sgt i64 {count_value}, 0\n  {count} = select i1 {positive}, i64 {count_value}, i64 0\n  store i64 1, ptr {step_slot}, align 8\n",
                                count_value = values[0].operand
                            ));
                        } else {
                            body.push_str(&format!(
                                "  {count} = call i64 @qiq_range_plan(i64 {start}, i64 {}, i64 {step}, i1 zeroext {has_step}, ptr {step_slot})\n",
                                values[1].operand
                            ));
                            self.emit_exception_check(&mut body);
                        }
                        let signed_step = self.new_register();
                        let index_at_test = self.new_register();
                        let condition = self.new_register();
                        let index = self.new_register();
                        let offset = self.new_register();
                        let value = self.new_register();
                        let next = self.new_register();
                        let test_label = format!("foreach.test{label_id}");
                        let body_label = format!("foreach.body{label_id}");
                        let increment_label = format!("foreach.increment{label_id}");
                        let end_label = format!("foreach.end{label_id}");
                        body.push_str(&format!(
                            "  {signed_step} = load i64, ptr {step_slot}, align 8\n  store i64 0, ptr {slot}, align 8\n  br label %{test_label}\n{test_label}:\n  {index_at_test} = load i64, ptr {slot}, align 8\n  {condition} = icmp ult i64 {index_at_test}, {count}\n  br i1 {condition}, label %{body_label}, label %{end_label}\n{body_label}:\n  {index} = load i64, ptr {slot}, align 8\n  {offset} = mul i64 {index}, {signed_step}\n  {value} = add i64 {start}, {offset}\n"
                        ));
                        let mut loop_variables = variables.clone();
                        if let Some(key_name) = key_name {
                            let key = Value { value_type: ValueType::Int, operand: index.clone(), slot: None };
                            let key = self.bind_read_only(key_name, key, &mut body);
                            loop_variables.insert(key_name.clone(), key);
                        }
                        let value = Value { value_type: ValueType::Int, operand: value, slot: None };
                        let value = self.bind_read_only(value_name, value, &mut body);
                        loop_variables.insert(value_name.clone(), value);
                        self.loop_targets.push(LoopTargets {
                            continue_label: Some(increment_label.clone()),
                            break_label: end_label.clone(),
                        });
                        let (loop_ir, body_terminated) = self.emit_block(loop_body, &loop_variables, return_type, is_main)?;
                        self.loop_targets.pop();
                        body.push_str(&loop_ir);
                        if !body_terminated {
                            body.push_str(&format!("  br label %{increment_label}\n"));
                        }
                        body.push_str(&format!(
                            "{increment_label}:\n  {next} = add i64 {index}, 1\n  store i64 {next}, ptr {slot}, align 8\n  br label %{test_label}\n{end_label}:\n"
                        ));
                        continue;
                    }
                    let collection = match range {
                        Some((kind, values)) => self.emit_range_array(kind, &values, &mut body)?,
                        None => self.emit_expression(iterable, &variables, &mut body)?,
                    };
                    // The loop holds its own reference, so reassigning the
                    // variable it came from inside the loop can't free it.
                    let collection = self.hold(collection, &mut body);
                    // For a map: the additions count at the start, to detect keys
                    // added during the loop.
                    let mut map_version = None;
                    let (key_type, value_type, length, iterator_kind) =
                        if let Some(element_type) = self.element_of(collection.value_type) {
                            let length = self.new_register();
                            body.push_str(&format!(
                                "  {length} = call i64 @qiq_array_length(ptr {})\n",
                                collection.operand
                            ));
                            // A null array throws RuntimeException.
                            self.emit_exception_check(&mut body);
                            (Some(ValueType::Int), element_type, length, 0)
                        } else if collection.value_type == ValueType::String {
                            let length = self.new_register();
                            body.push_str(&format!(
                                "  {length} = call i64 @qiq_string_len(ptr {})\n",
                                collection.operand
                            ));
                            self.emit_exception_check(&mut body);
                            (Some(ValueType::Int), ValueType::Char, length, 1)
                        } else if let ValueType::Map(type_id) = collection.value_type {
                            let descriptor = *self
                                .map_types
                                .get(type_id as usize)
                                .ok_or_else(|| "invalid map type descriptor".to_string())?;
                            // Positions in the map's entries, in insertion order;
                            // removed entries are skipped below.
                            let length = self.new_register();
                            body.push_str(&format!(
                                "  {length} = call i64 @qiq_map_iteration_end(ptr {})\n",
                                collection.operand
                            ));
                            self.emit_exception_check(&mut body);
                            let version = self.new_register();
                            body.push_str(&format!(
                                "  {version} = call i64 @qiq_map_version(ptr {})\n",
                                collection.operand
                            ));
                            map_version = Some(version);
                            (Some(descriptor.key_type), descriptor.value_type, length, 2)
                        } else {
                            return Err("foreach requires an array, map, or string".to_string());
                        };

                    let label_id = self.next_label;
                    self.next_label += 1;
                    let index_at_test = self.new_register();
                    let condition = self.new_register();
                    let index_at_body = self.new_register();
                    let index_next = self.new_register();
                    let slot = format!("%foreach.slot{label_id}");
                    // An array can grow or shrink inside the loop, so its
                    // current length is checked before each element, as in Python.
                    let (length_now, bound) = if iterator_kind == 0 {
                        (
                            format!("  %foreach.length{label_id} = load i64, ptr {}, align 8\n", collection.operand),
                            format!("%foreach.length{label_id}"),
                        )
                    } else {
                        (String::new(), length.clone())
                    };
                    // Allocated once in the entry block, so a foreach nested in
                    // another loop doesn't grow the stack on every iteration.
                    self.entry_allocas.push_str(&format!("  {slot} = alloca i64, align 8\n"));
                    let test_label = format!("foreach.test{label_id}");
                    let body_label = format!("foreach.body{label_id}");
                    let increment_label = format!("foreach.increment{label_id}");
                    let end_label = format!("foreach.end{label_id}");
                    body.push_str(&format!(
                        "  store i64 0, ptr {slot}, align 8\n  br label %{test_label}\n{test_label}:\n  {index_at_test} = load i64, ptr {slot}, align 8\n{length_now}  {condition} = icmp ult i64 {index_at_test}, {bound}\n  br i1 {condition}, label %{body_label}, label %{end_label}\n{body_label}:\n  {index_at_body} = load i64, ptr {slot}, align 8\n"
                    ));

                    if let Some(version) = &map_version {
                        let live = self.new_register();
                        body.push_str(&format!(
                            "  {live} = call i1 @qiq_map_entry_live(ptr {}, i64 {index_at_body}, i64 {version})\n",
                            collection.operand
                        ));
                        self.emit_exception_check(&mut body);
                        body.push_str(&format!(
                            "  br i1 {live}, label %foreach.entry{label_id}, label %{increment_label}\nforeach.entry{label_id}:\n"
                        ));
                    }
                    let mut loop_variables = variables.clone();
                    if let Some(key_name) = key_name {
                        let key_value = match iterator_kind {
                            0 | 1 => {
                                Value {
                                    value_type: ValueType::Int,
                                    operand: index_at_body.clone(),
                                    slot: None,
                                }
                            }
                            _ => {
                                let encoded = self.new_register();
                                body.push_str(&format!(
                                    "  {encoded} = call i64 @qiq_map_key_at(ptr {}, i64 {index_at_body})\n",
                                    collection.operand
                                ));
                                let key = self.decode_payload(key_type.unwrap(), &encoded, &mut body)?;
                                self.hold(key, &mut body)
                            }
                        };
                        let key_value = self.bind_read_only(key_name, key_value, &mut body);
                        loop_variables.insert(key_name.clone(), key_value);
                    }

                    let loop_value = match iterator_kind {
                        0 => {
                            let element_type = value_type;
                            let data = self.new_register();
                            let address = self.new_register();
                            body.push_str(&format!(
                                "  {data}.at = getelementptr inbounds i8, ptr {}, i64 {ARRAY_DATA_OFFSET}\n  {data} = load ptr, ptr {data}.at, align 8\n  {address} = getelementptr inbounds {}, ptr {data}, i64 {index_at_body}\n",
                                collection.operand,
                                element_type.llvm()
                            ));
                            let loaded = self.new_register();
                            body.push_str(&format!(
                                "  {loaded} = load {}, ptr {address}, align {}\n",
                                element_type.llvm(), element_type.alignment()
                            ));
                            self.hold(
                                Value {
                                    value_type: element_type,
                                    operand: loaded,
                                    slot: None,
                                },
                                &mut body,
                            )
                        }
                        1 => {
                            let character = self.new_register();
                            body.push_str(&format!(
                                "  {character} = call ptr @qiq_string_char_at(ptr {}, i64 {index_at_body})\n",
                                collection.operand
                            ));
                            self.emit_exception_check(&mut body);
                            let character = self.own(Value {
                                value_type: ValueType::Char,
                                operand: character,
                                slot: None,
                            });
                            self.hold(character, &mut body)
                        }
                        _ => {
                            let encoded = self.new_register();
                            body.push_str(&format!(
                                "  {encoded} = call i64 @qiq_map_value_at(ptr {}, i64 {index_at_body})\n",
                                collection.operand
                            ));
                            let value = self.decode_payload(value_type, &encoded, &mut body)?;
                            self.hold(value, &mut body)
                        }
                    };
                    let loop_value = self.bind_read_only(value_name, loop_value, &mut body);
                    loop_variables.insert(value_name.clone(), loop_value);
                    self.loop_targets.push(LoopTargets {
                        continue_label: Some(increment_label.clone()),
                        break_label: end_label.clone(),
                    });
                    let (loop_ir, body_terminated) =
                        self.emit_block(loop_body, &loop_variables, return_type, is_main)?;
                    self.loop_targets.pop();
                    body.push_str(&loop_ir);
                    if !body_terminated {
                        body.push_str(&format!("  br label %{increment_label}\n"));
                    }
                    body.push_str(&format!(
                        "{increment_label}:\n  {index_next} = add i64 {index_at_body}, 1\n  store i64 {index_next}, ptr {slot}, align 8\n  br label %{test_label}\n"
                    ));
                    body.push_str(&format!("{end_label}:\n"));
                }
            }
        }
        if scope_open {
            if terminated {
                // Nothing runs after a return, throw, break, or continue.
                self.temporaries.pop();
            } else {
                self.close_scope(&mut body);
            }
        }

        Ok((body, terminated))
    }

    fn emit_comparison(
        &mut self,
        left: Value,
        operator: CompareOp,
        right: Value,
        body: &mut String,
    ) -> Result<Value, String> {
        // `value == null` and `value != null` test strings, chars, arrays,
        // maps, and objects for null.
        let nullable = is_reference;
        let null_test = (left.value_type == ValueType::Null && nullable(right.value_type))
            || (right.value_type == ValueType::Null && nullable(left.value_type));
        if null_test {
            let predicate = match operator {
                CompareOp::Equal => "eq",
                CompareOp::NotEqual => "ne",
                _ => return Err("null supports only == and != comparisons".to_string()),
            };
            let register = self.new_register();
            body.push_str(&format!(
                "  {register} = icmp {predicate} ptr {}, {}\n",
                left.operand, right.operand
            ));
            return Ok(Value {
                value_type: ValueType::Bool,
                operand: register,
                slot: None,
            });
        }
        if let (ValueType::Class(a), ValueType::Class(b)) = (left.value_type, right.value_type) {
            // `==` and `!=` on objects: the same object (PHP's ===), for two
            // values of the same class or of related ones (a class and its
            // subclass, or an interface and a class implementing it).
            let related = a == b || self.is_subclass(a, b) || self.is_subclass(b, a);
            if !related {
                return Err(format!(
                    "comparison requires related types, found {} and {}: an object of one can never be the other",
                    self.type_name(left.value_type),
                    self.type_name(right.value_type)
                ));
            }
            let predicate = match operator {
                CompareOp::Equal => "eq",
                CompareOp::NotEqual => "ne",
                _ => return Err(format!("{} values support only == and != comparisons", self.type_name(left.value_type))),
            };
            let register = self.new_register();
            body.push_str(&format!("  {register} = icmp {predicate} ptr {}, {}\n", left.operand, right.operand));
            return Ok(Value { value_type: ValueType::Bool, operand: register, slot: None });
        }
        if left.value_type != right.value_type {
            return Err(format!(
                "comparison requires matching types, found {} and {}",
                self.type_name(left.value_type),
                self.type_name(right.value_type)
            ));
        }
        let mut string_result = None;
        let predicate = match left.value_type {
            ValueType::Int => match operator {
                CompareOp::Less => "slt",
                CompareOp::Greater => "sgt",
                CompareOp::LessEqual => "sle",
                CompareOp::GreaterEqual => "sge",
                CompareOp::Equal => "eq",
                CompareOp::NotEqual => "ne",
            },
            ValueType::Byte => match operator {
                CompareOp::Less => "ult",
                CompareOp::Greater => "ugt",
                CompareOp::LessEqual => "ule",
                CompareOp::GreaterEqual => "uge",
                CompareOp::Equal => "eq",
                CompareOp::NotEqual => "ne",
            },
            ValueType::Double => match operator {
                CompareOp::Less => "olt",
                CompareOp::Greater => "ogt",
                CompareOp::LessEqual => "ole",
                CompareOp::GreaterEqual => "oge",
                CompareOp::Equal => "oeq",
                CompareOp::NotEqual => "une",
            },
            ValueType::String | ValueType::Char => {
                // Null-safe: for == and !=, null equals only null; ordering a
                // null string throws RuntimeException.
                let result = self.new_register();
                let equality = matches!(operator, CompareOp::Equal | CompareOp::NotEqual);
                let function = if equality { "qiq_string_equality" } else { "qiq_string_compare" };
                body.push_str(&format!(
                    "  {result} = call i32 @{function}(ptr {}, ptr {})\n",
                    left.operand, right.operand
                ));
                if !equality {
                    self.emit_exception_check(body);
                }
                string_result = Some(result);
                match operator {
                    CompareOp::Less => "slt",
                    CompareOp::Greater => "sgt",
                    CompareOp::LessEqual => "sle",
                    CompareOp::GreaterEqual => "sge",
                    CompareOp::Equal => "eq",
                    CompareOp::NotEqual => "ne",
                }
            }
            ValueType::Bool => match operator {
                CompareOp::Equal => "eq",
                CompareOp::NotEqual => "ne",
                CompareOp::Less | CompareOp::Greater | CompareOp::LessEqual | CompareOp::GreaterEqual => {
                    return Err("bool values support only == and != comparisons".to_string());
                }
            },
            ValueType::Array(_) | ValueType::Map(_) => {
                return self.emit_contents_comparison(left, operator, right, body);
            }
            ValueType::Function(_) => {
                return Err(format!("{} values can only be compared with null", self.type_name(left.value_type)));
            }
            ValueType::Class(_) => unreachable!("objects are compared above"),
            ValueType::Null => match operator {
                CompareOp::Equal => "eq",
                CompareOp::NotEqual => "ne",
                CompareOp::Less | CompareOp::Greater | CompareOp::LessEqual | CompareOp::GreaterEqual => {
                    return Err("null values support only == and != comparisons".to_string());
                }
            },
            ValueType::Void => return Err("cannot compare void values".to_string()),
        };
        let register = self.new_register();
        if left.value_type == ValueType::Double {
            body.push_str(&format!(
                "  {register} = fcmp {predicate} double {}, {}\n",
                left.operand, right.operand
            ));
        } else if matches!(left.value_type, ValueType::String | ValueType::Char) {
            body.push_str(&format!(
                "  {register} = icmp {predicate} i32 {}, 0\n",
                string_result.unwrap()
            ));
        } else {
            body.push_str(&format!(
                "  {register} = icmp {predicate} {} {}, {}\n",
                left.value_type.llvm(),
                left.operand,
                right.operand
            ));
        }
        Ok(Value {
            value_type: ValueType::Bool,
            operand: register,
            slot: None,
        })
    }

    /// Throws RuntimeException with `message` if `object` is null, before it's
    /// used: reading or assigning a property, or calling a method, on null
    /// would otherwise read address 0 and crash. `this` is never null.
    fn emit_null_check(&mut self, object: &Value, expression: &Expr, message: &str, body: &mut String) {
        if matches!(expression, Expr::Variable(name) if name == "this") || matches!(expression, Expr::NewObject(..) | Expr::NewStatic(..)) {
            return;
        }
        let id = self.next_label;
        self.next_label += 1;
        let is_null = self.new_register();
        let text = self.string_literal(message.as_bytes());
        body.push_str(&format!(
            "  {is_null} = icmp eq ptr {}, null\n  br i1 {is_null}, label %null{id}, label %notnull{id}, !prof !{{!\"branch_weights\", i32 1, i32 1048575}}\nnull{id}:\n  call void @qiq_raise(i32 0, ptr {text})\n",
            object.operand
        ));
        self.emit_exception_check(body);
        body.push_str(&format!("  br label %notnull{id}\nnotnull{id}:\n"));
    }

    /// `object?->rest`, and `object?->rest ?? default` (with `default`): if
    /// the object is null, null, or the default; otherwise `rest` on it (and,
    /// with a default, the default when `rest` is null too). Both paths store
    /// the result's payload in a stack slot, read where they meet. The rest
    /// of the chain gets a scope of its own, so its temporaries are released
    /// on its path. A result that can't be null (a number or bool) needs a
    /// default.
    fn emit_nullsafe(
        &mut self,
        expression: &Expr,
        default: Option<&Expr>,
        variables: &HashMap<String, Value>,
        body: &mut String,
    ) -> Result<Value, String> {
        let Expr::Nullsafe(object_expression, placeholder, rest) = expression else { unreachable!() };
        let object = self.emit_expression(object_expression, variables, body)?;
        if !matches!(object.value_type, ValueType::Class(_)) {
            return Err(format!("'?->' needs an object, found {}", self.type_name(object.value_type)));
        }
        let id = self.next_label;
        self.next_label += 1;
        let slot = format!("%nullsafe.slot{id}");
        self.entry_allocas.push_str(&format!("  {slot} = alloca i64, align 8\n"));
        let is_null = self.new_register();
        body.push_str(&format!(
            "  {is_null} = icmp eq ptr {}, null\n  br i1 {is_null}, label %nullsafe.null{id}, label %nullsafe.object{id}\nnullsafe.object{id}:\n",
            object.operand
        ));
        // The rest of the chain, on the object.
        let mut scope = variables.clone();
        scope.insert(placeholder.clone(), Value { slot: None, ..object.clone() });
        self.open_scope();
        // `a?->b?->c ?? d`: the default also covers a null further along.
        let value = match (default, &**rest) {
            (Some(_), Expr::Nullsafe(..)) => self.emit_nullsafe(rest, default, &scope, body)?,
            _ => self.emit_expression(rest, &scope, body)?,
        };
        let value_type = value.value_type;
        // A void method's call has the type null.
        if matches!(value_type, ValueType::Void | ValueType::Null) {
            // `logger?->log(message);`: nothing to give back; skipped when null.
            if default.is_some() {
                return Err("'??' needs a value, but the method after '?->' returns void".to_string());
            }
            self.close_scope(body);
            body.push_str(&format!("  br label %nullsafe.end{id}\nnullsafe.null{id}:\n  br label %nullsafe.end{id}\nnullsafe.end{id}:\n"));
            return Ok(Value { value_type: ValueType::Null, operand: "null".to_string(), slot: None });
        }
        if !is_reference(value_type) && default.is_none() {
            return Err(format!(
                "'?->' gives null when the object is null, and a value of type {} can't be null: add a default with ??, as in `x?->count ?? 0`",
                self.type_name(value_type)
            ));
        }
        if is_reference(value_type) {
            self.take(&value, body);
        }
        let encoded = self.encode_payload(&value, body)?;
        self.close_scope(body);
        if default.is_some() && is_reference(value_type) {
            // `?? default` also covers a null result.
            let missing = self.new_register();
            body.push_str(&format!(
                "  {missing} = icmp eq i64 {encoded}, 0\n  store i64 {encoded}, ptr {slot}, align 8\n  br i1 {missing}, label %nullsafe.null{id}, label %nullsafe.end{id}\n"
            ));
        } else {
            body.push_str(&format!("  store i64 {encoded}, ptr {slot}, align 8\n  br label %nullsafe.end{id}\n"));
        }
        // The object was null: null, or the default.
        body.push_str(&format!("nullsafe.null{id}:\n"));
        match default {
            None => body.push_str(&format!("  store i64 0, ptr {slot}, align 8\n")),
            Some(default) => {
                self.open_scope();
                let fallback = self.emit_expecting(default, Some(value_type), variables, body)?;
                if !self.is_assignable(value_type, fallback.value_type) {
                    return Err(format!(
                        "'??' after '?->' needs a default of type {}, found {}",
                        self.type_name(value_type),
                        self.type_name(fallback.value_type)
                    ));
                }
                if is_reference(value_type) {
                    self.take(&fallback, body);
                }
                let encoded = if fallback.value_type == ValueType::Null { "0".to_string() } else { self.encode_payload(&fallback, body)? };
                self.close_scope(body);
                body.push_str(&format!("  store i64 {encoded}, ptr {slot}, align 8\n"));
            }
        }
        body.push_str(&format!("  br label %nullsafe.end{id}\nnullsafe.end{id}:\n"));
        let bits = self.new_register();
        body.push_str(&format!("  {bits} = load i64, ptr {slot}, align 8\n"));
        let result = self.decode_payload(value_type, &bits, body)?;
        Ok(if is_reference(value_type) { self.own(result) } else { result })
    }

    /// An object as text, for print, `+`, to_string, and sprintf: the result
    /// of its `__toString()` (dispatched like any call, so a subclass's own
    /// version runs), or None if `value` isn't an object whose class (or
    /// interface) has one.
    fn emit_to_string_call(
        &mut self,
        value: &Value,
        variables: &HashMap<String, Value>,
        body: &mut String,
    ) -> Result<Option<Value>, String> {
        let ValueType::Class(class_id) = value.value_type else { return Ok(None) };
        if self.find_method(class_id, "__toString").is_err() {
            return Ok(None);
        }
        let mut scope = variables.clone();
        scope.insert("#stringified".to_string(), Value { slot: None, ..value.clone() });
        let call = Expr::MethodCall(Box::new(Expr::Variable("#stringified".to_string())), "__toString".to_string(), Vec::new());
        self.emit_expression(&call, &scope, body).map(Some)
    }

    /// `==` and `!=` on two arrays or two maps of the same type: equal when
    /// they hold equal elements in the same order, or the same keys with
    /// equal values in any order (qiq_values_equal in map.c). Their elements
    /// can't be objects or functions, which can only be compared with null,
    /// so a value can't contain itself and the comparison always ends.
    fn emit_contents_comparison(&mut self, left: Value, operator: CompareOp, right: Value, body: &mut String) -> Result<Value, String> {
        if !matches!(operator, CompareOp::Equal | CompareOp::NotEqual) {
            return Err(format!("{} values support only == and != comparisons", self.type_name(left.value_type)));
        }
        if !self.compares_by_contents(left.value_type) {
            return Err(format!(
                "{} values can't be compared: objects and functions in them can only be compared with null",
                self.type_name(left.value_type)
            ));
        }
        let descriptor = self.type_descriptor(left.value_type);
        let descriptor = self.string_literal(descriptor.as_bytes());
        let equal = self.new_register();
        body.push_str(&format!(
            "  {equal} = call zeroext i1 @qiq_values_equal(ptr {descriptor}, ptr {}, ptr {})\n",
            left.operand, right.operand
        ));
        if matches!(operator, CompareOp::Equal) {
            return Ok(Value { value_type: ValueType::Bool, operand: equal, slot: None });
        }
        let different = self.new_register();
        body.push_str(&format!("  {different} = xor i1 {equal}, true\n"));
        Ok(Value { value_type: ValueType::Bool, operand: different, slot: None })
    }

    /// Whether `==` can compare values of this type: numbers, bools, strings,
    /// chars, and arrays and maps of those, at any depth.
    fn compares_by_contents(&self, value_type: ValueType) -> bool {
        match value_type {
            ValueType::Array(_) => self.compares_by_contents(self.element_of(value_type).expect("array type")),
            ValueType::Map(type_id) => {
                let map = self.map_types[type_id as usize];
                self.compares_by_contents(map.key_type) && self.compares_by_contents(map.value_type)
            }
            ValueType::Int | ValueType::Double | ValueType::Bool | ValueType::Byte | ValueType::String | ValueType::Char => true,
            ValueType::Class(_) | ValueType::Function(_) | ValueType::Null | ValueType::Void => false,
        }
    }

    /// Notes that the program uses `class_id` (and so its ancestors' methods).
    fn use_class(&mut self, class_id: u32) {
        let mut current = Some(class_id);
        while let Some(id) = current {
            if self.used_classes.contains(&id) {
                return;
            }
            self.used_classes.push(id);
            current = self.classes[id as usize].parent_class;
        }
    }

    /// Records `value` as an owned temporary when it is a reference: a value
    /// the program just created or retained, released at the end of the
    /// statement unless something takes it.
    fn own(&mut self, value: Value) -> Value {
        if is_reference(value.value_type) && value.operand.starts_with('%') {
            self.temporaries.last_mut().expect("a scope is open").push(value.operand.clone());
        }
        value
    }

    /// One reference to `value`, to store somewhere that keeps it: an owned
    /// temporary is handed over, anything else (a variable, a parameter) is
    /// retained. Literals and null need neither.
    fn take(&mut self, value: &Value, body: &mut String) -> String {
        if !is_reference(value.value_type) || !value.operand.starts_with('%') {
            return value.operand.clone();
        }
        let scope = self.temporaries.last_mut().expect("a scope is open");
        if let Some(position) = scope.iter().rposition(|operand| *operand == value.operand) {
            scope.remove(position);
        } else {
            body.push_str(&format!("  call void @qiq_retain(ptr {})\n", value.operand));
        }
        value.operand.clone()
    }

    /// Retains a reference read from memory the program can change (a
    /// property, element, map entry, or static property) and owns it, so it
    /// stays valid for the rest of the statement.
    fn retained(&mut self, value: Value, body: &mut String) -> Value {
        if is_reference(value.value_type) {
            body.push_str(&format!("  call void @qiq_retain(ptr {})\n", value.operand));
        }
        self.own(value)
    }

    /// Emits `expression` for a caller that uses its value straight away,
    /// before any other code runs, in a runtime call that neither keeps it
    /// nor releases anything (a map lookup's key, an operand of `+` or a
    /// string comparison, len's argument). An element or property read is
    /// then borrowed instead of retained: nothing can replace it in its
    /// array, map, or object before that use, and the container itself is a
    /// variable or an owned temporary that outlives the statement. Saves a
    /// retain and a release, which both write to the value's header, for
    /// each such read: `m[keys[i]]` in a loop, `a[i] == b[j]`. Something that
    /// keeps a borrowed value still gets its own reference from `take`.
    fn emit_borrowed(
        &mut self,
        expression: &Expr,
        expected: Option<ValueType>,
        variables: &HashMap<String, Value>,
        body: &mut String,
    ) -> Result<Value, String> {
        // Only the read itself is borrowed: Index and Member clear the flag
        // before emitting their own operands.
        self.borrow_read = matches!(expression, Expr::Index(..) | Expr::Member(..));
        let value = match expected {
            None => self.emit_expression(expression, variables, body),
            Some(_) => self.emit_expecting(expression, expected, variables, body),
        };
        self.borrow_read = false;
        value
    }

    fn open_scope(&mut self) {
        self.temporaries.push(Vec::new());
    }

    /// Releases the innermost scope's temporaries and closes it.
    fn close_scope(&mut self, body: &mut String) {
        let scope = self.temporaries.pop().expect("a scope is open");
        for operand in scope.iter().rev() {
            body.push_str(&format!("  call void @qiq_release(ptr {operand})\n"));
        }
    }

    /// Releases the innermost scope's temporaries, keeping it open: for
    /// conditions, whose values are used up before the statement branches.
    fn release_current(&mut self, body: &mut String) {
        let scope = std::mem::take(self.temporaries.last_mut().expect("a scope is open"));
        for operand in scope.iter().rev() {
            body.push_str(&format!("  call void @qiq_release(ptr {operand})\n"));
        }
    }

    /// Releases every open scope's temporaries without closing them, for a
    /// return: the path leaves the function, while the scopes stay open for
    /// the code after the return's block.
    fn release_all(&mut self, body: &mut String) {
        for scope in self.temporaries.iter().rev() {
            for operand in scope.iter().rev() {
                body.push_str(&format!("  call void @qiq_release(ptr {operand})\n"));
            }
        }
    }

    /// A stack slot that owns a reference: null at the start of the
    /// function, released before every `ret`.
    fn owned_slot(&mut self) -> String {
        let slot = self.new_register();
        self.entry_allocas
            .push_str(&format!("  {slot} = alloca ptr, align 8\n  store ptr null, ptr {slot}, align 8\n"));
        self.owned_slots.push(slot.clone());
        slot
    }

    /// Stores `operand` (one reference, already taken) into `slot`,
    /// releasing the reference the slot held before.
    fn store_owned(&mut self, slot: &str, operand: &str, body: &mut String) {
        let old = self.new_register();
        body.push_str(&format!(
            "  {old} = load ptr, ptr {slot}, align 8\n  store ptr {operand}, ptr {slot}, align 8\n  call void @qiq_release(ptr {old})\n"
        ));
    }

    /// Keeps a reference to `value` in a hidden slot of its own, for names
    /// bound to values that can't be reassigned (objects, foreach values,
    /// caught exceptions). Each time the code runs, as in a loop, the
    /// previous value is released.
    fn hold(&mut self, value: Value, body: &mut String) -> Value {
        if !is_reference(value.value_type) {
            return value;
        }
        let operand = self.take(&value, body);
        let slot = self.owned_slot();
        self.store_owned(&slot, &operand, body);
        Value { operand, ..value }
    }

    /// Releases the owned slots before each `ret` in a function's IR.
    fn release_slots_before_returns(&mut self, function_ir: &str) -> String {
        if self.owned_slots.is_empty() {
            return function_ir.to_string();
        }
        let mut output = String::with_capacity(function_ir.len());
        for line in function_ir.lines() {
            if line.starts_with("  ret ") {
                for slot in self.owned_slots.clone() {
                    let id = self.next_release;
                    self.next_release += 1;
                    output.push_str(&format!(
                        "  %release{id} = load ptr, ptr {slot}, align 8\n  call void @qiq_release(ptr %release{id})\n"
                    ));
                }
            }
            output.push_str(line);
            output.push('\n');
        }
        output
    }

    /// Binds a named variable: in a cell when an anonymous function captures
    /// it, otherwise in a stack slot.
    fn bind_name(&mut self, name: &str, value: Value, body: &mut String) -> Value {
        if self.boxed.contains(name) && value.value_type != ValueType::Null {
            return self.new_cell(value, false, body);
        }
        self.bind_variable(value, body)
    }

    /// For foreach and catch variables (which can't be reassigned): a
    /// read-only cell when captured, otherwise the value as bound.
    fn bind_read_only(&mut self, name: &str, value: Value, body: &mut String) -> Value {
        if self.boxed.contains(name) {
            return self.new_cell(value, true, body);
        }
        value
    }

    /// The synthetic class of the cell that holds one captured value of
    /// `value_type`: an object with one field, so cells are reference
    /// counted and cycle collected like any object.
    fn cell_class(&mut self, value_type: ValueType) -> u32 {
        if let Some((_, class_id)) = self.cell_classes.iter().find(|(candidate, _)| *candidate == value_type) {
            return *class_id;
        }
        let class_id = self.add_synthetic_class(
            "{cell}",
            vec![Property { name: "value".to_string(), value_type, visibility: Visibility::Private, default: None, default_at: (0, 0), readonly: false }],
        );
        self.cell_classes.push((value_type, class_id));
        class_id
    }

    fn add_synthetic_class(&mut self, name: &str, properties: Vec<Property>) -> u32 {
        let class_id = self.classes.len() as u32;
        self.classes.push(ClassDef {
            name: name.to_string(),
            parent_class: None,
            interfaces: Vec::new(),
            is_abstract: false,
            is_final: true,
            is_interface: false,
            module: 0,
            symbol: format!("synthetic{class_id}"),
            properties,
            static_properties: Vec::new(),
            constants: Vec::new(),
            methods: Vec::new(),
            line: 0,
            column: 0,
        });
        class_id
    }

    /// A new cell holding `value` (one reference to it), kept in an owned
    /// slot; the returned variable reads and writes through it.
    fn new_cell(&mut self, value: Value, read_only: bool, body: &mut String) -> Value {
        let value_type = value.value_type;
        let class_id = self.cell_class(value_type);
        let operand = self.take(&value, body);
        let bits = if operand == "null" {
            "0".to_string()
        } else {
            self.encode_payload(&Value { value_type, operand, slot: None }, body).expect("cells hold storable values")
        };
        let cell = self.new_register();
        let at = self.new_register();
        body.push_str(&format!(
            "  {cell} = call nonnull ptr @qiq_object_new(i64 {}, i64 {class_id})\n  {at} = getelementptr inbounds i64, ptr {cell}, i64 {OBJECT_HEADER_FIELDS}\n  store i64 {bits}, ptr {at}, align 8\n",
            OBJECT_HEADER_FIELDS + 1
        ));
        let slot = self.owned_slot();
        self.store_owned(&slot, &cell, body);
        let prefix = if read_only { "cellro" } else { "cell" };
        Value { value_type, operand: String::new(), slot: Some(format!("{prefix}:{slot}")) }
    }

    /// Replaces a cell's value, releasing the one it held.
    fn store_in_cell(&mut self, cell: &str, value_type: ValueType, value: Value, body: &mut String) -> Result<(), String> {
        let operand = self.take(&value, body);
        let bits = if operand == "null" {
            "0".to_string()
        } else {
            self.encode_payload(&Value { value_type, operand, slot: None }, body)?
        };
        let at = self.new_register();
        body.push_str(&format!("  {at} = getelementptr inbounds i64, ptr {cell}, i64 {OBJECT_HEADER_FIELDS}\n"));
        if is_reference(value_type) {
            let old = self.new_register();
            body.push_str(&format!(
                "  {old} = load ptr, ptr {at}, align 8\n  store i64 {bits}, ptr {at}, align 8\n  call void @qiq_release(ptr {old})\n"
            ));
        } else {
            body.push_str(&format!("  store i64 {bits}, ptr {at}, align 8\n"));
        }
        Ok(())
    }

    /// For a variable in a cell: the register holding the cell, and whether
    /// the variable is read-only. None for ordinary variables.
    fn cell_of(&mut self, slot: &str, body: &mut String) -> Option<(String, bool)> {
        let (prefix, register) = slot.split_once(':')?;
        match prefix {
            "cell" | "cellro" => {
                let cell = self.new_register();
                body.push_str(&format!("  {cell} = load ptr, ptr {register}, align 8\n"));
                Some((cell, prefix == "cellro"))
            }
            "cellptr" | "cellptrro" => Some((register.to_string(), prefix == "cellptrro")),
            _ => None,
        }
    }

    /// Binds a named variable to a stack slot (allocated in the entry block,
    /// initialized at the point of declaration), so it can be reassigned.
    fn bind_variable(&mut self, value: Value, body: &mut String) -> Value {
        if is_reference(value.value_type) {
            // The variable holds its own reference, in a slot released when
            // the function returns (or when this declaration runs again).
            let operand = self.take(&value, body);
            let slot = self.owned_slot();
            self.store_owned(&slot, &operand, body);
            return Value {
                value_type: value.value_type,
                operand: String::new(),
                slot: Some(slot),
            };
        }
        let reassignable = value.value_type.is_scalar()
            || matches!(value.value_type, ValueType::Array(_))
            || matches!(value.value_type, ValueType::Map(_))
            || value.value_type == ValueType::Null;
        if !reassignable {
            return value;
        }
        let llvm = value.value_type.llvm();
        let slot = self.new_register();
        self.entry_allocas
            .push_str(&format!("  {slot} = alloca {llvm}, align 8\n"));
        body.push_str(&format!(
            "  store {llvm} {}, ptr {slot}, align 8\n",
            value.operand
        ));
        Value {
            value_type: value.value_type,
            operand: String::new(),
            slot: Some(slot),
        }
    }

    /// A call to a `void` function evaluates to `null`.
    fn call_result_type(return_type: ValueType) -> ValueType {
        if return_type == ValueType::Void {
            ValueType::Null
        } else {
            return_type
        }
    }

    /// Name of the dispatch table for `owner::method_name`; tables of default-
    /// filling adapters get the adapters' suffix (see `adapter_suffix`).
    fn dispatch_table_name(&self, owner: u32, method_name: &str, supplied: Option<&[usize]>) -> String {
        let suffix = supplied.map(Self::adapter_suffix).unwrap_or_default();
        format!("qiq_dispatch_{}_{method_name}{suffix}", self.classes[owner as usize].symbol)
    }

    /// Emits a constant table mapping each class id to the implementation of
    /// `owner::method_name` for that class, or null when the class cannot be
    /// instantiated or is not a subtype of `owner`.
    fn emit_dispatch_table(&self, owner: u32, method_name: &str, supplied: Option<&[usize]>) -> String {
        let suffix = supplied.map(Self::adapter_suffix).unwrap_or_default();
        let entries: Vec<String> = (0..self.classes.len() as u32)
            .map(|class_id| {
                let class = &self.classes[class_id as usize];
                let is_subtype = class_id == owner || self.is_subclass(class_id, owner);
                match self.find_implementation(class_id, method_name) {
                    Some(implementer) if is_subtype && !class.is_abstract => format!(
                        "ptr @qiq_{}_{method_name}{suffix}",
                        self.classes[implementer as usize].symbol
                    ),
                    _ => "ptr null".to_string(),
                }
            })
            .collect();
        format!(
            "@{} = private unnamed_addr constant [{} x ptr] [{}], align 8\n",
            self.dispatch_table_name(owner, method_name, supplied),
            entries.len(),
            entries.join(", ")
        )
    }

    /// Whether a subclass of `owner` declares its own `name` method.
    fn is_overridden(&self, owner: u32, name: &str) -> bool {
        (0..self.classes.len() as u32).any(|class_id| {
            class_id != owner
                && self.is_subclass(class_id, owner)
                && self.classes[class_id as usize].methods.iter().any(|method| method.name == name)
        })
    }

    /// Returns the class that provides the concrete implementation of `name`
    /// for instances of `class_id`, searching the class and its superclasses.
    fn find_implementation(&self, class_id: u32, name: &str) -> Option<u32> {
        let mut current = Some(class_id);
        while let Some(current_id) = current {
            let class = &self.classes[current_id as usize];
            if let Some(method) = class.methods.iter().find(|method| method.name == name) {
                return (!method.is_abstract).then_some(current_id);
            }
            current = class.parent_class;
        }
        None
    }

    fn find_property(
        &self,
        class_id: u32,
        name: &str,
    ) -> Result<(usize, ValueType, Visibility, u32), String> {
        let class = self
            .classes
            .get(class_id as usize)
            .ok_or_else(|| "invalid class type".to_string())?;
        let parent_fields = match class.parent_class {
            Some(parent_id) => self.class_field_count(parent_id)?,
            None => 0,
        };
        if let Some((index, property)) = class
            .properties
            .iter()
            .enumerate()
            .find(|(_, property)| property.name == name)
        {
            return Ok((
                OBJECT_HEADER_FIELDS + parent_fields + index,
                property.value_type,
                property.visibility,
                class_id,
            ));
        }
        if let Some(parent_id) = class.parent_class {
            return self.find_property(parent_id, name);
        }
        Err(format!("class '{}' has no property '{name}'", class.name))
    }

    fn class_field_count(&self, class_id: u32) -> Result<usize, String> {
        let class = self
            .classes
            .get(class_id as usize)
            .ok_or_else(|| "invalid class type".to_string())?;
        let inherited = match class.parent_class {
            Some(parent_id) => self.class_field_count(parent_id)?,
            None => 0,
        };
        Ok(inherited + class.properties.len())
    }

    fn check_visibility(
        &self,
        visibility: Visibility,
        owner_class: u32,
        member_name: &str,
    ) -> Result<(), String> {
        if visibility == Visibility::Public
            || self.current_class == Some(owner_class)
            || (visibility == Visibility::Protected
                && self.current_class.is_some_and(|current| self.is_subclass(current, owner_class)))
        {
            Ok(())
        } else {
            Err(format!("cannot access non-public member '{member_name}'"))
        }
    }

    /// True when `ancestor_id` is a superclass of `class_id` or an interface it
    /// implements (directly, through a superclass, or through interface extension).
    fn is_subclass(&self, class_id: u32, ancestor_id: u32) -> bool {
        let Some(class) = self.classes.get(class_id as usize) else {
            return false;
        };
        class
            .parent_class
            .iter()
            .chain(class.interfaces.iter())
            .any(|&super_id| super_id == ancestor_id || self.is_subclass(super_id, ancestor_id))
    }

    fn is_assignable(&self, expected: ValueType, actual: ValueType) -> bool {
        expected == actual
            || match (expected, actual) {
                (ValueType::Class(parent), ValueType::Class(child)) => {
                    self.is_subclass(child, parent)
                }
                _ => false,
            }
    }

    fn find_method(
        &self,
        class_id: u32,
        name: &str,
    ) -> Result<(Vec<Parameter>, ValueType, Visibility, u32, bool), String> {
        let class = self
            .classes
            .get(class_id as usize)
            .ok_or_else(|| "invalid class type".to_string())?;
        if let Some(method) = class
            .methods
            .iter()
            .find(|method| method.name == name)
        {
            return Ok((
                method.parameters.clone(),
                method.return_type,
                method.visibility,
                class_id,
                method.is_abstract,
            ));
        }
        if let Some(parent_id) = class.parent_class {
            if let Ok(found) = self.find_method(parent_id, name) {
                return Ok(found);
            }
        }
        for &interface_id in &class.interfaces {
            if let Ok(found) = self.find_method(interface_id, name) {
                return Ok(found);
            }
        }
        Err(format!("class '{}' has no method '{name}'", class.name))
    }

    /// The class a static:: or self:: call was made on, at run time: the
    /// hidden parameter in a static method, the object's class otherwise.
    fn emit_called_class(&mut self, body: &mut String) -> Result<String, String> {
        if self.current_class.is_none() {
            return Err("static:: can only be used inside a class method".to_string());
        }
        if self.in_closure && !self.closure_has_this {
            return Err("self::, parent::, and static:: calls that depend on the called class aren't supported in anonymous functions inside static methods".to_string());
        }
        if self.current_method_is_static {
            return Ok("%called".to_string());
        }
        let register = self.new_register();
        body.push_str(&format!("  {register} = load i64, ptr %this, align 8\n"));
        Ok(register)
    }

    /// The class a StaticTarget is checked against at compile time.
    /// The constant `name` of a class, from it, a parent, or an interface:
    /// the class or interface declaring it, and the constant.
    fn find_constant(&self, class_id: u32, name: &str) -> Option<(u32, Constant)> {
        let class = self.classes.get(class_id as usize)?;
        if let Some(constant) = class.constants.iter().find(|constant| constant.name == name) {
            return Some((class_id, constant.clone()));
        }
        class
            .parent_class
            .iter()
            .chain(class.interfaces.iter())
            .find_map(|&super_id| self.find_constant(super_id, name))
    }

    /// A constant's value, as a literal, and its type. The value is a
    /// literal or another constant; `depth` stops a constant that refers to
    /// itself.
    fn constant_value(&self, owner: u32, constant: &Constant, depth: usize) -> Result<(Expr, ValueType), String> {
        let at = format!("{}:{}: ", constant.line, constant.column);
        if depth > 64 {
            return Err(format!("{at}constant '{}' is defined in terms of itself", constant.name));
        }
        let (literal, literal_type) = match &constant.value {
            Expr::Number(_) => (constant.value.clone(), ValueType::Int),
            Expr::Decimal(_) => (constant.value.clone(), ValueType::Double),
            Expr::Negate(inner) if matches!(**inner, Expr::Decimal(_)) => {
                let Expr::Decimal(value) = **inner else { unreachable!() };
                (Expr::Decimal(-value), ValueType::Double)
            }
            Expr::Boolean(_) => (constant.value.clone(), ValueType::Bool),
            Expr::Text(_) => (constant.value.clone(), ValueType::String),
            Expr::Character(_) => (constant.value.clone(), ValueType::Char),
            Expr::StaticProperty(target, name) => {
                let class_id = Self::static_target_class(*target);
                let (other_owner, other) = self.find_constant(class_id, name).ok_or_else(|| {
                    format!("{at}'{}::{name}' isn't a constant: a constant's value must be a literal or another constant", self.classes[class_id as usize].name)
                })?;
                if other.visibility == Visibility::Private && other_owner != owner {
                    return Err(format!("{at}constant '{name}' is private to '{}'", self.classes[other_owner as usize].name));
                }
                self.constant_value(other_owner, &other, depth + 1)?
            }
            _ => {
                return Err(format!(
                    "{at}constant '{}' must be a literal (int, double, bool, string, or char) or another constant",
                    constant.name
                ))
            }
        };
        match constant.declared {
            None => Ok((literal, literal_type)),
            Some(ValueType::Byte) => match literal {
                Expr::Number(number) if (0..=255).contains(&number) => Ok((literal, ValueType::Byte)),
                _ => Err(format!("{at}constant '{}' must be a byte (0 to 255)", constant.name)),
            },
            Some(declared) if self.is_assignable(declared, literal_type) && declared.is_scalar() => Ok((literal, declared)),
            Some(declared) if !declared.is_scalar() => Err(format!(
                "{at}constant '{}' can't be {}: constants are ints, doubles, bools, bytes, strings, or chars",
                constant.name,
                self.type_name(declared)
            )),
            Some(declared) => Err(format!(
                "{at}constant '{}' is declared {}, but its value is {}",
                constant.name,
                self.type_name(declared),
                self.type_name(literal_type)
            )),
        }
    }

    /// Checks that `default`, a property's default, is a constant of type
    /// `expected`: a literal, a class constant, null for a reference, or an
    /// array or map literal of those.
    fn check_constant_value(&self, default: &Expr, expected: ValueType, owner: u32, at: &str, name: &str) -> Result<(), String> {
        let mismatch = |this: &Self, found: ValueType| {
            Err(format!(
                "{at}the default for '{name}' must be {}, found {}",
                this.type_name(expected),
                this.type_name(found)
            ))
        };
        let scalar = |this: &Self, found: ValueType| {
            if this.is_assignable(expected, found) { Ok(()) } else { mismatch(this, found) }
        };
        match default {
            Expr::Number(number) if expected == ValueType::Byte => {
                if (0..=255).contains(number) {
                    Ok(())
                } else {
                    Err(format!("{at}the default for '{name}' must be a byte (0 to 255), found {number}"))
                }
            }
            Expr::Number(_) => scalar(self, ValueType::Int),
            Expr::Decimal(_) => scalar(self, ValueType::Double),
            Expr::Negate(inner) if matches!(**inner, Expr::Decimal(_)) => scalar(self, ValueType::Double),
            Expr::Boolean(_) => scalar(self, ValueType::Bool),
            Expr::Text(_) => scalar(self, ValueType::String),
            Expr::Character(_) => scalar(self, ValueType::Char),
            Expr::Null if is_reference(expected) => Ok(()),
            Expr::Null => mismatch(self, ValueType::Null),
            Expr::StaticProperty(target, constant_name) => {
                let class_id = Self::static_target_class(*target);
                let Some((constant_owner, constant)) = self.find_constant(class_id, constant_name) else {
                    return Err(format!("{at}the default for '{name}' must be a constant, and '{constant_name}' isn't one"));
                };
                if constant.visibility == Visibility::Private && constant_owner != owner {
                    return Err(format!("{at}constant '{constant_name}' is private to '{}'", self.classes[constant_owner as usize].name));
                }
                let (_, found) = self.constant_value(constant_owner, &constant, 0)?;
                scalar(self, found)
            }
            Expr::ArrayLiteral(elements) => match self.element_of(expected) {
                Some(element) => {
                    for item in elements {
                        self.check_constant_value(item, element, owner, at, name)?;
                    }
                    Ok(())
                }
                None => Err(format!("{at}the default for '{name}' must be {}, found an array", self.type_name(expected))),
            },
            Expr::MapLiteral(entries) => match expected {
                ValueType::Map(type_id) => {
                    let map = self.map_types[type_id as usize];
                    for (key, value) in entries {
                        self.check_constant_value(key, map.key_type, owner, at, name)?;
                        self.check_constant_value(value, map.value_type, owner, at, name)?;
                    }
                    Ok(())
                }
                _ => Err(format!("{at}the default for '{name}' must be {}, found a map", self.type_name(expected))),
            },
            _ => Err(format!(
                "{at}the default for '{name}' must be a constant: a literal, a class constant, or an array or map of those (set other values in the constructor)"
            )),
        }
    }

    /// Checks every class's constants and property defaults, so mistakes are
    /// reported where they're written, whether or not the class is used.
    fn validate_constants_and_defaults(&self) -> Result<(), String> {
        for class_id in 0..self.classes.len() as u32 {
            let class = &self.classes[class_id as usize];
            for constant in &class.constants {
                let at = format!("{}:{}: ", constant.line, constant.column);
                let checked = (|| {
                    self.constant_value(class_id, constant, 0)?;
                    // Not redeclared: a subclass sees its parents' constants,
                    // so `static::NAME` and `self::NAME` always agree.
                    for &super_id in class.parent_class.iter().chain(class.interfaces.iter()) {
                        if let Some((owner, _)) = self.find_constant(super_id, &constant.name) {
                            return Err(format!(
                                "{at}constant '{}' is already declared in '{}'",
                                constant.name,
                                self.classes[owner as usize].name
                            ));
                        }
                    }
                    if let Some((owner, _)) = class.parent_class.and_then(|parent| self.find_static_property(parent, &constant.name)) {
                        return Err(format!(
                            "{at}'{}' is already a static property of '{}'",
                            constant.name,
                            self.classes[owner as usize].name
                        ));
                    }
                    Ok(())
                })();
                checked.map_err(|error| self.in_module(class.module, error))?;
            }
            if let Some(method) = class.methods.iter().find(|method| method.name == "__toString") {
                let wrong = method.visibility != Visibility::Public
                    || method.is_static
                    || !method.parameters.is_empty()
                    || method.return_type != ValueType::String;
                if wrong {
                    return Err(self.in_module(
                        class.module,
                        format!(
                            "{}:{}: __toString must be declared `public function __toString(): string`, with no parameters",
                            method.line, method.column
                        ),
                    ));
                }
            }
            for property in &class.properties {
                if let Some(default) = &property.default {
                    let at = format!("{}:{}: ", property.default_at.0, property.default_at.1);
                    self.check_constant_value(default, property.value_type, class_id, &at, &property.name)
                        .map_err(|error| self.in_module(class.module, error))?;
                }
            }
        }
        Ok(())
    }

    /// Whether objects of `class_id` have property defaults to set (its own
    /// or inherited), in qiq_defaults.<class>.
    fn has_property_defaults(&self, class_id: u32) -> bool {
        self.class_fields(class_id).iter().any(|(property, _)| property.default.is_some())
    }

    /// `define void @qiq_defaults.<class>(ptr %object)`: stores the default
    /// of each property that has one (inherited ones too) in a new object,
    /// before its constructor runs. Each object gets its own arrays and maps.
    fn emit_defaults_function(&mut self, class_id: u32) -> Result<String, String> {
        let mut body = String::new();
        let saved_method = self.current_method.take();
        for (property, slot) in self.class_fields(class_id) {
            let Some(default) = &property.default else { continue };
            let value = self.emit_expecting(default, Some(property.value_type), &HashMap::new(), &mut body)?;
            if is_reference(property.value_type) {
                self.take(&value, &mut body);
            }
            let encoded = self.encode_payload(&value, &mut body)?;
            let address = self.new_register();
            body.push_str(&format!(
                "  {address} = getelementptr inbounds i64, ptr %object, i64 {slot}\n  store i64 {encoded}, ptr {address}, align 8\n"
            ));
        }
        self.current_method = saved_method;
        Ok(format!(
            "define void @qiq_defaults.{}(ptr %object) {{\nentry:\n{body}  ret void\nunwind:\n  ret void\n}}\n",
            self.classes[class_id as usize].symbol
        ))
    }

    fn static_target_class(target: StaticTarget) -> u32 {
        match target {
            StaticTarget::Class { class_id, .. } => class_id,
            StaticTarget::Parent { parent, .. } => parent,
            StaticTarget::Late { enclosing } => enclosing,
        }
    }

    /// The class (this one or a superclass) that declares static property `name`.
    fn find_static_property(&self, class_id: u32, name: &str) -> Option<(u32, StaticProperty)> {
        let mut current = Some(class_id);
        while let Some(current_id) = current {
            let class = &self.classes[current_id as usize];
            if let Some(property) = class.static_properties.iter().find(|property| property.name == name) {
                return Some((current_id, property.clone()));
            }
            current = class.parent_class;
        }
        None
    }

    /// Classes that `class_id` code can run on: itself and its subclasses.
    fn subtypes_of(&self, class_id: u32) -> Vec<u32> {
        (0..self.classes.len() as u32)
            .filter(|candidate| {
                !self.classes[*candidate as usize].is_interface
                    && (*candidate == class_id || self.is_subclass(*candidate, class_id))
            })
            .collect()
    }

    /// Address of a static property's storage, and its declaration.
    fn emit_static_property_address(
        &mut self,
        target: StaticTarget,
        name: &str,
        body: &mut String,
    ) -> Result<(String, StaticProperty), String> {
        let class_id = Self::static_target_class(target);
        let (owner, property) = self.find_static_property(class_id, name).ok_or_else(|| {
            format!("class '{}' has no constant or static property '{name}'", self.classes[class_id as usize].name)
        })?;
        self.check_visibility(property.visibility, owner, name)?;
        let address = match target {
            StaticTarget::Late { enclosing } => {
                let key = (enclosing, name.to_string());
                if !self.late_property_tables.contains(&key) {
                    self.late_property_tables.push(key);
                }
                let called = self.emit_called_class(body)?;
                let slot = self.new_register();
                let address = self.new_register();
                body.push_str(&format!(
                    "  {slot} = getelementptr inbounds ptr, ptr @qiq_static_slots.{}.{name}, i64 {called}\n  {address} = load ptr, ptr {slot}, align 8\n",
                    self.classes[enclosing as usize].symbol
                ));
                address
            }
            _ => format!("@qiq_static.{}.{name}", self.classes[owner as usize].symbol),
        };
        Ok((address, property))
    }

    /// The class (this one or a superclass) that declares method `name`.
    fn find_declaring_class(&self, class_id: u32, name: &str) -> Option<u32> {
        let mut current = Some(class_id);
        while let Some(current_id) = current {
            let class = &self.classes[current_id as usize];
            if class.methods.iter().any(|method| method.name == name) {
                return Some(current_id);
            }
            current = class.parent_class;
        }
        None
    }

    /// Finds a method declared by the class or a superclass, ignoring interfaces.
    fn find_class_method(&self, class_id: u32, name: &str) -> Option<&Function> {
        let mut current = Some(class_id);
        while let Some(current_id) = current {
            let class = &self.classes[current_id as usize];
            if let Some(method) = class.methods.iter().find(|method| method.name == name) {
                return Some(method);
            }
            current = class.parent_class;
        }
        None
    }

    /// Collects every interface the class implements, including those inherited
    /// from superclasses and those extended by other interfaces.
    fn collect_interfaces(&self, class_id: u32, found: &mut Vec<u32>) {
        let class = &self.classes[class_id as usize];
        for &interface_id in &class.interfaces {
            if !found.contains(&interface_id) {
                found.push(interface_id);
                self.collect_interfaces(interface_id, found);
            }
        }
        if let Some(parent_id) = class.parent_class {
            self.collect_interfaces(parent_id, found);
        }
    }

    fn validate_class_contracts(&self) -> Result<(), String> {
        for (class_id, class) in self.classes.iter().enumerate() {
            // Errors not about one method are about the class.
            self.validate_class_contract(class_id as u32, class)
                .map_err(|error| self.in_module(class.module, at(class.line, class.column, error)))?;
        }
        Ok(())
    }

    /// An overriding or implementing method may change defaults and may make a
    /// required parameter optional, but must not make an optional one required:
    /// a call that omits it through the parent type would have no value for it.
    fn check_keeps_optional(class: &ClassDef, method: &Function, inherited: &[Parameter]) -> Result<(), String> {
        for (own, parent) in method.parameters.iter().zip(inherited) {
            if parent.default.is_some() && own.default.is_none() {
                return Err(format!(
                    "method '{}::{}' cannot make optional parameter '{}' required",
                    class.name, method.name, own.name
                ));
            }
        }
        Ok(())
    }

    /// Matches a call's arguments to `parameters`: positional arguments by
    /// position, named ones by name. Returns (parameter index, expression)
    /// pairs in the order written. Omitted parameters must have defaults.
    fn resolve_arguments<'e>(
        parameters: &[Parameter],
        arguments: &'e [Expr],
        callee: &str,
    ) -> Result<Vec<(usize, &'e Expr)>, String> {
        let required = parameters.iter().filter(|parameter| parameter.default.is_none()).count();
        let positional = arguments
            .iter()
            .filter(|argument| !matches!(argument, Expr::NamedArgument(..)))
            .count();
        let has_named = positional < arguments.len();
        if positional > parameters.len() || (!has_named && positional < required) {
            let expected = if required == parameters.len() {
                required.to_string()
            } else {
                format!("{required} to {}", parameters.len())
            };
            return Err(format!(
                "{callee} expects {expected} argument(s), found {}",
                arguments.len()
            ));
        }
        let mut resolved = Vec::new();
        for (position, argument) in arguments.iter().enumerate() {
            match argument {
                Expr::NamedArgument(name, value) => {
                    let index = parameters
                        .iter()
                        .position(|parameter| parameter.name == *name)
                        .ok_or_else(|| format!("{callee} has no parameter named '{name}'"))?;
                    if resolved.iter().any(|(existing, _)| *existing == index) {
                        return Err(format!(
                            "{callee}: parameter '{name}' is already given by position"
                        ));
                    }
                    resolved.push((index, value.as_ref()));
                }
                positional => resolved.push((position, positional)),
            }
        }
        if let Some(missing) = parameters.iter().enumerate().find(|(index, parameter)| {
            parameter.default.is_none() && !resolved.iter().any(|(given, _)| given == index)
        }) {
            return Err(format!(
                "{callee}: missing argument for required parameter '{}'",
                missing.1.name
            ));
        }
        Ok(resolved)
    }

    /// Evaluates resolved arguments in the order written and returns their
    /// LLVM operands in parameter order. Omitted parameters are filled with their defaults when
    /// `fill_defaults` is set; otherwise they are left out (a dispatched call,
    /// whose adapter fills them in). `mismatch` prefixes type errors.
    #[allow(clippy::too_many_arguments)]
    fn emit_arguments(
        &mut self,
        parameters: &[Parameter],
        resolved: &[(usize, &Expr)],
        fill_defaults: bool,
        _callee: &str,
        mismatch: &str,
        variables: &HashMap<String, Value>,
        body: &mut String,
    ) -> Result<Vec<String>, String> {
        let mut values: Vec<Option<Value>> = vec![None; parameters.len()];
        for (index, expression) in resolved {
            let parameter = &parameters[*index];
            let value = self.emit_expecting(expression, Some(parameter.value_type), variables, body)?;
            if !self.is_assignable(parameter.value_type, value.value_type) {
                return Err(format!(
                    "{mismatch}: expected {}, found {}",
                    self.type_name(parameter.value_type),
                    self.type_name(value.value_type)
                ));
            }
            values[*index] = Some(value);
        }
        let mut operands = Vec::new();
        for (index, parameter) in parameters.iter().enumerate() {
            let value = match values[index].take() {
                Some(value) => value,
                None if fill_defaults => {
                    let default = parameter.default.as_ref().expect("omitted parameters have defaults");
                    self.emit_expression(default, variables, body)?
                }
                None => continue,
            };
            operands.push(format!("{} {}", parameter.value_type.llvm(), value.operand));
        }
        Ok(operands)
    }

    /// Symbol suffix for an adapter taking the parameters in `supplied`:
    /// `.<count>` when they are the first `count` parameters, otherwise
    /// `.s<indices>` (e.g. `.s0_2` for parameters 0 and 2).
    fn adapter_suffix(supplied: &[usize]) -> String {
        if supplied.iter().enumerate().all(|(position, index)| position == *index) {
            format!(".{}", supplied.len())
        } else {
            let indices: Vec<String> = supplied.iter().map(usize::to_string).collect();
            format!(".s{}", indices.join("_"))
        }
    }

    /// Emits an adapter (`@qiq_<Class>_<method>` plus `adapter_suffix`) that
    /// takes the parameters in `supplied`, fills the others with this class's
    /// defaults, and calls the method.
    fn emit_default_adapter(&mut self, class_id: u32, method_name: &str, supplied: &[usize]) -> Result<String, String> {
        let method = self.classes[class_id as usize]
            .methods
            .iter()
            .find(|method| method.name == method_name)
            .cloned()
            .ok_or_else(|| format!("class has no method '{method_name}'"))?;
        self.next_register = 0;
        let mut parameters = vec!["ptr %this".to_string()];
        let mut operands = vec!["ptr %this".to_string()];
        let mut body = String::new();
        for (index, parameter) in method.parameters.iter().enumerate() {
            let llvm = parameter.value_type.llvm();
            if supplied.contains(&index) {
                parameters.push(format!("{llvm} %arg{index}"));
                operands.push(format!("{llvm} %arg{index}"));
            } else {
                let default = parameter.default.as_ref().ok_or_else(|| {
                    format!("method '{method_name}' has no default for parameter '{}'", parameter.name)
                })?;
                let value = self.emit_expression(default, &HashMap::new(), &mut body)?;
                operands.push(format!("{llvm} {}", value.operand));
            }
        }
        let symbol = format!("qiq_{}_{method_name}", self.classes[class_id as usize].symbol);
        let return_llvm = method.return_type.llvm();
        if method.return_type == ValueType::Void {
            body.push_str(&format!("  call void @{symbol}({})\n  ret void\n", operands.join(", ")));
        } else {
            body.push_str(&format!(
                "  %result = call {return_llvm} @{symbol}({})\n  ret {return_llvm} %result\n",
                operands.join(", ")
            ));
        }
        Ok(format!(
            "define {return_llvm} @{symbol}{}({}) {{\nentry:\n{body}}}\n",
            Self::adapter_suffix(supplied),
            parameters.join(", ")
        ))
    }

    /// `to_int`, `to_double`, `to_byte`, and `to_string`: built in rather than
    /// declared in the standard library because each accepts several types.
    /// A call to std/math's abs, min, max, clamp, or number_format, which
    /// take ints or doubles (qiq has no overloading, so the compiler does
    /// this, as it does for len). abs, min, max, and clamp are inline: one or
    /// two instructions per value, and min and max of an array call the
    /// runtime. number_format of an int converts it to a double and then
    /// calls the declared function. Returns None for the other functions.
    fn emit_math_generic(
        &mut self,
        function: &str,
        called: &str,
        arguments: &[Expr],
        variables: &HashMap<String, Value>,
        body: &mut String,
    ) -> Result<Option<Value>, String> {
        if function == "number_format" {
            // The first argument is evaluated once and passed on as a hidden
            // variable. A double goes to the declared function (seeing the
            // variable again means this is that call); an int goes to
            // __number_format_int, which writes every int exactly.
            let Some(first) = arguments.first() else { return Ok(None) };
            if matches!(first, Expr::NamedArgument(..)) || matches!(first, Expr::Variable(name) if name == "#math.number") {
                return Ok(None);
            }
            let value = self.emit_expression(first, variables, body)?;
            let is_int = value.value_type == ValueType::Int;
            let mut scope = variables.clone();
            scope.insert("#math.number".to_string(), Value { slot: None, ..value });
            let mut rest = vec![Expr::Variable("#math.number".to_string())];
            rest.extend_from_slice(&arguments[1..]);
            if !is_int {
                // By the name it was called by, which an import can change.
                return self.emit_expression(&Expr::Call(called.to_string(), rest), &scope, body).map(Some);
            }
            let symbol = "qiq_stdlib___number_format_int".to_string();
            let mut parameters = self.signatures.get("qiq_stdlib_number_format").ok_or("std/math isn't loaded")?.parameters.clone();
            parameters[0].value_type = ValueType::Int;
            if !self.called_functions.contains(&symbol) {
                self.called_functions.push(symbol.clone());
            }
            let callee = format!("function '{called}'");
            let resolved = Self::resolve_arguments(&parameters, &rest, &callee)?;
            let operands = self.emit_arguments(&parameters, &resolved, true, &callee, &format!("argument type mismatch calling '{called}'"), &scope, body)?;
            let register = self.new_register();
            body.push_str(&format!("  {register} = call ptr @{symbol}({})\n", operands.join(", ")));
            self.emit_exception_check(body);
            return Ok(Some(self.own(Value { value_type: ValueType::String, operand: register, slot: None })));
        }
        if !matches!(function, "abs" | "min" | "max" | "clamp") {
            return Ok(None);
        }
        if arguments.iter().any(|argument| matches!(argument, Expr::NamedArgument(..))) {
            return Err(format!("{function} takes no named arguments"));
        }
        let mut values = Vec::new();
        for argument in arguments {
            values.push(self.emit_expression(argument, variables, body)?);
        }
        let number = |this: &Self, value: &Value| -> Result<bool, String> {
            match value.value_type {
                ValueType::Int => Ok(false),
                ValueType::Double => Ok(true),
                other => Err(format!("{function} works on ints and doubles, found {}", this.type_name(other))),
            }
        };
        // min and max of an array.
        if matches!(function, "min" | "max") && values.len() == 1 {
            let element = self.element_of(values[0].value_type);
            let double = match element {
                Some(ValueType::Int) => false,
                Some(ValueType::Double) => true,
                _ => return Err(format!("{function} takes two or more values, or an int[] or double[], found {}", self.type_name(values[0].value_type))),
            };
            let register = self.new_register();
            let (llvm, kind) = if double { ("double", "double") } else { ("i64", "int") };
            body.push_str(&format!("  {register} = call {llvm} @qiq_math_{function}_{kind}(ptr {})\n", values[0].operand));
            self.emit_exception_check(body);
            let value_type = if double { ValueType::Double } else { ValueType::Int };
            return Ok(Some(Value { value_type, operand: register, slot: None }));
        }
        let expected = match function {
            "abs" => "1 value",
            "clamp" => "3 values (x, low, high)",
            _ => "two or more values, or an array",
        };
        let count_ok = match function {
            "abs" => values.len() == 1,
            "clamp" => values.len() == 3,
            _ => values.len() >= 2,
        };
        if !count_ok {
            return Err(format!("{function} takes {expected}, found {}", values.len()));
        }
        let double = number(self, &values[0])?;
        for value in &values[1..] {
            if number(self, value)? != double {
                return Err(format!(
                    "{function}: the values must all be ints or all doubles, found {} and {}",
                    self.type_name(values[0].value_type),
                    self.type_name(value.value_type)
                ));
            }
        }
        let (llvm, less, greater) = if double { ("double", "fcmp olt double", "fcmp ogt double") } else { ("i64", "icmp slt i64", "icmp sgt i64") };
        let value_type = if double { ValueType::Double } else { ValueType::Int };
        // `pick(a, b, comparison)`: b if `b comparison a` holds, else a.
        let pick = |this: &mut Self, a: &str, b: &str, comparison: &str, body: &mut String| -> String {
            let test = this.new_register();
            let result = this.new_register();
            body.push_str(&format!("  {test} = {comparison} {b}, {a}\n  {result} = select i1 {test}, {llvm} {b}, {llvm} {a}\n"));
            result
        };
        let operand = match function {
            "abs" if double => {
                let register = self.new_register();
                body.push_str(&format!("  {register} = call double @llvm.fabs.f64(double {})\n", values[0].operand));
                register
            }
            "abs" => {
                // INT64_MIN stays INT64_MIN, as - wraps.
                let negated = self.new_register();
                body.push_str(&format!("  {negated} = sub i64 0, {}\n", values[0].operand));
                pick(self, &values[0].operand, &negated, greater, body)
            }
            "min" | "max" => {
                let comparison = if function == "min" { less } else { greater };
                let mut best = values[0].operand.clone();
                for value in &values[1..] {
                    best = pick(self, &best, &value.operand, comparison, body);
                }
                best
            }
            _ => {
                // clamp: inline when low <= high; the runtime throws otherwise.
                let (x, low, high) = (&values[0].operand, &values[1].operand, &values[2].operand);
                let register = self.new_register();
                let order = if double { "fcmp ole double" } else { "icmp sle i64" };
                let condition = format!("  %chk.ok# = {order} {low}, {high}\n");
                // max(low, min(x, high)), as one expression for the fast path.
                let inner = self.new_register();
                let inner_test = self.new_register();
                let outer_test = self.new_register();
                body.push_str(&format!(
                    "  {inner_test} = {less} {high}, {x}\n  {inner} = select i1 {inner_test}, {llvm} {high}, {llvm} {x}\n  {outer_test} = {greater} {low}, {inner}\n"
                ));
                let kind = if double { "double" } else { "int" };
                self.emit_fast_path(
                    &register,
                    llvm,
                    &condition,
                    &format!("select i1 {outer_test}, {llvm} {low}, {llvm} {inner}"),
                    &format!("call {llvm} @qiq_math_clamp_{kind}({llvm} {x}, {llvm} {low}, {llvm} {high})"),
                    body,
                );
                register
            }
        };
        Ok(Some(Value { value_type, operand, slot: None }))
    }

    /// An f-string's `{expression}` as text: as to_string converts it, with
    /// an object's __toString, and with null (a null string, char, or
    /// object, or `null` itself) as "null", the way print writes it.
    fn emit_interpolated(&mut self, arguments: &[Expr], variables: &HashMap<String, Value>, body: &mut String) -> Result<Value, String> {
        let [argument] = arguments else { unreachable!("an f-string part has one expression") };
        let value = self.emit_expression(argument, variables, body)?;
        match value.value_type {
            ValueType::Null => self.emit_expression(&Expr::Text("null".to_string()), variables, body),
            ValueType::Class(class_id) if self.find_method(class_id, "__toString").is_ok() => {
                let mut scope = variables.clone();
                scope.insert("#interpolated".to_string(), Value { slot: None, ..value.clone() });
                let call = Expr::Nullsafe(
                    Box::new(Expr::Variable("#interpolated".to_string())),
                    "#interpolated.object".to_string(),
                    Box::new(Expr::MethodCall(Box::new(Expr::Variable("#interpolated.object".to_string())), "__toString".to_string(), Vec::new())),
                );
                self.emit_coalesce(&call, &Expr::Text("null".to_string()), &scope, body)
            }
            ValueType::Class(_) => Err(format!(
                "an f-string can't include a value of type {} unless its class declares `public function __toString(): string`",
                self.type_name(value.value_type)
            )),
            ValueType::String | ValueType::Char => {
                // A null string or char reads "null"; the result borrows
                // either the value (released as usual) or the literal.
                let null = self.emit_expression(&Expr::Text("null".to_string()), variables, body)?;
                let is_null = self.new_register();
                let text = self.new_register();
                let operand = value.operand.clone();
                body.push_str(&format!(
                    "  {is_null} = icmp eq ptr {operand}, null\n  {text} = select i1 {is_null}, ptr {}, ptr {operand}\n",
                    null.operand
                ));
                Ok(Value { value_type: ValueType::String, operand: text, slot: None })
            }
            ValueType::Array(_) | ValueType::Map(_) | ValueType::Function(_) | ValueType::Void => Err(format!(
                "an f-string can't include a value of type {}; convert it to text first, as with implode or json_encode",
                self.type_name(value.value_type)
            )),
            _ => {
                // Converted from the value already computed, not by
                // evaluating the expression again.
                let mut scope = variables.clone();
                scope.insert("#interpolated".to_string(), Value { slot: None, ..value });
                self.emit_conversion("to_string", &[Expr::Variable("#interpolated".to_string())], &scope, body)
            }
        }
    }

    fn emit_conversion(
        &mut self,
        name: &str,
        arguments: &[Expr],
        variables: &HashMap<String, Value>,
        body: &mut String,
    ) -> Result<Value, String> {
        if name == "sprintf" {
            return self.emit_sprintf(arguments, variables, body);
        }
        if name == "__interpolate" {
            return self.emit_interpolated(arguments, variables, body);
        }
        if name == "json_encode" {
            return self.emit_json_encode(arguments, variables, body);
        }
        if name == "range" {
            let (kind, values) = self.emit_range_arguments(arguments, variables, body)?;
            return self.emit_range_array(kind, &values, body);
        }
        if matches!(name, "array_push" | "array_pop" | "array_shift" | "array_unshift" | "array_splice") {
            return self.emit_array_function(name, arguments, variables, body);
        }
        if matches!(name, "map_key_exists" | "map_keys" | "map_values" | "in_map" | "unset") {
            return self.emit_map_function(name, arguments, variables, body);
        }
        if name == "json_decode" {
            return Err(
                "json_decode needs to know the type to decode into: use it where the type is known, such as `User user = json_decode(text);` or `int[] ids = json_decode(text);`"
                    .to_string(),
            );
        }
        let [argument] = arguments else {
            return Err(format!("{name} expects 1 argument, found {}", arguments.len()));
        };
        if matches!(argument, Expr::NamedArgument(..)) {
            return Err(format!("{name} takes no named arguments"));
        }
        if name == "len" {
            let value = self.emit_borrowed(argument, None, variables, body)?;
            return self.emit_len(value, body);
        }
        let value = self.emit_expression(argument, variables, body)?;
        let from = value.value_type;
        let result = |value_type: ValueType, operand: String| Value {
            value_type,
            operand,
            slot: None,
        };
        // A single instruction, or a runtime call that can throw.
        let instruction = |this: &mut Self, text: String, can_throw: bool, body: &mut String| {
            let register = this.new_register();
            body.push_str(&format!("  {register} = {text}\n"));
            if can_throw {
                this.emit_exception_check(body);
            }
            register
        };
        let operand = &value.operand;
        let converted = match (name, from) {
            ("to_int", ValueType::Int)
            | ("to_double", ValueType::Double)
            | ("to_byte", ValueType::Byte)
            | ("to_string", ValueType::String) => return Ok(value),
            ("to_string", ValueType::Char) => return Ok(result(ValueType::String, value.operand)),
            ("to_string", ValueType::Class(_)) => {
                return match self.emit_to_string_call(&value, variables, body)? {
                    Some(text) => Ok(text),
                    None => Err(format!(
                        "to_string of a {} needs the class to declare `public function __toString(): string`",
                        self.type_name(value.value_type)
                    )),
                };
            }
            ("to_int", ValueType::Byte) => result(ValueType::Int, instruction(self, format!("zext i8 {operand} to i64"), false, body)),
            ("to_int", ValueType::Bool) => result(ValueType::Int, instruction(self, format!("zext i1 {operand} to i64"), false, body)),
            ("to_int", ValueType::Double) => {
                // Inline unless out of range or NaN, for which the runtime throws.
                let register = self.new_register();
                let condition = format!("  %chk.low# = fcmp oge double {operand}, -9.223372036854775808e18\n  %chk.high# = fcmp olt double {operand}, 9.223372036854775808e18\n  %chk.ok# = and i1 %chk.low#, %chk.high#\n");
                self.emit_fast_path(&register, "i64", &condition, &format!("fptosi double {operand} to i64"), &format!("call i64 @qiq_double_to_int(double {operand})"), body);
                result(ValueType::Int, register)
            }
            ("to_int", ValueType::String) => result(
                ValueType::Int,
                instruction(self, format!("call i64 @qiq_string_to_int(ptr {operand})"), true, body),
            ),
            ("to_double", ValueType::Int) => result(ValueType::Double, instruction(self, format!("sitofp i64 {operand} to double"), false, body)),
            ("to_double", ValueType::Byte) => result(ValueType::Double, instruction(self, format!("uitofp i8 {operand} to double"), false, body)),
            ("to_double", ValueType::String) => result(
                ValueType::Double,
                instruction(self, format!("call double @qiq_string_to_double(ptr {operand})"), true, body),
            ),
            ("to_byte", ValueType::Int) => {
                // Inline for 0 to 255; the runtime throws for the rest.
                let register = self.new_register();
                let condition = format!("  %chk.ok# = icmp ult i64 {operand}, 256\n");
                self.emit_fast_path(&register, "i8", &condition, &format!("trunc i64 {operand} to i8"), &format!("call zeroext i8 @qiq_int_to_byte(i64 {operand})"), body);
                result(ValueType::Byte, register)
            }
            ("to_string", ValueType::Int) => result(
                ValueType::String,
                instruction(self, format!("call ptr @qiq_int_to_string(i64 {operand})"), false, body),
            ),
            ("to_string", ValueType::Byte) => {
                let widened = instruction(self, format!("zext i8 {operand} to i64"), false, body);
                result(
                    ValueType::String,
                    instruction(self, format!("call ptr @qiq_int_to_string(i64 {widened})"), false, body),
                )
            }
            ("to_string", ValueType::Double) => result(
                ValueType::String,
                instruction(self, format!("call ptr @qiq_double_to_string(double {operand})"), false, body),
            ),
            ("to_string", ValueType::Bool) => {
                let yes = self.emit_expression(&Expr::Text("true".to_string()), variables, body)?;
                let no = self.emit_expression(&Expr::Text("false".to_string()), variables, body)?;
                result(
                    ValueType::String,
                    instruction(self, format!("select i1 {operand}, ptr {}, ptr {}", yes.operand, no.operand), false, body),
                )
            }
            _ => {
                return Err(format!("{name} cannot convert {}", self.type_name(from)));
            }
        };
        // to_string of an int, byte, or double makes a new string.
        if name == "to_string" && matches!(from, ValueType::Int | ValueType::Byte | ValueType::Double) {
            return Ok(self.own(converted));
        }
        Ok(converted)
    }

    /// `sprintf(format, values...)`: formatted by the runtime (qiq_sprintf in
    /// strings.c), which receives the values as 64-bit payloads with a type
    /// character each. A literal format is checked here.
    fn emit_sprintf(
        &mut self,
        arguments: &[Expr],
        variables: &HashMap<String, Value>,
        body: &mut String,
    ) -> Result<Value, String> {
        if arguments.iter().any(|argument| matches!(argument, Expr::NamedArgument(..))) {
            return Err("sprintf takes no named arguments".to_string());
        }
        let Some((format_expression, values)) = arguments.split_first() else {
            return Err("sprintf expects a format string".to_string());
        };
        if let Expr::Text(format) = format_expression {
            let required = sprintf_arguments_required(format).map_err(|error| format!("sprintf: {error}"))?;
            if required > values.len() {
                return Err(format!(
                    "sprintf: the format uses {required} value{}, but {} {} given",
                    if required == 1 { "" } else { "s" },
                    values.len(),
                    if values.len() == 1 { "is" } else { "are" }
                ));
            }
        }
        let format = self.emit_expression(format_expression, variables, body)?;
        if format.value_type != ValueType::String {
            return Err(format!("sprintf format must be a string, found {}", self.type_name(format.value_type)));
        }
        let mut types = String::new();
        let mut payloads = Vec::new();
        for (index, expression) in values.iter().enumerate() {
            let value = self.emit_expression(expression, variables, body)?;
            let value = self.emit_to_string_call(&value, variables, body)?.unwrap_or(value);
            let code = match value.value_type {
                ValueType::Int => 'i',
                ValueType::Double => 'd',
                ValueType::Bool => 'b',
                ValueType::Byte => 'y',
                ValueType::String => 's',
                ValueType::Char => 'c',
                other => {
                    return Err(format!(
                        "sprintf value {} must be an int, double, bool, byte, string, or char, found {}",
                        index + 1,
                        self.type_name(other)
                    ))
                }
            };
            types.push(code);
            payloads.push(self.encode_payload(&value, body)?);
        }
        let slots = payloads.len().max(1);
        let array = self.new_register();
        self.entry_allocas.push_str(&format!("  {array} = alloca [{slots} x i64], align 8\n"));
        for (index, payload) in payloads.iter().enumerate() {
            let address = self.new_register();
            body.push_str(&format!(
                "  {address} = getelementptr inbounds [{slots} x i64], ptr {array}, i64 0, i64 {index}\n  store i64 {payload}, ptr {address}, align 8\n"
            ));
        }
        let descriptor = self.string_literal(types.as_bytes());
        let result = self.new_register();
        body.push_str(&format!(
            "  {result} = call ptr @qiq_sprintf(ptr {}, ptr {descriptor}, ptr {array}, i64 {})\n",
            format.operand,
            payloads.len()
        ));
        self.emit_exception_check(body);
        Ok(self.own(Value {
            value_type: ValueType::String,
            operand: result,
            slot: None,
        }))
    }

    /// The name of the class whose id is in `class_id`, from a table of the
    /// classes' names as string literals (emitted when used).
    fn emit_class_name_of(&mut self, class_id: &str, body: &mut String) -> Value {
        self.uses_class_name_table = true;
        let slot = self.new_register();
        let name = self.new_register();
        body.push_str(&format!(
            "  {slot} = getelementptr inbounds ptr, ptr @qiq_class_name_strings, i64 {class_id}\n  {name} = load ptr, ptr {slot}, align 8\n"
        ));
        Value { value_type: ValueType::String, operand: name, slot: None }
    }

    /// Notes which network databases a `Database::open(url)` can open: those
    /// a literal URL names, or all of them for one computed at run time.
    fn note_database_open(&mut self, target: StaticTarget, method_name: &str, arguments: &[Expr]) {
        let StaticTarget::Class { class_id, .. } = target else { return };
        if method_name != "open" || self.stdlib_class("Database") != Some(class_id) {
            return;
        }
        let url = match arguments.first() {
            Some(Expr::NamedArgument(_, value)) => value.as_ref(),
            Some(value) => value,
            None => return,
        };
        match url {
            Expr::Text(text) if text.starts_with("sqlite:") => {}
            Expr::Text(text) if text.starts_with("postgres://") || text.starts_with("postgresql://") => self.uses_postgres = true,
            Expr::Text(text) if text.starts_with("mysql://") || text.starts_with("mariadb://") => self.uses_mysql = true,
            Expr::Text(_) => {}
            _ => {
                self.uses_postgres = true;
                self.uses_mysql = true;
            }
        }
    }

    /// `db->queryAs(sql, ...)` and `db->queryOneAs(sql, ...)`, and a cursor's
    /// `fetchAs(count)` and `nextAs()`: query, queryOne, fetch, or next with
    /// the same arguments, then each row read into an object of the class
    /// the result is used as (`User[] users = ...`), by a function made for
    /// the class (see row_reader).
    fn emit_query_as(
        &mut self,
        object: Value,
        method_name: &str,
        arguments: &[Expr],
        expected: Option<ValueType>,
        variables: &HashMap<String, Value>,
        body: &mut String,
    ) -> Result<Value, String> {
        // The method that gives the Row or Row[], and how it's used.
        let (plain, many, example) = match method_name {
            "queryAs" => ("query", true, "User[] users = db->queryAs(sql, ...);"),
            "queryOneAs" => ("queryOne", false, "User user = db->queryOneAs(sql, ...);"),
            "fetchAs" => ("fetch", true, "User[] users = cursor->fetchAs(100);"),
            _ => ("next", false, "User user = cursor->nextAs();"),
        };
        let class_id = match (many, expected) {
            (true, Some(value_type)) => match self.element_of(value_type) {
                Some(ValueType::Class(class_id)) => Some(class_id),
                _ => None,
            },
            (false, Some(ValueType::Class(class_id))) => Some(class_id),
            _ => None,
        };
        let Some(class_id) = class_id else {
            return Err(match expected {
                None | Some(ValueType::Void | ValueType::Null) => format!(
                    "{method_name} needs to know the class to read rows into: use it where the type is known, such as `{example}`"
                ),
                Some(value_type) => format!(
                    "{method_name} reads rows into {}, not {}",
                    if many { "an array of objects, such as User[]" } else { "an object" },
                    self.type_name(value_type)
                ),
            });
        };
        let class = &self.classes[class_id as usize];
        if class.is_interface || class.is_abstract {
            return Err(format!(
                "{method_name} can't create {}, which is {}; read rows into a concrete class",
                class.name,
                if class.is_interface { "an interface" } else { "abstract" }
            ));
        }
        let symbol = self.row_reader(class_id, many, method_name)?;
        let mut scope = variables.clone();
        scope.insert("#query.database".to_string(), Value { slot: None, ..object });
        let query = Expr::MethodCall(Box::new(Expr::Variable("#query.database".to_string())), plain.to_string(), arguments.to_vec());
        let rows = self.emit_expression(&query, &scope, body)?;
        let result = self.new_register();
        body.push_str(&format!("  {result} = call ptr @{symbol}(ptr {})\n", rows.operand));
        self.emit_exception_check(body);
        let value_type = if many { self.array_of(ValueType::Class(class_id)) } else { ValueType::Class(class_id) };
        Ok(self.own(Value { value_type, operand: result, slot: None }))
    }

    /// The function that reads a Row (or, for queryAs, each Row of an
    /// array) into objects of `class_id`, made the first time it's needed.
    /// With a constructor, it passes each parameter the column of the same
    /// name, read with the getter for its type; a parameter with a default
    /// gets the default when the column is missing or NULL. Without one, it
    /// sets each public property the same way, after `new Class()`.
    fn row_reader(&mut self, class_id: u32, many: bool, method_name: &str) -> Result<String, String> {
        if let Some((_, _, symbol)) = self.row_readers.iter().find(|(class, all, _)| *class == class_id && *all == many) {
            return Ok(symbol.clone());
        }
        let row_class = self.stdlib_class("Row").ok_or("internal error: std/sql's Row class isn't loaded")?;
        let class_name = self.classes[class_id as usize].name.clone();
        let row = || Box::new(Expr::Variable("#row".to_string()));
        let text = |name: &str| Expr::Text(name.to_string());
        let statement = |kind: StatementKind| Statement { kind, line: 0, column: 0 };
        let has = |name: &str| Expr::MethodCall(row(), "has".to_string(), vec![text(name)]);
        // How to read a column of this type, or why it can't be.
        let getter = |this: &Self, value_type: ValueType, what: &str| -> Result<(&'static str, Option<&'static str>), String> {
            Ok(match value_type {
                ValueType::Int => ("getInt", Some("getIntOr")),
                ValueType::Double => ("getDouble", Some("getDoubleOr")),
                ValueType::Bool => ("getBool", Some("getBoolOr")),
                ValueType::String => ("getString", None),
                ValueType::Class(class_id) if this.stdlib_class("DateTime") == Some(class_id) => ("getDateTime", None),
                other if this.element_of(other) == Some(ValueType::Byte) => ("getBytes", None),
                other => {
                    return Err(format!(
                        "{method_name} can't read {what} of {class_name}, a {}, from a column: columns give an int, double, bool, string, byte[], or DateTime",
                        this.type_name(other)
                    ))
                }
            })
        };
        // The value for a column, with `default` when it's missing or NULL.
        let read = |name: &str, default: Option<&Expr>, getter: (&str, Option<&str>), into: Expr| -> Vec<Statement> {
            let (plain, or) = getter;
            match default {
                None => vec![],
                Some(default) => {
                    let value = match or {
                        Some(or) => Expr::MethodCall(row(), or.to_string(), vec![text(name), default.clone()]),
                        None => Expr::Coalesce(
                            Box::new(Expr::MethodCall(row(), plain.to_string(), vec![text(name)])),
                            Box::new(default.clone()),
                        ),
                    };
                    let assign = match into {
                        Expr::Variable(variable) => StatementKind::Assign(variable, value),
                        Expr::Member(object, property) => StatementKind::AssignMember(*object, property, value),
                        _ => unreachable!("only variables and properties are filled in"),
                    };
                    vec![statement(StatementKind::If { condition: has(name), then_body: vec![statement(assign)], else_body: vec![] })]
                }
            }
        };
        let object = ValueType::Class(class_id);
        let mut steps = Vec::new();
        if let Ok((parameters, ..)) = self.find_method(class_id, "__construct") {
            let mut arguments = Vec::new();
            for (index, parameter) in parameters.iter().enumerate() {
                let how = getter(self, parameter.value_type, &format!("the constructor's parameter '{}'", parameter.name))?;
                match &parameter.default {
                    None => arguments.push(Expr::MethodCall(row(), how.0.to_string(), vec![text(&parameter.name)])),
                    Some(default) => {
                        let local = format!("#argument{index}");
                        steps.push(statement(StatementKind::Declare(parameter.value_type, local.clone(), default.clone())));
                        steps.extend(read(&parameter.name, Some(default), how, Expr::Variable(local.clone())));
                        arguments.push(Expr::Variable(local));
                    }
                }
            }
            steps.push(statement(StatementKind::Declare(object, "#object".to_string(), Expr::NewObject(class_id, arguments))));
        } else {
            steps.push(statement(StatementKind::Declare(object, "#object".to_string(), Expr::NewObject(class_id, Vec::new()))));
            for (property, _) in self.class_fields(class_id) {
                if property.visibility != Visibility::Public {
                    continue;
                }
                let how = getter(self, property.value_type, &format!("the property '{}'", property.name))?;
                let target = Expr::Member(Box::new(Expr::Variable("#object".to_string())), property.name.clone());
                match &property.default {
                    None => steps.push(statement(StatementKind::AssignMember(
                        Expr::Variable("#object".to_string()),
                        property.name.clone(),
                        Expr::MethodCall(row(), how.0.to_string(), vec![text(&property.name)]),
                    ))),
                    Some(default) => steps.extend(read(&property.name, Some(default), how, target)),
                }
            }
        }
        let row_type = ValueType::Class(row_class);
        let (parameter, return_type, statements) = if many {
            let rows_type = self.array_of(row_type);
            let result_type = self.array_of(object);
            steps.push(statement(StatementKind::AssignIndex(
                Expr::Variable("#result".to_string()),
                Expr::Variable("#index".to_string()),
                Expr::Variable("#object".to_string()),
            )));
            let statements = vec![
                statement(StatementKind::Declare(
                    result_type,
                    "#result".to_string(),
                    Expr::NewArray(object, Box::new(Expr::Call("len".to_string(), vec![Expr::Variable("#rows".to_string())]))),
                )),
                statement(StatementKind::Foreach {
                    iterable: Expr::Variable("#rows".to_string()),
                    key_name: Some("#index".to_string()),
                    value_name: "#row".to_string(),
                    body: steps,
                }),
                statement(StatementKind::Return(Some(Expr::Variable("#result".to_string())))),
            ];
            (Parameter { value_type: rows_type, name: "#rows".to_string(), default: None }, result_type, statements)
        } else {
            let mut statements = vec![statement(StatementKind::If {
                condition: Expr::Compare(CompareOp::Equal, row(), Box::new(Expr::Null)),
                then_body: vec![statement(StatementKind::Return(Some(Expr::Null)))],
                else_body: vec![],
            })];
            statements.extend(steps);
            statements.push(statement(StatementKind::Return(Some(Expr::Variable("#object".to_string())))));
            (Parameter { value_type: row_type, name: "#row".to_string(), default: None }, object, statements)
        };
        let symbol = format!(
            "qiq.{}.{}",
            if many { "rows_to" } else { "row_to" },
            self.classes[class_id as usize].symbol
        );
        let function = Function {
            name: "#reader".to_string(),
            parameters: vec![parameter],
            return_type,
            body: statements,
            visibility: Visibility::Public,
            is_abstract: false,
            is_final: false,
            is_static: false,
            returns_static: false,
            module: self.current_module as u32,
            symbol: symbol.clone(),
            is_extern: false,
            binds_values: false,
            line: 0,
            column: 0,
        };
        let saved = self.save_function_state();
        let emitted = self.emit_function(&function, None);
        self.restore_function_state(saved);
        let ir = emitted.map_err(|error| {
            let message = error.trim_start_matches("0:0: ").trim_start_matches("function '#reader': ");
            format!("{method_name} into {class_name}: {message}")
        })?;
        self.closure_functions.push_str(&ir);
        self.closure_functions.push('\n');
        self.row_readers.push((class_id, many, symbol.clone()));
        Ok(symbol)
    }

    /// The values after a `...` method's parameters (see
    /// Function::binds_values): evaluated in order, then handed to the
    /// runtime in one call, as sprintf's are, so a query in one of them can't
    /// replace them. Always made, even with no values, so the method never
    /// sees values left from a call that threw before it ran.
    fn emit_bind_values(
        &mut self,
        method_name: &str,
        values: &[Expr],
        variables: &HashMap<String, Value>,
        body: &mut String,
    ) -> Result<(), String> {
        let mut types = String::new();
        let mut payloads = Vec::new();
        for (index, expression) in values.iter().enumerate() {
            if matches!(expression, Expr::NamedArgument(..)) {
                return Err(format!("{method_name}: the values for the query's placeholders can't be named arguments"));
            }
            let mut value = self.emit_expression(expression, variables, body)?;
            // A DateTime goes as text (DateTime::__sqlText).
            let mut datetime = false;
            if let ValueType::Class(class_id) = value.value_type {
                if self.stdlib_class("DateTime") == Some(class_id) {
                    let mut scope = variables.clone();
                    scope.insert("#bind.datetime".to_string(), Value { slot: None, ..value.clone() });
                    let text = Expr::StaticCall(
                        StaticTarget::Class { class_id, forwards: false },
                        "__sqlText".to_string(),
                        vec![Expr::Variable("#bind.datetime".to_string())],
                    );
                    value = self.emit_expression(&text, &scope, body)?;
                    datetime = true;
                }
            }
            let code = match value.value_type {
                ValueType::String if datetime => 'T',
                ValueType::Int => 'i',
                ValueType::Double => 'd',
                ValueType::Bool => 'b',
                ValueType::Byte => 'y',
                ValueType::String | ValueType::Char => 's',
                ValueType::Null => 'n',
                other if self.element_of(other) == Some(ValueType::Byte) => 'a',
                other => {
                    return Err(format!(
                        "{method_name}: value {} for the query must be an int, double, bool, byte, string, char, byte[], DateTime, or null, found {}",
                        index + 1,
                        self.type_name(other)
                    ))
                }
            };
            types.push(code);
            payloads.push(if code == 'n' { "0".to_string() } else { self.encode_payload(&value, body)? });
        }
        let slots = payloads.len().max(1);
        let array = self.new_register();
        self.entry_allocas.push_str(&format!("  {array} = alloca [{slots} x i64], align 8\n"));
        for (index, payload) in payloads.iter().enumerate() {
            let address = self.new_register();
            body.push_str(&format!(
                "  {address} = getelementptr inbounds [{slots} x i64], ptr {array}, i64 0, i64 {index}\n  store i64 {payload}, ptr {address}, align 8\n"
            ));
        }
        let descriptor = self.string_literal(types.as_bytes());
        body.push_str(&format!(
            "  call void @qiq_bind_values(ptr {descriptor}, ptr {array}, i64 {})\n",
            payloads.len()
        ));
        Ok(())
    }

    /// `left ?? right`. The result goes through a stack slot holding one
    /// reference (for reference types), filled on whichever path runs.
    fn emit_coalesce(
        &mut self,
        left: &Expr,
        right: &Expr,
        variables: &HashMap<String, Value>,
        body: &mut String,
    ) -> Result<Value, String> {
        if matches!(left, Expr::Nullsafe(..)) {
            return self.emit_nullsafe(left, Some(right), variables, body);
        }
        let id = self.next_label;
        self.next_label += 1;
        let found_label = format!("coalesce.found{id}");
        let missing_label = format!("coalesce.missing{id}");
        let end_label = format!("coalesce.end{id}");
        // Emits the found block's start; returns the left value there.
        let left_value = match left {
            Expr::Index(container_expression, key_expression) => {
                let container = self.emit_expression(container_expression, variables, body)?;
                // Used by qiq_map_find (or the bounds check) straight away.
                let key = self.emit_borrowed(key_expression, None, variables, body)?;
                if let ValueType::Map(type_id) = container.value_type {
                    let signature = self.map_types[type_id as usize];
                    if key.value_type != signature.key_type {
                        return Err(format!(
                            "map key requires {}, found {}",
                            self.type_name(signature.key_type),
                            self.type_name(key.value_type)
                        ));
                    }
                    let key_bits = self.encode_payload(&key, body)?;
                    let out = self.new_register();
                    self.entry_allocas.push_str(&format!("  {out} = alloca i64, align 8\n"));
                    let found = self.new_register();
                    body.push_str(&format!(
                        "  {found} = call zeroext i1 @qiq_map_find(ptr {}, i64 {key_bits}, ptr {out})\n",
                        container.operand
                    ));
                    self.emit_exception_check(body);
                    body.push_str(&format!("  br i1 {found}, label %coalesce.present{id}, label %{missing_label}\ncoalesce.present{id}:\n"));
                    let bits = self.new_register();
                    body.push_str(&format!("  {bits} = load i64, ptr {out}, align 8\n"));
                    self.decode_payload(signature.value_type, &bits, body)?
                } else if let Some(element_type) = self.element_of(container.value_type) {
                    if key.value_type != ValueType::Int {
                        return Err("array index must be an int".to_string());
                    }
                    // In range: a non-null array and 0 <= index < length.
                    let not_null = self.new_register();
                    let length = self.new_register();
                    let in_range = self.new_register();
                    body.push_str(&format!(
                        "  {not_null} = icmp ne ptr {}, null\n  br i1 {not_null}, label %coalesce.bounds{id}, label %{missing_label}\ncoalesce.bounds{id}:\n  {length} = load i64, ptr {}, align 8\n  {in_range} = icmp ult i64 {}, {length}\n  br i1 {in_range}, label %coalesce.present{id}, label %{missing_label}\ncoalesce.present{id}:\n",
                        container.operand, container.operand, key.operand
                    ));
                    let data = self.new_register();
                    let address = self.new_register();
                    let loaded = self.new_register();
                    body.push_str(&format!(
                        "  {data}.at = getelementptr inbounds i8, ptr {}, i64 {ARRAY_DATA_OFFSET}\n  {data} = load ptr, ptr {data}.at, align 8\n  {address} = getelementptr inbounds {}, ptr {data}, i64 {}\n  {loaded} = load {}, ptr {address}, align {}\n",
                        container.operand,
                        element_type.llvm(),
                        key.operand,
                        element_type.llvm(),
                        element_type.alignment()
                    ));
                    Value { value_type: element_type, operand: loaded, slot: None }
                } else {
                    return Err("indexed value is not an array or map".to_string());
                }
            }
            _ => {
                let value = self.emit_expression(left, variables, body)?;
                if !is_reference(value.value_type) {
                    return Err(format!(
                        "'??' needs a map or array element, or a value that can be null (a string, char, array, map, or object); found {}",
                        self.type_name(value.value_type)
                    ));
                }
                value
            }
        };
        let value_type = left_value.value_type;
        // A null element or value counts as missing, as in PHP.
        if is_reference(value_type) {
            let not_null = self.new_register();
            body.push_str(&format!(
                "  {not_null} = icmp ne ptr {}, null\n  br i1 {not_null}, label %{found_label}, label %{missing_label}\n",
                left_value.operand
            ));
        } else {
            body.push_str(&format!("  br label %{found_label}\n"));
        }
        let slot = self.new_register();
        self.entry_allocas.push_str(&format!("  {slot} = alloca {}, align 8\n", value_type.llvm()));
        body.push_str(&format!("{found_label}:\n"));
        // An element read from a map or array is retained; a temporary is
        // handed over; a variable is retained.
        let operand = if is_reference(value_type) {
            if matches!(left, Expr::Index(..)) {
                body.push_str(&format!("  call void @qiq_retain(ptr {})\n", left_value.operand));
                left_value.operand.clone()
            } else {
                self.take(&left_value, body)
            }
        } else {
            left_value.operand.clone()
        };
        body.push_str(&format!(
            "  store {} {operand}, ptr {slot}, align 8\n  br label %{end_label}\n{missing_label}:\n",
            value_type.llvm()
        ));
        self.open_scope();
        let fallback = self.emit_expecting(right, Some(value_type), variables, body)?;
        if !self.is_assignable(value_type, fallback.value_type) {
            return Err(format!(
                "the right side of '??' must be {}, found {}",
                self.type_name(value_type),
                self.type_name(fallback.value_type)
            ));
        }
        let fallback_operand = self.take(&fallback, body);
        self.close_scope(body);
        let result = self.new_register();
        body.push_str(&format!(
            "  store {} {fallback_operand}, ptr {slot}, align 8\n  br label %{end_label}\n{end_label}:\n  {result} = load {}, ptr {slot}, align 8\n",
            value_type.llvm(),
            value_type.llvm()
        ));
        Ok(self.own(Value { value_type, operand: result, slot: None }))
    }

    /// `range(start, end, step)`'s arguments: their element type (int,
    /// double, or char) and their values, ints converted to doubles when
    /// any of them is a double. `range(count)` has one value, an int: it
    /// counts from 0 up to count - 1, and gives nothing when count <= 0.
    fn emit_range_arguments(
        &mut self,
        arguments: &[Expr],
        variables: &HashMap<String, Value>,
        body: &mut String,
    ) -> Result<(ValueType, Vec<Value>), String> {
        if arguments.iter().any(|argument| matches!(argument, Expr::NamedArgument(..))) {
            return Err("range takes no named arguments".to_string());
        }
        if !(1..=3).contains(&arguments.len()) {
            return Err(format!(
                "range expects a count, or start, end, and an optional step; found {} arguments",
                arguments.len()
            ));
        }
        let mut values = Vec::new();
        for argument in arguments {
            values.push(self.emit_expression(argument, variables, body)?);
        }
        if let [count] = values.as_slice() {
            if count.value_type != ValueType::Int {
                return Err(format!("range(count) counts from 0 up to count - 1 and needs an int, found {}", self.type_name(count.value_type)));
            }
            return Ok((ValueType::Int, values));
        }
        let types: Vec<ValueType> = values.iter().map(|value| value.value_type).collect();
        if types[0] == ValueType::Char || types[1] == ValueType::Char {
            if types[0] != ValueType::Char || types[1] != ValueType::Char {
                return Err(format!(
                    "range of chars needs two chars, found {} and {}",
                    self.type_name(types[0]),
                    self.type_name(types[1])
                ));
            }
            if let Some(step) = types.get(2).filter(|step| **step != ValueType::Int) {
                return Err(format!("the step of a range of chars must be an int, found {}", self.type_name(*step)));
            }
            return Ok((ValueType::Char, values));
        }
        for value_type in &types {
            if !matches!(value_type, ValueType::Int | ValueType::Double) {
                return Err(format!("range expects ints, doubles, or two chars, found {}", self.type_name(*value_type)));
            }
        }
        if !types.contains(&ValueType::Double) {
            return Ok((ValueType::Int, values));
        }
        for value in &mut values {
            if value.value_type == ValueType::Int {
                let converted = self.new_register();
                body.push_str(&format!("  {converted} = sitofp i64 {} to double\n", value.operand));
                *value = Value { value_type: ValueType::Double, operand: converted, slot: None };
            }
        }
        Ok((ValueType::Double, values))
    }

    /// `range(...)` as an array: `int[]`, `double[]`, or `char[]`.
    fn emit_range_array(&mut self, kind: ValueType, values: &[Value], body: &mut String) -> Result<Value, String> {
        if let [count] = values {
            let array = self.new_register();
            body.push_str(&format!("  {array} = call ptr @qiq_range_count(i64 {})\n", count.operand));
            self.emit_exception_check(body);
            let array_type = self.array_of(ValueType::Int);
            return Ok(self.own(Value { value_type: array_type, operand: array, slot: None }));
        }
        let (step, has_step) = match values.get(2) {
            Some(step) => (step.operand.clone(), "true"),
            None if kind == ValueType::Double => ("1.0".to_string(), "false"),
            None => ("1".to_string(), "false"),
        };
        let (function, llvm) = match kind {
            ValueType::Int => ("qiq_range_int", "i64"),
            ValueType::Double => ("qiq_range_double", "double"),
            _ => ("qiq_range_char", "ptr"),
        };
        let step_llvm = if kind == ValueType::Double { "double" } else { "i64" };
        let array = self.new_register();
        body.push_str(&format!(
            "  {array} = call ptr @{function}({llvm} {}, {llvm} {}, {step_llvm} {step}, i1 zeroext {has_step})\n",
            values[0].operand, values[1].operand
        ));
        self.emit_exception_check(body);
        let array_type = self.array_of(kind);
        Ok(self.own(Value { value_type: array_type, operand: array, slot: None }))
    }

    /// The functions that grow and shrink arrays: `array_push(array, values...)`,
    /// `array_pop(array)`, `array_shift(array)`, `array_unshift(array,
    /// values...)`, and `array_splice(array, offset, length, replacement)`.
    /// Built in because they work with any element type.
    fn emit_array_function(
        &mut self,
        name: &str,
        arguments: &[Expr],
        variables: &HashMap<String, Value>,
        body: &mut String,
    ) -> Result<Value, String> {
        if arguments.iter().any(|argument| matches!(argument, Expr::NamedArgument(..))) {
            return Err(format!("{name} takes no named arguments"));
        }
        let (minimum, maximum) = match name {
            "array_push" | "array_unshift" => (2, usize::MAX),
            "array_splice" => (2, 4),
            _ => (1, 1),
        };
        if arguments.len() < minimum || arguments.len() > maximum {
            let expected = match name {
                "array_push" | "array_unshift" => "an array and at least one value".to_string(),
                "array_splice" => "2 to 4 arguments".to_string(),
                _ => "1 argument".to_string(),
            };
            return Err(format!("{name} expects {expected}, found {} argument{}", arguments.len(), if arguments.len() == 1 { "" } else { "s" }));
        }
        let array = self.emit_expression(&arguments[0], variables, body)?;
        let Some(element_type) = self.element_of(array.value_type) else {
            return Err(format!("{name} expects an array, found {}", self.type_name(array.value_type)));
        };
        let size = element_type.size_bytes();
        let nothing = Value { value_type: ValueType::Null, operand: "null".to_string(), slot: None };
        match name {
            "array_push" | "array_unshift" => {
                for (position, expression) in arguments[1..].iter().enumerate() {
                    let value = self.emit_expecting(expression, Some(element_type), variables, body)?;
                    if !self.is_assignable(element_type, value.value_type) {
                        return Err(format!(
                            "{name} adds {} values to this array, found {}",
                            self.type_name(element_type),
                            self.type_name(value.value_type)
                        ));
                    }
                    // The array takes a reference to each value.
                    self.take(&value, body);
                    let bits = if value.operand == "null" { "0".to_string() } else { self.encode_payload(&value, body)? };
                    if name == "array_push" {
                        body.push_str(&format!("  call void @qiq_array_push(ptr {}, i64 {size}, i64 {bits})\n", array.operand));
                    } else {
                        // Several values keep their order at the front, as in PHP.
                        body.push_str(&format!(
                            "  call void @qiq_array_insert(ptr {}, i64 {size}, i64 {position}, i64 {bits})\n",
                            array.operand
                        ));
                    }
                    self.emit_exception_check(body);
                }
                Ok(nothing)
            }
            "array_pop" | "array_shift" => {
                let bits = self.new_register();
                body.push_str(&format!("  {bits} = call i64 @qiq_{name}(ptr {}, i64 {size})\n", array.operand));
                self.emit_exception_check(body);
                // The caller gets the array's reference to the element.
                let value = self.decode_payload(element_type, &bits, body)?;
                Ok(self.own(value))
            }
            _ => {
                let offset = self.emit_expression(&arguments[1], variables, body)?;
                if offset.value_type != ValueType::Int {
                    return Err(format!("array_splice offset must be an int, found {}", self.type_name(offset.value_type)));
                }
                let (length, has_length) = match arguments.get(2) {
                    Some(expression) => {
                        let length = self.emit_expression(expression, variables, body)?;
                        if length.value_type != ValueType::Int {
                            return Err(format!("array_splice length must be an int, found {}", self.type_name(length.value_type)));
                        }
                        (length.operand, "true")
                    }
                    None => ("0".to_string(), "false"),
                };
                let replacement = match arguments.get(3) {
                    Some(expression) => {
                        let replacement = self.emit_expecting(expression, Some(array.value_type), variables, body)?;
                        if replacement.value_type != array.value_type {
                            return Err(format!(
                                "array_splice replacement must be {}, found {}",
                                self.type_name(array.value_type),
                                self.type_name(replacement.value_type)
                            ));
                        }
                        replacement.operand
                    }
                    None => "null".to_string(),
                };
                let removed = self.new_register();
                body.push_str(&format!(
                    "  {removed} = call ptr @qiq_array_splice(ptr {}, i64 {size}, i64 {}, i64 {length}, i1 zeroext {has_length}, ptr {replacement})\n",
                    array.operand, offset.operand
                ));
                self.emit_exception_check(body);
                Ok(self.own(Value { value_type: array.value_type, operand: removed, slot: None }))
            }
        }
    }

    fn save_function_state(&mut self) -> FunctionState {
        FunctionState {
            next_register: self.next_register,
            next_label: self.next_label,
            entry_allocas: std::mem::take(&mut self.entry_allocas),
            temporaries: std::mem::take(&mut self.temporaries),
            owned_slots: std::mem::take(&mut self.owned_slots),
            handlers: std::mem::take(&mut self.handlers),
            handler_scopes: std::mem::take(&mut self.handler_scopes),
            finally_stack: std::mem::take(&mut self.finally_stack),
            loop_targets: std::mem::take(&mut self.loop_targets),
            current_class: self.current_class,
            current_method: self.current_method.clone(),
            current_method_is_static: self.current_method_is_static,
            boxed: std::mem::take(&mut self.boxed),
            in_closure: self.in_closure,
            closure_has_this: self.closure_has_this,
        }
    }

    fn restore_function_state(&mut self, state: FunctionState) {
        self.next_register = state.next_register;
        self.next_label = state.next_label;
        self.entry_allocas = state.entry_allocas;
        self.temporaries = state.temporaries;
        self.owned_slots = state.owned_slots;
        self.handlers = state.handlers;
        self.handler_scopes = state.handler_scopes;
        self.finally_stack = state.finally_stack;
        self.loop_targets = state.loop_targets;
        self.current_class = state.current_class;
        self.current_method = state.current_method;
        self.current_method_is_static = state.current_method_is_static;
        self.boxed = state.boxed;
        self.in_closure = state.in_closure;
        self.closure_has_this = state.closure_has_this;
    }

    /// An anonymous function: emits its body as a function of its own
    /// (`@qiq.closure.N`, taking the closure object first) and, here, code
    /// that creates the closure object: a pointer to that function and the
    /// cells (and `this`) it captures.
    fn emit_closure(
        &mut self,
        closure: &Closure,
        expected: Option<ValueType>,
        variables: &HashMap<String, Value>,
        body: &mut String,
    ) -> Result<Value, String> {
        let parameter_types: Vec<ValueType> = closure.parameters.iter().map(|parameter| parameter.value_type).collect();
        // Captures: the variables in scope that the body uses.
        let mut captures: Vec<(Capture, Value)> = Vec::new();
        for name in closure_free_names(closure) {
            let Some(variable) = variables.get(&name) else {
                continue;
            };
            let read_only = match variable.slot.as_deref() {
                _ if name == "this" => true,
                Some(slot) if slot.starts_with("cell") => slot.starts_with("cellro") || slot.starts_with("cellptrro"),
                _ => return Err(format!("internal error: captured variable '{name}' isn't in a cell")),
            };
            captures.push((Capture { name, value_type: variable.value_type, read_only }, variable.clone()));
        }
        let return_type = match closure.return_type {
            Some(return_type) => return_type,
            None => match expected {
                Some(ValueType::Function(type_id)) if self.function_types[type_id as usize].parameters == parameter_types => {
                    self.function_types[type_id as usize].return_type
                }
                _ => self.infer_arrow_type(closure, &captures)?,
            },
        };
        // An arrow function whose expression is a void call returns nothing.
        let statements = match (&closure.return_type, &closure.body[..]) {
            (None, [statement @ Statement { kind: StatementKind::Return(Some(expression)), .. }])
                if return_type == ValueType::Void =>
            {
                vec![Statement::like(statement, StatementKind::Expression(expression.clone()))]
            }
            _ => closure.body.clone(),
        };
        let type_id = match function_type_of(
            &mut self.function_types,
            FunctionType { parameters: parameter_types, return_type },
        ) {
            ValueType::Function(type_id) => type_id,
            _ => unreachable!("function_type_of returns a function type"),
        };

        // The closure object's class: the function pointer, then the captures.
        let mut properties = vec![Property { name: "function".to_string(), value_type: ValueType::Int, visibility: Visibility::Private, default: None, default_at: (0, 0), readonly: false }];
        for (capture, _) in &captures {
            let field_type = if capture.name == "this" {
                capture.value_type
            } else {
                ValueType::Class(self.cell_class(capture.value_type))
            };
            properties.push(Property { name: capture.name.clone(), value_type: field_type, visibility: Visibility::Private, default: None, default_at: (0, 0), readonly: false });
        }
        let field_count = OBJECT_HEADER_FIELDS + properties.len();
        let class_id = self.add_synthetic_class("Closure", properties);

        // The body, emitted now as a separate function.
        let id = self.next_closure;
        self.next_closure += 1;
        let symbol = format!("qiq.closure.{id}");
        let function = Function {
            name: "{closure}".to_string(),
            parameters: closure.parameters.clone(),
            return_type,
            body: statements,
            visibility: Visibility::Public,
            is_abstract: false,
            is_final: false,
            is_static: false,
            returns_static: false,
            module: self.current_module as u32,
            symbol: symbol.clone(),
            is_extern: false,
            binds_values: false,
            line: closure.line,
            column: closure.column,
        };
        let context = ClosureContext {
            captures: captures.iter().map(|(capture, _)| capture.clone()).collect(),
            class: self.current_class,
            method: self.current_method.clone(),
        };
        let saved = self.save_function_state();
        self.closure_context = Some(context);
        let emitted = self.emit_function(&function, None);
        self.restore_function_state(saved);
        let ir = emitted.map_err(|error| error.replacen("function '{closure}': ", "anonymous function: ", 1))?;
        self.closure_functions.push_str(&ir);
        self.closure_functions.push('\n');

        // The closure object.
        let object = self.new_register();
        let at = self.new_register();
        body.push_str(&format!(
            "  {object} = call nonnull ptr @qiq_object_new(i64 {field_count}, i64 {class_id})\n  {at} = getelementptr inbounds i64, ptr {object}, i64 {OBJECT_HEADER_FIELDS}\n  store ptr @{symbol}, ptr {at}, align 8\n"
        ));
        for (index, (capture, variable)) in captures.iter().enumerate() {
            let pointer = if capture.name == "this" {
                variable.operand.clone()
            } else {
                let slot = variable.slot.clone().expect("checked above");
                self.cell_of(&slot, body).expect("checked above").0
            };
            let field = self.new_register();
            body.push_str(&format!(
                "  call void @qiq_retain(ptr {pointer})\n  {field} = getelementptr inbounds i64, ptr {object}, i64 {}\n  store ptr {pointer}, ptr {field}, align 8\n",
                OBJECT_HEADER_FIELDS + 1 + index
            ));
        }
        Ok(self.own(Value { value_type: ValueType::Function(type_id), operand: object, slot: None }))
    }

    /// The type of an arrow function's expression, found by emitting it once
    /// in a scratch context (the code is discarded).
    fn infer_arrow_type(&mut self, closure: &Closure, captures: &[(Capture, Value)]) -> Result<ValueType, String> {
        let Some(StatementKind::Return(Some(expression))) = closure.body.first().map(|statement| &statement.kind) else {
            return Err("an anonymous function needs a return type".to_string());
        };
        let saved = self.save_function_state();
        self.temporaries = vec![Vec::new()];
        self.handlers = vec!["unwind".to_string()];
        self.handler_scopes = vec![0];
        self.boxed = captured_names(&closure.body);
        self.in_closure = true;
        self.closure_has_this = captures.iter().any(|(capture, _)| capture.name == "this");
        self.current_method_is_static = false;
        let mut variables = HashMap::new();
        for (index, (capture, _)) in captures.iter().enumerate() {
            let value = if capture.name == "this" {
                Value { value_type: capture.value_type, operand: "%this".to_string(), slot: None }
            } else {
                Value { value_type: capture.value_type, operand: String::new(), slot: Some(format!("cellptr:%capture{index}")) }
            };
            variables.insert(capture.name.clone(), value);
        }
        for (index, parameter) in closure.parameters.iter().enumerate() {
            let slot = self.boxed.contains(&parameter.name).then(|| format!("cellptr:%arg{index}"));
            variables.insert(
                parameter.name.clone(),
                Value { value_type: parameter.value_type, operand: format!("%arg{index}"), slot },
            );
        }
        let emitted = self.closure_functions.len();
        let mut scratch = String::new();
        let result = self.emit_expression(expression, &variables, &mut scratch);
        self.closure_functions.truncate(emitted);
        self.restore_function_state(saved);
        let value_type = result?.value_type;
        Ok(if value_type == ValueType::Null { ValueType::Void } else { value_type })
    }

    /// Calls a function value: through the function pointer in the closure
    /// object, which it passes first.
    fn emit_value_call(
        &mut self,
        callee: Value,
        arguments: &[Expr],
        variables: &HashMap<String, Value>,
        body: &mut String,
    ) -> Result<Value, String> {
        let ValueType::Function(type_id) = callee.value_type else {
            return Err(format!("{} is not a function, so it can't be called", self.type_name(callee.value_type)));
        };
        let signature = self.function_types[type_id as usize].clone();
        let description = self.type_name(callee.value_type);
        if arguments.iter().any(|argument| matches!(argument, Expr::NamedArgument(..))) {
            return Err("function values take no named arguments".to_string());
        }
        if arguments.len() != signature.parameters.len() {
            return Err(format!(
                "this {description} takes {} argument{}, found {}",
                signature.parameters.len(),
                if signature.parameters.len() == 1 { "" } else { "s" },
                arguments.len()
            ));
        }
        let mut operands = vec![format!("ptr {}", callee.operand)];
        for (index, (argument, parameter)) in arguments.iter().zip(&signature.parameters).enumerate() {
            let value = self.emit_expecting(argument, Some(*parameter), variables, body)?;
            if !self.is_assignable(*parameter, value.value_type) {
                return Err(format!(
                    "argument {} of this {description} must be {}, found {}",
                    index + 1,
                    self.type_name(*parameter),
                    self.type_name(value.value_type)
                ));
            }
            operands.push(format!("{} {}", parameter.llvm(), value.operand));
        }
        body.push_str(&format!("  call void @qiq_check_callable(ptr {})\n", callee.operand));
        self.emit_exception_check(body);
        let at = self.new_register();
        let function = self.new_register();
        body.push_str(&format!(
            "  {at} = getelementptr inbounds i64, ptr {}, i64 {OBJECT_HEADER_FIELDS}\n  {function} = load ptr, ptr {at}, align 8\n",
            callee.operand
        ));
        let operands = operands.join(", ");
        let result = if signature.return_type == ValueType::Void {
            body.push_str(&format!("  call void {function}({operands})\n"));
            "null".to_string()
        } else {
            let result = self.new_register();
            body.push_str(&format!("  {result} = call {} {function}({operands})\n", signature.return_type.llvm()));
            result
        };
        self.emit_exception_check(body);
        Ok(self.own(Value { value_type: Self::call_result_type(signature.return_type), operand: result, slot: None }))
    }

    /// A named function used as a value: a closure object whose function is
    /// an adapter that calls it.
    fn emit_function_reference(&mut self, name: &str, body: &mut String) -> Result<Value, String> {
        let symbol = self.module_functions[self.current_module][name].clone();
        if !self.called_functions.contains(&symbol) {
            self.called_functions.push(symbol.clone());
        }
        let signature = self.signatures.get(&symbol).ok_or_else(|| format!("function '{name}' is not declared"))?;
        let parameters: Vec<ValueType> = signature.parameters.iter().map(|parameter| parameter.value_type).collect();
        let return_type = signature.return_type;
        let value_type = function_type_of(&mut self.function_types, FunctionType { parameters: parameters.clone(), return_type });
        let class_id = match self.function_references.iter().find(|(existing, _)| *existing == symbol) {
            Some((_, class_id)) => *class_id,
            None => {
                let declared: Vec<String> =
                    parameters.iter().enumerate().map(|(index, parameter)| format!("{} %arg{index}", parameter.llvm())).collect();
                let passed = declared.clone();
                let call = if return_type == ValueType::Void {
                    format!("  call void @{symbol}({})\n  ret void\n", passed.join(", "))
                } else {
                    format!("  %result = call {0} @{symbol}({1})\n  ret {0} %result\n", return_type.llvm(), passed.join(", "))
                };
                let mut all = vec!["ptr %env".to_string()];
                all.extend(declared);
                self.closure_functions.push_str(&format!(
                    "define {} @qiq.reference.{symbol}({}) {{\nentry:\n{call}}}\n\n",
                    return_type.llvm(),
                    all.join(", ")
                ));
                let class_id = self.add_synthetic_class(
                    "Closure",
                    vec![Property { name: "function".to_string(), value_type: ValueType::Int, visibility: Visibility::Private, default: None, default_at: (0, 0), readonly: false }],
                );
                self.function_references.push((symbol.clone(), class_id));
                class_id
            }
        };
        let object = self.new_register();
        let at = self.new_register();
        body.push_str(&format!(
            "  {object} = call nonnull ptr @qiq_object_new(i64 {}, i64 {class_id})\n  {at} = getelementptr inbounds i64, ptr {object}, i64 {OBJECT_HEADER_FIELDS}\n  store ptr @qiq.reference.{symbol}, ptr {at}, align 8\n",
            OBJECT_HEADER_FIELDS + 1
        ));
        Ok(self.own(Value { value_type, operand: object, slot: None }))
    }

    /// The map functions: `map_key_exists(key, map)`, `map_keys(map)`,
    /// `map_values(map)`, `in_map(value, map)`, and `unset(map[key])`. Built
    /// in because they work with any key and value types.
    fn emit_map_function(
        &mut self,
        name: &str,
        arguments: &[Expr],
        variables: &HashMap<String, Value>,
        body: &mut String,
    ) -> Result<Value, String> {
        if arguments.iter().any(|argument| matches!(argument, Expr::NamedArgument(..))) {
            return Err(format!("{name} takes no named arguments"));
        }
        let expected_count = if matches!(name, "map_key_exists" | "in_map") { 2 } else { 1 };
        if arguments.len() != expected_count {
            return Err(format!(
                "{name} expects {expected_count} argument{}, found {}",
                if expected_count == 1 { "" } else { "s" },
                arguments.len()
            ));
        }
        let map_signature = |this: &Self, map: &Value| -> Result<MapType, String> {
            match map.value_type {
                ValueType::Map(type_id) => Ok(this.map_types[type_id as usize]),
                other => Err(format!("{name} expects a map, found {}", this.type_name(other))),
            }
        };
        let result = |value_type: ValueType, operand: String| Value { value_type, operand, slot: None };
        match name {
            "unset" => {
                let Expr::Index(map_expression, key_expression) = &arguments[0] else {
                    return Err("unset expects a map entry, such as unset(ages[\"ada\"])".to_string());
                };
                let map = self.emit_expression(map_expression, variables, body)?;
                if self.element_of(map.value_type).is_some() {
                    return Err("unset can't remove array elements: arrays have a fixed length".to_string());
                }
                let signature = map_signature(self, &map)?;
                let key = self.emit_expression(key_expression, variables, body)?;
                if key.value_type != signature.key_type {
                    return Err(format!(
                        "map key requires {}, found {}",
                        self.type_name(signature.key_type),
                        self.type_name(key.value_type)
                    ));
                }
                let key_bits = self.encode_payload(&key, body)?;
                body.push_str(&format!("  call void @qiq_map_remove(ptr {}, i64 {key_bits})\n", map.operand));
                self.emit_exception_check(body);
                Ok(result(ValueType::Null, "null".to_string()))
            }
            "map_keys" | "map_values" => {
                let map = self.emit_expression(&arguments[0], variables, body)?;
                let signature = map_signature(self, &map)?;
                let (element_type, function) = if name == "map_keys" {
                    (signature.key_type, "qiq_map_keys")
                } else {
                    (signature.value_type, "qiq_map_values")
                };
                let array = self.new_register();
                body.push_str(&format!(
                    "  {array} = call ptr @{function}(ptr {}, i64 {}, i32 {})\n",
                    map.operand,
                    element_type.size_bytes(),
                    self.contents_flags(element_type)
                ));
                self.emit_exception_check(body);
                let array_type = self.array_of(element_type);
                Ok(self.own(result(array_type, array)))
            }
            _ => {
                // map_key_exists(key, map) and in_map(value, map): the value
                // looked for comes first, as in PHP.
                let map = {
                    // Evaluated in order, but the map's type decides how the
                    // first argument is read (e.g. a null literal).
                    let first = arguments[0].clone();
                    let looked_for_first = !matches!(first, Expr::Null | Expr::ArrayLiteral(_) | Expr::MapLiteral(_));
                    if looked_for_first {
                        None
                    } else {
                        Some(self.emit_expression(&arguments[1], variables, body)?)
                    }
                };
                let (looked_for, map) = match map {
                    Some(map) => {
                        let signature = map_signature(self, &map)?;
                        let expected = if name == "in_map" { signature.value_type } else { signature.key_type };
                        (self.emit_expecting(&arguments[0], Some(expected), variables, body)?, map)
                    }
                    None => {
                        // Borrowed when evaluating the map can't change anything.
                        let looked_for = if is_pure(&arguments[1]) {
                            self.emit_borrowed(&arguments[0], None, variables, body)?
                        } else {
                            self.emit_expression(&arguments[0], variables, body)?
                        };
                        (looked_for, self.emit_expression(&arguments[1], variables, body)?)
                    }
                };
                let signature = map_signature(self, &map)?;
                let register = self.new_register();
                if name == "map_key_exists" {
                    if looked_for.value_type != signature.key_type {
                        return Err(format!(
                            "map key requires {}, found {}",
                            self.type_name(signature.key_type),
                            self.type_name(looked_for.value_type)
                        ));
                    }
                    let key_bits = self.encode_payload(&looked_for, body)?;
                    body.push_str(&format!("  {register} = call zeroext i1 @qiq_map_has(ptr {}, i64 {key_bits})\n", map.operand));
                } else {
                    if !self.is_assignable(signature.value_type, looked_for.value_type) {
                        return Err(format!(
                            "in_map: this map's values are {}, found {}",
                            self.type_name(signature.value_type),
                            self.type_name(looked_for.value_type)
                        ));
                    }
                    // How values compare: as == does (arrays and maps by
                    // contents), and objects and functions by identity ("o").
                    let descriptor = self.type_descriptor(signature.value_type);
                    let descriptor = self.string_literal(descriptor.as_bytes());
                    let bits = if looked_for.value_type == ValueType::Null || looked_for.operand == "null" {
                        "0".to_string()
                    } else {
                        self.encode_payload(&looked_for, body)?
                    };
                    body.push_str(&format!(
                        "  {register} = call zeroext i1 @qiq_map_contains_value(ptr {}, i64 {bits}, ptr {descriptor})\n",
                        map.operand
                    ));
                }
                self.emit_exception_check(body);
                Ok(result(ValueType::Bool, register))
            }
        }
    }

    /// `json_encode(value, pretty = false)`: the runtime (json.c) walks the
    /// value with its JSON descriptor.
    fn emit_json_encode(
        &mut self,
        arguments: &[Expr],
        variables: &HashMap<String, Value>,
        body: &mut String,
    ) -> Result<Value, String> {
        if arguments.iter().any(|argument| matches!(argument, Expr::NamedArgument(..))) {
            return Err("json_encode takes no named arguments".to_string());
        }
        let (value_expression, pretty_expression) = match arguments {
            [value] => (value, None),
            [value, pretty] => (value, Some(pretty)),
            _ => return Err(format!("json_encode expects 1 or 2 arguments, found {}", arguments.len())),
        };
        let value = self.emit_expression(value_expression, variables, body)?;
        if value.value_type == ValueType::Void {
            return Err("json_encode needs a value".to_string());
        }
        let pretty = match pretty_expression {
            Some(expression) => {
                let pretty = self.emit_expression(expression, variables, body)?;
                if pretty.value_type != ValueType::Bool {
                    return Err(format!("json_encode's second argument (pretty) must be a bool, found {}", self.type_name(pretty.value_type)));
                }
                pretty.operand
            }
            None => "false".to_string(),
        };
        let descriptor = self.json_descriptor(value.value_type);
        let descriptor = self.string_literal(descriptor.as_bytes());
        let payload = if value.value_type == ValueType::Null {
            "0".to_string()
        } else {
            self.encode_payload(&value, body)?
        };
        let result = self.new_register();
        body.push_str(&format!(
            "  {result} = call ptr @qiq_json_encode(ptr {descriptor}, i64 {payload}, i1 zeroext {pretty})\n"
        ));
        self.emit_exception_check(body);
        Ok(self.own(Value {
            value_type: ValueType::String,
            operand: result,
            slot: None,
        }))
    }

    /// `json_decode(text)` into `target`, checked against the type by the
    /// runtime (json.c) as it parses.
    fn emit_json_decode(
        &mut self,
        arguments: &[Expr],
        target: ValueType,
        variables: &HashMap<String, Value>,
        body: &mut String,
    ) -> Result<Value, String> {
        let [text_expression] = arguments else {
            return Err(format!("json_decode expects 1 argument, found {}", arguments.len()));
        };
        if matches!(text_expression, Expr::NamedArgument(..)) {
            return Err("json_decode takes no named arguments".to_string());
        }
        let mut reached = Vec::new();
        self.check_json_target(target, &mut reached)?;
        // json_decode creates objects of these classes.
        for class_id in reached {
            self.use_class(class_id);
        }
        let text = self.emit_expression(text_expression, variables, body)?;
        if text.value_type != ValueType::String {
            return Err(format!("json_decode expects a string, found {}", self.type_name(text.value_type)));
        }
        let descriptor = self.json_descriptor(target);
        let descriptor = self.string_literal(descriptor.as_bytes());
        let payload = self.new_register();
        body.push_str(&format!(
            "  {payload} = call i64 @qiq_json_decode(ptr {}, ptr {descriptor})\n",
            text.operand
        ));
        self.emit_exception_check(body);
        let value = self.decode_payload(target, &payload, body)?;
        Ok(self.own(value))
    }

    /// Whether json_decode can build values of this type: every class it
    /// reaches through public properties must be instantiable.
    fn check_json_target(&self, value_type: ValueType, visited: &mut Vec<u32>) -> Result<(), String> {
        match value_type {
            ValueType::Void | ValueType::Null => Err("json_decode can't decode into null".to_string()),
            ValueType::Array(_) => self.check_json_target(self.element_of(value_type).expect("array type"), visited),
            ValueType::Map(type_id) => self.check_json_target(self.map_types[type_id as usize].value_type, visited),
            ValueType::Class(class_id) => {
                if visited.contains(&class_id) {
                    return Ok(());
                }
                visited.push(class_id);
                let class = &self.classes[class_id as usize];
                if [self.stdlib_class("DateTime"), self.stdlib_class("Duration"), self.stdlib_class("Url")].contains(&Some(class_id)) {
                    let reader = match class.name.as_str() {
                        "DateTime" => "DateTime::parse",
                        "Url" => "Url::parse",
                        _ => "Duration's constructors",
                    };
                    return Err(format!("json_decode can't create a {} yet: decode it as a string and read that with {reader}", class.name));
                }
                if class.is_interface || class.is_abstract {
                    return Err(format!(
                        "json_decode can't create {}, which is {}; decode into a concrete class",
                        class.name,
                        if class.is_interface { "an interface" } else { "abstract" }
                    ));
                }
                for (property, _) in self.class_fields(class_id) {
                    if property.visibility == Visibility::Public {
                        self.check_json_target(property.value_type, visited)?;
                    }
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// A type's descriptor for json.c: like `type_descriptor`, with each
    /// class id written as `o<id>;`.
    fn json_descriptor(&self, value_type: ValueType) -> String {
        match value_type {
            ValueType::Class(class_id) => format!("o{class_id};"),
            ValueType::Array(_) => format!("a{}", self.json_descriptor(self.element_of(value_type).expect("array type"))),
            ValueType::Map(type_id) => {
                let map = self.map_types[type_id as usize];
                format!("m{}{}", self.json_descriptor(map.key_type), self.json_descriptor(map.value_type))
            }
            other => self.type_descriptor(other),
        }
    }

    /// An object's properties with their slots, in layout order (inherited
    /// first).
    fn class_fields(&self, class_id: u32) -> Vec<(Property, usize)> {
        let class = &self.classes[class_id as usize];
        let mut fields = match class.parent_class {
            Some(parent) => self.class_fields(parent),
            None => Vec::new(),
        };
        for property in &class.properties {
            let slot = OBJECT_HEADER_FIELDS + fields.len();
            fields.push((property.clone(), slot));
        }
        fields
    }

    /// `len(value)`: characters in a string, elements in an array, or entries
    /// in a map. A null value throws RuntimeException.
    fn emit_len(&mut self, value: Value, body: &mut String) -> Result<Value, String> {
        let function = match value.value_type {
            ValueType::String => "qiq_len_string",
            ValueType::Array(_) => "qiq_len_array",
            ValueType::Map(_) => "qiq_len_map",
            other => {
                return Err(format!(
                    "len expects a string, array, or map, found {}",
                    self.type_name(other)
                ))
            }
        };
        let register = self.new_register();
        body.push_str(&format!("  {register} = call i64 @{function}(ptr {})\n", value.operand));
        self.emit_exception_check(body);
        Ok(Value {
            value_type: ValueType::Int,
            operand: register,
            slot: None,
        })
    }

    /// Prints one scalar or null with a single printf and a newline.
    fn emit_print_scalar(&mut self, value: &Value, body: &mut String) {
        match value.value_type {
            ValueType::Int => body.push_str(&format!(
                "  call i32 (ptr, ...) @printf(ptr @.fmt.int, i64 {})\n",
                value.operand
            )),
            ValueType::Double => body.push_str(&format!(
                "  call i32 (ptr, ...) @printf(ptr @.fmt.double, double {})\n",
                value.operand
            )),
            ValueType::Bool => {
                let register = self.new_register();
                body.push_str(&format!(
                    "  {register} = select i1 {}, ptr @.str.true, ptr @.str.false\n  call i32 (ptr, ...) @printf(ptr @.fmt.bool, ptr {register})\n",
                    value.operand
                ));
            }
            ValueType::Byte => {
                let register = self.new_register();
                body.push_str(&format!(
                    "  {register} = zext i8 {} to i64\n  call i32 (ptr, ...) @printf(ptr @.fmt.int, i64 {register})\n",
                    value.operand
                ));
            }
            ValueType::String | ValueType::Char => {
                // A null string prints as "null", as it does among other
                // values (musl's printf would crash on a null pointer).
                let null = self.new_register();
                let text = self.new_register();
                body.push_str(&format!(
                    "  {null} = icmp eq ptr {operand}, null\n  {text} = select i1 {null}, ptr @.str.null, ptr {operand}\n  call i32 (ptr, ...) @printf(ptr @.fmt.string, ptr {text})\n",
                    operand = value.operand
                ));
            }
            ValueType::Null => body.push_str(
                "  call i32 (ptr, ...) @printf(ptr @.fmt.string, ptr @.str.null)\n",
            ),
            ValueType::Void | ValueType::Array(_) | ValueType::Map(_) | ValueType::Class(_) | ValueType::Function(_) => {
                unreachable!("only scalars and null take the printf path")
            }
        }
    }

    /// The runtime's description of a type for print (see stdlib/runtime/print.c).
    fn type_descriptor(&self, value_type: ValueType) -> String {
        match value_type {
            ValueType::Int => "i".to_string(),
            ValueType::Double => "d".to_string(),
            ValueType::Bool => "b".to_string(),
            ValueType::String => "s".to_string(),
            ValueType::Char => "c".to_string(),
            ValueType::Byte => "y".to_string(),
            ValueType::Null | ValueType::Void => "n".to_string(),
            // Function values are objects too, printed as "Closure".
            ValueType::Class(_) | ValueType::Function(_) => "o".to_string(),
            ValueType::Array(_) => format!(
                "a{}",
                self.type_descriptor(self.element_of(value_type).expect("array type"))
            ),
            ValueType::Map(type_id) => {
                let map = self.map_types[type_id as usize];
                format!("m{}{}", self.type_descriptor(map.key_type), self.type_descriptor(map.value_type))
            }
        }
    }

    /// A type as written in qiq source, for error messages.
    fn type_name(&self, value_type: ValueType) -> String {
        type_name(value_type, &self.map_types, &self.array_types, &self.function_types, &self.classes)
    }

    fn array_of(&mut self, element: ValueType) -> ValueType {
        array_type_of(&mut self.array_types, element)
    }

    fn element_of(&self, value_type: ValueType) -> Option<ValueType> {
        element_type_of(&self.array_types, value_type)
    }

    /// Emits `expression`, giving array and map literals the `expected` type
    /// when one is known (an empty literal needs it). Other expressions are
    /// emitted as usual and type-checked by the caller.
    fn emit_expecting(
        &mut self,
        expression: &Expr,
        expected: Option<ValueType>,
        variables: &HashMap<String, Value>,
        body: &mut String,
    ) -> Result<Value, String> {
        match expression {
            Expr::Number(number) if expected == Some(ValueType::Byte) => {
                if !(0..=255).contains(number) {
                    return Err(format!("{number} doesn't fit in a byte (0 to 255)"));
                }
                Ok(Value {
                    value_type: ValueType::Byte,
                    operand: number.to_string(),
                    slot: None,
                })
            }
            Expr::ArrayLiteral(elements) => self.emit_array_literal(elements, expected, variables, body),
            Expr::MapLiteral(entries) => self.emit_map_literal(entries, expected, variables, body),
            Expr::Closure(closure) => self.emit_closure(closure, expected, variables, body),
            // Database::queryAs reads rows into the type its result is used as.
            Expr::MethodCall(_, name, _) if matches!(name.as_str(), "queryAs" | "queryOneAs" | "nextAs" | "fetchAs") => {
                self.call_expectation = expected;
                let result = self.emit_expression(expression, variables, body);
                self.call_expectation = None;
                result
            }
            // json_decode decodes into the type its result is used as.
            Expr::Call(name, arguments) if name == "json_decode" && expected.is_some_and(|expected| !matches!(expected, ValueType::Void | ValueType::Null)) => {
                self.emit_json_decode(arguments, expected.expect("checked above"), variables, body)
            }
            // `null` can be stored wherever a reference is expected: a string,
            // char, array, map, or object variable, property, element, map
            // value, argument, or return value.
            Expr::Null if expected.is_some_and(is_reference) => Ok(Value {
                value_type: expected.expect("checked above"),
                operand: "null".to_string(),
                slot: None,
            }),
            _ => self.emit_expression(expression, variables, body),
        }
    }

    /// `[a, b, ...]`: the element type is the expected array type's, or else
    /// the first element's.
    fn emit_array_literal(
        &mut self,
        elements: &[Expr],
        expected: Option<ValueType>,
        variables: &HashMap<String, Value>,
        body: &mut String,
    ) -> Result<Value, String> {
        let mut element_type = expected.and_then(|expected| self.element_of(expected));
        if let (Some(expected), None, true) = (expected, element_type, elements.is_empty()) {
            return Err(format!("an array literal cannot be used as {}", self.type_name(expected)));
        }
        if element_type.is_none() && elements.is_empty() {
            return Err(
                "cannot infer the type of an empty array literal []; use it where an array type is known, such as `int[] values = [];`"
                    .to_string(),
            );
        }
        let mut values = Vec::new();
        for element in elements {
            let value = self.emit_expecting(element, element_type, variables, body)?;
            let required = *element_type.get_or_insert(value.value_type);
            if !self.is_assignable(required, value.value_type) {
                return Err(format!(
                    "array elements must all be {}, found {}",
                    self.type_name(required),
                    self.type_name(value.value_type)
                ));
            }
            values.push(value);
        }
        let element_type = element_type.expect("set from expected type or first element");
        if matches!(element_type, ValueType::Void | ValueType::Null) {
            return Err(format!("arrays of {} are not supported", self.type_name(element_type)));
        }
        let array_type = self.array_of(element_type);
        let element_llvm = element_type.llvm();
        let array = self.new_register();
        let data = self.new_register();
        body.push_str(&format!(
            "  {array} = call ptr @qiq_array_new(i64 {}, i64 {}, i32 {})\n  {data}.at = getelementptr inbounds i8, ptr {array}, i64 {ARRAY_DATA_OFFSET}\n  {data} = load ptr, ptr {data}.at, align 8\n",
            values.len(),
            element_type.size_bytes(),
            self.contents_flags(element_type)
        ));
        // The array holds a reference to each element.
        let values: Vec<String> = values.iter().map(|value| self.take(value, body)).collect();
        for (index, value) in values.iter().enumerate() {
            let address = self.new_register();
            body.push_str(&format!(
                "  {address} = getelementptr inbounds {element_llvm}, ptr {data}, i64 {index}\n  store {element_llvm} {value}, ptr {address}, align {}\n",
                element_type.alignment()
            ));
        }
        Ok(self.own(Value {
            value_type: array_type,
            operand: array,
            slot: None,
        }))
    }

    /// `{key => value, ...}`: the key and value types are the expected map
    /// type's, or else the first entry's.
    fn emit_map_literal(
        &mut self,
        entries: &[(Expr, Expr)],
        expected: Option<ValueType>,
        variables: &HashMap<String, Value>,
        body: &mut String,
    ) -> Result<Value, String> {
        let mut signature = match expected {
            Some(ValueType::Map(type_id)) => Some(
                *self
                    .map_types
                    .get(type_id as usize)
                    .ok_or_else(|| "invalid map type descriptor".to_string())?,
            ),
            _ => None,
        };
        if let (Some(expected), None, true) = (expected, signature, entries.is_empty()) {
            return Err(format!("a map literal cannot be used as {}", self.type_name(expected)));
        }
        if signature.is_none() && entries.is_empty() {
            return Err(
                "cannot infer the type of an empty map literal {}; use it where a map type is known, such as `map[string, int] counts = {};`"
                    .to_string(),
            );
        }
        // Entries are evaluated in order before the map is created, since an
        // unannotated literal's types come from its first entry.
        let mut evaluated = Vec::new();
        for (key_expression, value_expression) in entries {
            let key = self.emit_expression(key_expression, variables, body)?;
            let value = self.emit_expecting(
                value_expression,
                signature.map(|signature| signature.value_type),
                variables,
                body,
            )?;
            let required = *signature.get_or_insert(MapType {
                key_type: key.value_type,
                value_type: value.value_type,
            });
            if key.value_type != required.key_type {
                return Err(format!(
                    "map key requires {}, found {}",
                    self.type_name(required.key_type),
                    self.type_name(key.value_type)
                ));
            }
            if !self.is_assignable(required.value_type, value.value_type) {
                return Err(format!(
                    "map value requires {}, found {}",
                    self.type_name(required.value_type),
                    self.type_name(value.value_type)
                ));
            }
            evaluated.push((key, value));
        }
        let signature = signature.expect("set from expected type or first entry");
        if !signature.key_type.is_scalar_key() {
            return Err(format!("map keys must be scalar, found {}", self.type_name(signature.key_type)));
        }
        if matches!(signature.value_type, ValueType::Void | ValueType::Null) {
            return Err("map values cannot be void or null".to_string());
        }
        let type_id = self.map_type_id(signature);
        let map = self.new_register();
        body.push_str(&format!(
            "  {map} = call ptr @qiq_map_new(i32 {}, i32 {})\n",
            signature.key_type.runtime_tag()?,
            self.contents_flags(signature.value_type)
        ));
        for (key, value) in &evaluated {
            // The map takes a reference to each key and value.
            self.take(key, body);
            self.take(value, body);
            let key_bits = self.encode_payload(key, body)?;
            let value_bits = self.encode_payload(value, body)?;
            body.push_str(&format!(
                "  call void @qiq_map_set(ptr {map}, i64 {key_bits}, i64 {value_bits})\n"
            ));
            self.emit_exception_check(body);
        }
        Ok(self.own(Value {
            value_type: ValueType::Map(type_id),
            operand: map,
            slot: None,
        }))
    }

    /// The id of the map type with these key and value types, registering it
    /// if no declaration named it.
    fn map_type_id(&mut self, signature: MapType) -> u32 {
        if let Some(index) = self.map_types.iter().position(|candidate| {
            candidate.key_type == signature.key_type && candidate.value_type == signature.value_type
        }) {
            return index as u32;
        }
        self.map_types.push(signature);
        (self.map_types.len() - 1) as u32
    }

    /// Checks `index` against the array's bounds (throwing OutOfRangeException,
    /// or RuntimeException for a null array) and returns the element's address.
    /// The check is inline (see emit_fast_path); only a null array or an
    /// index out of range calls the runtime, which throws.
    fn emit_element_address(&mut self, array: &Value, element_type: ValueType, index: &str, body: &mut String) -> String {
        let id = self.next_label;
        self.next_label += 1;
        let operand = &array.operand;
        body.push_str(&format!(
            "  %chk.null{id} = icmp eq ptr {operand}, null\n  br i1 %chk.null{id}, label %chk.slow{id}, label %chk.length{id}\nchk.length{id}:\n  %chk.l{id} = load i64, ptr {operand}, align 8\n  %chk.in{id} = icmp ult i64 {index}, %chk.l{id}\n  br i1 %chk.in{id}, label %chk.fast{id}, label %chk.slow{id}\nchk.slow{id}:\n  call void @qiq_array_check_index(ptr {operand}, i64 {index})\n"
        ));
        self.emit_exception_check(body);
        body.push_str(&format!("  br label %chk.fast{id}\nchk.fast{id}:\n"));
        let data = self.new_register();
        let address = self.new_register();
        body.push_str(&format!(
            "  {data}.at = getelementptr inbounds i8, ptr {}, i64 {ARRAY_DATA_OFFSET}\n  {data} = load ptr, ptr {data}.at, align 8\n  {address} = getelementptr inbounds {}, ptr {data}, i64 {index}\n",
            array.operand,
            element_type.llvm()
        ));
        address
    }

    /// A checked operation done inline on its common path. The runtime's
    /// checked functions use the exception guard (setjmp), so LLVM can't
    /// inline them; calling one for every `%`, `/`, or array index would cost
    /// a call and a setjmp each time. Instead `condition` computes an i1,
    /// `%chk.ok#` (with `#` standing for this check's number), that is true
    /// when `fast` gives the right result; otherwise the runtime function
    /// `slow` runs, which throws for the error cases (and handles any others,
    /// such as INT64_MIN / -1). Writes the result, of type `llvm_type`, to
    /// `register`. Registers are named from the label counter, as in
    /// emit_exception_check.
    fn emit_fast_path(&mut self, register: &str, llvm_type: &str, condition: &str, fast: &str, slow: &str, body: &mut String) {
        let id = self.next_label;
        self.next_label += 1;
        let condition = condition.replace('#', &id.to_string());
        body.push_str(&format!(
            "{condition}  br i1 %chk.ok{id}, label %chk.fast{id}, label %chk.slow{id}\nchk.fast{id}:\n  %chk.f{id} = {fast}\n  br label %chk.join{id}\nchk.slow{id}:\n  %chk.s{id} = {slow}\n"
        ));
        let checked = self.next_label;
        self.emit_exception_check(body);
        body.push_str(&format!(
            "  br label %chk.join{id}\nchk.join{id}:\n  {register} = phi {llvm_type} [ %chk.f{id}, %chk.fast{id} ], [ %chk.s{id}, %ok{checked} ]\n"
        ));
    }

    /// After a call that can throw: if an exception is pending, branch to the
    /// innermost handler. Registers are named from the label counter so the
    /// check doesn't renumber the surrounding `%vN` registers.
    fn emit_exception_check(&mut self, body: &mut String) {
        let (handler, cleanup) = self.handler_cleanup();
        let id = self.next_label;
        self.next_label += 1;
        if cleanup.is_empty() {
            body.push_str(&format!(
                "  %pending{id} = load ptr, ptr @qiq_pending_exception, align 8\n  %thrown{id} = icmp ne ptr %pending{id}, null\n  br i1 %thrown{id}, label %{handler}, label %ok{id}\nok{id}:\n"
            ));
        } else {
            body.push_str(&format!(
                "  %pending{id} = load ptr, ptr @qiq_pending_exception, align 8\n  %thrown{id} = icmp ne ptr %pending{id}, null\n  br i1 %thrown{id}, label %cleanup{id}, label %ok{id}\ncleanup{id}:\n{cleanup}  br label %{handler}\nok{id}:\n"
            ));
        }
    }

    /// The innermost handler, and the releases an exception going there
    /// needs: the temporaries of statements inside it. In main, an uncaught
    /// exception ends the program, so it needs none.
    fn handler_cleanup(&self) -> (String, String) {
        let handler = self.handlers.last().cloned().unwrap_or_else(|| "unwind".to_string());
        let first_scope = self.handler_scopes.last().copied().unwrap_or(0);
        let mut cleanup = String::new();
        if handler == "unwind" && self.current_method.is_none() {
            return (handler, cleanup);
        }
        for scope in self.temporaries[first_scope.min(self.temporaries.len())..].iter().rev() {
            for operand in scope.iter().rev() {
                cleanup.push_str(&format!("  call void @qiq_release(ptr {operand})\n"));
            }
        }
        (handler, cleanup)
    }

    /// Emits the finally block at `index` of the finally stack, as code on
    /// the current path. Exceptions inside it go to the handler outside its
    /// try, and enclosing finally blocks stay active.
    fn emit_finally(&mut self, index: usize, return_type: ValueType, is_main: bool) -> Result<(String, bool), String> {
        let context = self.finally_stack[index].clone();
        let saved_handlers = std::mem::take(&mut self.handlers);
        let saved_scopes = std::mem::take(&mut self.handler_scopes);
        let saved_finally = std::mem::take(&mut self.finally_stack);
        self.handlers = saved_handlers[..context.handler_depth].to_vec();
        self.handler_scopes = saved_scopes[..context.handler_depth].to_vec();
        self.finally_stack = saved_finally[..index].to_vec();
        let result = self.emit_block(&context.body, &context.variables, return_type, is_main);
        self.handlers = saved_handlers;
        self.handler_scopes = saved_scopes;
        self.finally_stack = saved_finally;
        result
    }

    /// Emits the finally blocks from the innermost down to index `down_to`,
    /// for a return, break, or continue that leaves their try statements.
    fn emit_finally_chain(&mut self, down_to: usize, return_type: ValueType, is_main: bool) -> Result<(String, bool), String> {
        let mut ir = String::new();
        for index in (down_to..self.finally_stack.len()).rev() {
            let (block, terminated) = self.emit_finally(index, return_type, is_main)?;
            ir.push_str(&block);
            if terminated {
                return Ok((ir, true));
            }
        }
        Ok((ir, false))
    }

    /// The standard library class named `name`.
    fn stdlib_class(&self, name: &str) -> Option<u32> {
        self.classes
            .iter()
            .position(|class| class.name == name && self.module_is_stdlib[class.module as usize])
            .map(|index| index as u32)
    }

    fn exception_class(&self) -> Result<u32, String> {
        self.stdlib_class("Exception")
            .ok_or_else(|| "the standard library has no Exception class".to_string())
    }

    /// `@qiq_new.<Class>(constructor arguments)`: allocates and constructs
    /// one class, for `new static`. For an abstract class it throws instead.
    fn emit_static_factory(&mut self, class_id: u32) -> Result<String, String> {
        let class = self.classes[class_id as usize].clone();
        let owner = self.find_implementation(class_id, "__construct");
        let parameters: Vec<Parameter> = owner
            .and_then(|owner| {
                self.classes[owner as usize]
                    .methods
                    .iter()
                    .find(|method| method.name == "__construct")
                    .map(|method| method.parameters.clone())
            })
            .unwrap_or_default();
        let declared: Vec<String> = parameters
            .iter()
            .enumerate()
            .map(|(index, parameter)| format!("{} %arg{index}", parameter.value_type.llvm()))
            .collect();
        let mut body = String::new();
        if class.is_abstract {
            let message = self.emit_expression(
                &Expr::Text(format!("cannot instantiate abstract class '{}'", class.name)),
                &HashMap::new(),
                &mut body,
            )?;
            body.push_str(&format!(
                "  call void @qiq_raise(i32 0, ptr {})\n  ret ptr null\n",
                message.operand
            ));
        } else {
            let field_count = OBJECT_HEADER_FIELDS + self.class_field_count(class_id)?;
            body.push_str(&format!(
                "  %object = call nonnull ptr @qiq_object_new(i64 {field_count}, i64 {class_id})\n"
            ));
            if self.has_property_defaults(class_id) {
                body.push_str(&format!("  call void @qiq_defaults.{}(ptr %object)\n", class.symbol));
            }
            if let Some(owner) = owner {
                let mut operands = vec!["ptr %object".to_string()];
                operands.extend(declared.iter().cloned());
                body.push_str(&format!(
                    "  call void @qiq_{}___construct({})\n",
                    self.classes[owner as usize].symbol,
                    operands.join(", ")
                ));
            }
            body.push_str("  ret ptr %object\n");
        }
        Ok(format!(
            "define ptr @qiq_new.{}({}) {{\nentry:\n{body}}}\n",
            class.symbol,
            declared.join(", ")
        ))
    }

    /// One global per static property, holding its initial value.
    fn emit_static_property_globals(&mut self) -> Result<String, String> {
        let mut ir = String::new();
        for class in self.classes.clone() {
            for property in &class.static_properties {
                let initial = match &property.default {
                    Some(default) => {
                        let mut scratch = String::new();
                        let value = self.emit_expecting(default, Some(property.value_type), &HashMap::new(), &mut scratch)?;
                        value.operand
                    }
                    None => match property.value_type {
                        ValueType::Int | ValueType::Byte => "0".to_string(),
                        ValueType::Double => "0.0".to_string(),
                        ValueType::Bool => "false".to_string(),
                        _ => "null".to_string(),
                    },
                };
                ir.push_str(&format!(
                    "@qiq_static.{}.{} = internal global {} {initial}, align 8\n",
                    class.symbol,
                    property.name,
                    property.value_type.llvm()
                ));
            }
        }
        Ok(ir)
    }

    /// Tables indexed by class id for static:: and new static.
    fn emit_late_tables(&self) -> String {
        let mut ir = String::new();
        let table = |name: String, entries: Vec<String>| {
            format!(
                "@{name} = private unnamed_addr constant [{} x ptr] [{}], align 8\n",
                entries.len(),
                entries.join(", ")
            )
        };
        let subtype_entries = |enclosing: u32, entry: &dyn Fn(u32) -> Option<String>| -> Vec<String> {
            (0..self.classes.len() as u32)
                .map(|class_id| {
                    let in_hierarchy = !self.classes[class_id as usize].is_interface
                        && (class_id == enclosing || self.is_subclass(class_id, enclosing));
                    in_hierarchy
                        .then(|| entry(class_id))
                        .flatten()
                        .map_or("ptr null".to_string(), |symbol| format!("ptr {symbol}"))
                })
                .collect()
        };
        for (enclosing, method) in &self.late_method_tables {
            let entries = subtype_entries(*enclosing, &|class_id| {
                self.find_declaring_class(class_id, method)
                    .map(|owner| format!("@qiq_{}_{method}", self.classes[owner as usize].symbol))
            });
            ir.push_str(&table(
                format!("qiq_static_dispatch.{}.{method}", self.classes[*enclosing as usize].symbol),
                entries,
            ));
        }
        for (enclosing, property) in &self.late_property_tables {
            let entries = subtype_entries(*enclosing, &|class_id| {
                self.find_static_property(class_id, property)
                    .map(|(owner, _)| format!("@qiq_static.{}.{property}", self.classes[owner as usize].symbol))
            });
            ir.push_str(&table(
                format!("qiq_static_slots.{}.{property}", self.classes[*enclosing as usize].symbol),
                entries,
            ));
        }
        for enclosing in &self.new_static_tables {
            let entries = subtype_entries(*enclosing, &|class_id| {
                Some(format!("@qiq_new.{}", self.classes[class_id as usize].symbol))
            });
            ir.push_str(&table(
                format!("qiq_new_static.{}", self.classes[*enclosing as usize].symbol),
                entries,
            ));
        }
        ir
    }

    /// `@qiq_raise(kind, message)`, called by the C runtime: creates the
    /// exception for `kind` (see RUNTIME_EXCEPTION_CLASSES) and makes it pending.
    fn emit_raise_function(&self) -> Result<String, String> {
        let mut cases = String::new();
        let mut blocks = String::new();
        for (kind, class_name) in RUNTIME_EXCEPTION_CLASSES.iter().enumerate() {
            let class_id = self
                .stdlib_class(class_name)
                .ok_or_else(|| format!("the standard library has no {class_name} class"))?;
            let owner = self
                .find_implementation(class_id, "__construct")
                .ok_or_else(|| format!("{class_name} has no constructor"))?;
            let field_count = OBJECT_HEADER_FIELDS + self.class_field_count(class_id)?;
            if kind > 0 {
                cases.push_str(&format!(" i32 {kind}, label %kind{kind}"));
            }
            blocks.push_str(&format!(
                "kind{kind}:\n  %object{kind} = call nonnull ptr @qiq_object_new(i64 {field_count}, i64 {class_id})\n  call void @qiq_{}___construct(ptr %object{kind}, ptr %message, i64 0)\n  store ptr %object{kind}, ptr @qiq_pending_exception, align 8\n  ret void\n",
                self.classes[owner as usize].symbol
            ));
        }
        Ok(format!(
            "define void @qiq_raise(i32 %kind, ptr %message) {{\nentry:\n  switch i32 %kind, label %kind0 [{cases} ]\n{blocks}}}\n"
        ))
    }

    /// `@qiq_class_names`: each class's name, indexed by class id, for
    /// reporting uncaught exceptions.
    fn emit_class_names(&self) -> String {
        let mut ir = String::new();
        let mut entries = Vec::new();
        for (class_id, class) in self.classes.iter().enumerate() {
            let bytes = class.name.as_bytes();
            ir.push_str(&format!(
                "@.class.name{class_id} = private unnamed_addr constant [{} x i8] c\"{}\\00\", align 1\n",
                bytes.len() + 1,
                llvm_escape(bytes)
            ));
            entries.push(format!("ptr @.class.name{class_id}"));
        }
        ir.push_str(&format!(
            // Not private: the runtime's print reads it for object values.
            "@qiq_class_names = unnamed_addr constant [{} x ptr] [{}], align 8\n",
            entries.len(),
            entries.join(", ")
        ));
        // For freeing objects and finding cycles: per class, one character
        // per field after the class id: '2' for a reference that can be part
        // of a cycle, '1' for any other reference, '0' for the rest.
        let mut fields = Vec::new();
        for class_id in 0..self.classes.len() as u32 {
            let mask: String = self
                .class_field_types(class_id)
                .into_iter()
                .map(|value_type| {
                    if self.may_cycle(value_type) {
                        '2'
                    } else if is_reference(value_type) {
                        '1'
                    } else {
                        '0'
                    }
                })
                .collect();
            ir.push_str(&format!(
                "@.class.fields{class_id} = private unnamed_addr constant [{} x i8] c\"{mask}\\00\", align 1\n",
                mask.len() + 1
            ));
            fields.push(format!("ptr @.class.fields{class_id}"));
        }
        ir.push_str(&format!(
            "@qiq_class_reference_fields = unnamed_addr constant [{} x ptr] [{}], align 8\n",
            fields.len(),
            fields.join(", ")
        ));
        // For JSON: per class, "<field count>|" then "<name>:<slot>:<descriptor>;"
        // for each public property.
        // std/time's DateTime and Duration, and std/net's Url, are values
        // rather than records: "!T<microseconds slot>,<zone slot>" and
        // "!D<microseconds slot>", which json.c writes as ISO 8601 text, and
        // "!S<string slot>", written as that string.
        let mut json = Vec::new();
        let slot_of = |fields: &[(Property, usize)], name: &str| fields.iter().find(|(property, _)| property.name == name).map(|(_, slot)| *slot);
        for class_id in 0..self.classes.len() as u32 {
            let fields = self.class_fields(class_id);
            let mut text = format!("{}|", OBJECT_HEADER_FIELDS + fields.len());
            if self.stdlib_class("DateTime") == Some(class_id) {
                text = format!("!T{},{}", slot_of(&fields, "micros").unwrap_or(0), slot_of(&fields, "zoneName").unwrap_or(0));
            } else if self.stdlib_class("Duration") == Some(class_id) {
                text = format!("!D{}", slot_of(&fields, "micros").unwrap_or(0));
            } else if self.stdlib_class("Url") == Some(class_id) {
                text = format!("!S{}", slot_of(&fields, "text").unwrap_or(0));
            }
            let special = text.starts_with('!');
            for (property, slot) in fields.iter().filter(|(property, _)| property.visibility == Visibility::Public && !special) {
                text.push_str(&format!("{}:{slot}:{};", property.name, self.json_descriptor(property.value_type)));
            }
            ir.push_str(&format!(
                "@.class.json{class_id} = private unnamed_addr constant [{} x i8] c\"{}\\00\", align 1\n",
                text.len() + 1,
                llvm_escape(text.as_bytes())
            ));
            json.push(format!("ptr @.class.json{class_id}"));
        }
        ir.push_str(&format!(
            "@qiq_class_json = unnamed_addr constant [{} x ptr] [{}], align 8\n",
            json.len(),
            json.join(", ")
        ));
        ir
    }

    /// Whether a value of this type can be part of a reference cycle: an
    /// object that can (see class_may_cycle), a function, or an array or map
    /// that can hold one. Only these are tracked by the cycle collector, so a
    /// type that can't saves it the work.
    fn may_cycle(&self, value_type: ValueType) -> bool {
        self.may_cycle_within(value_type, &mut Vec::new())
    }

    fn may_cycle_within(&self, value_type: ValueType, visiting: &mut Vec<u32>) -> bool {
        match value_type {
            // A function value holds its captured variables, which can refer back to it.
            ValueType::Function(_) => true,
            ValueType::Class(class_id) => self.class_may_cycle(class_id, visiting),
            ValueType::Array(_) => self
                .element_of(value_type)
                .is_some_and(|element| self.may_cycle_within(element, visiting)),
            ValueType::Map(type_id) => self
                .map_types
                .get(type_id as usize)
                .is_some_and(|map| self.may_cycle_within(map.value_type, visiting)),
            _ => false,
        }
    }

    /// Whether an object of this class can be part of a reference cycle.
    /// The whole program is known, so a class no other class extends (and
    /// that isn't an interface) holds exactly its own fields: it can't be in
    /// a cycle when none of them can, as with an object holding only
    /// strings, numbers, and arrays of them, or other such objects. A class
    /// that can reach itself through its fields (`visiting`) can.
    fn class_may_cycle(&self, class_id: u32, visiting: &mut Vec<u32>) -> bool {
        let Some(class) = self.classes.get(class_id as usize) else { return true };
        if class.is_interface || visiting.contains(&class_id) {
            return true;
        }
        if self.classes.iter().any(|other| other.parent_class == Some(class_id)) {
            return true;
        }
        visiting.push(class_id);
        let result = self
            .class_field_types(class_id)
            .into_iter()
            .any(|field| self.may_cycle_within(field, visiting));
        visiting.pop();
        result
    }

    /// The runtime flags for an array's elements or a map's values of this
    /// type: 1 for references, plus 4 (QIQ_MAY_CYCLE) when they can be part
    /// of a cycle.
    fn contents_flags(&self, value_type: ValueType) -> i32 {
        i32::from(is_reference(value_type)) | if self.may_cycle(value_type) { 4 } else { 0 }
    }

    /// The types of an object's fields in layout order (inherited first).
    fn class_field_types(&self, class_id: u32) -> Vec<ValueType> {
        let class = &self.classes[class_id as usize];
        let mut types = match class.parent_class {
            Some(parent) => self.class_field_types(parent),
            None => Vec::new(),
        };
        types.extend(class.properties.iter().map(|property| property.value_type));
        types
    }

    /// The `unwind` block of a function: an exception is pending, so return a
    /// placeholder and let the caller's check propagate it.
    fn unwind_block(return_type: ValueType) -> String {
        let ret = match return_type {
            ValueType::Void => "ret void".to_string(),
            ValueType::Int => "ret i64 0".to_string(),
            ValueType::Double => "ret double 0.0".to_string(),
            ValueType::Bool => "ret i1 false".to_string(),
            ValueType::Byte => "ret i8 0".to_string(),
            other => format!("ret {} null", other.llvm()),
        };
        format!("unwind:\n  {ret}\n")
    }

    /// Prefixes an error with the path of the module it occurred in.
    fn in_module(&self, module: u32, error: String) -> String {
        match self.module_paths.get(module as usize) {
            Some(path) if !path.is_empty() && has_position(&error) => format!("{path}:{error}"),
            Some(path) if !path.is_empty() => format!("{path}: {error}"),
            _ => error,
        }
    }

    fn validate_class_contract(&self, class_id: u32, class: &ClassDef) -> Result<(), String> {
        // A redeclared static property must keep its type: static:: reads it
        // through the declaring class's type.
        if let Some(parent_id) = class.parent_class {
            for property in &class.static_properties {
                if let Some((owner, inherited)) = self.find_static_property(parent_id, &property.name) {
                    if inherited.value_type != property.value_type {
                        return Err(format!(
                            "static property '{}::{}' must keep the type {} of '{}::{}'",
                            class.name,
                            property.name,
                            self.type_name(inherited.value_type),
                            self.classes[owner as usize].name,
                            property.name
                        ));
                    }
                }
            }
        }
        // A final method (constructors included) can't be redeclared below it.
        if let Some(parent_id) = class.parent_class {
            for method in &class.methods {
                if let Some(owner) = self.find_declaring_class(parent_id, &method.name) {
                    let inherited = self.classes[owner as usize]
                        .methods
                        .iter()
                        .find(|candidate| candidate.name == method.name)
                        .expect("find_declaring_class found it");
                    if inherited.is_final {
                        return Err(at(method.line, method.column, format!(
                            "method '{}::{}' cannot override final method '{}::{}'",
                            class.name, method.name, self.classes[owner as usize].name, method.name
                        )));
                    }
                    if inherited.is_static != method.is_static {
                        let (kind, inherited_kind) = if method.is_static { ("static", "instance") } else { ("instance", "static") };
                        return Err(at(method.line, method.column, format!(
                            "{kind} method '{}::{}' cannot override {inherited_kind} method '{}::{}'",
                            class.name, method.name, self.classes[owner as usize].name, method.name
                        )));
                    }
                }
            }
        }

        // Methods redeclared from a superclass or interface must keep the
        // same signature and must not reduce visibility.
        for method in &class.methods {
            if method.name == "__construct" {
                continue;
            }
            // A class's own interfaces are checked below with a more specific
            // error; an interface checks the interfaces it extends here.
            let supertypes = if class.is_interface { &class.interfaces[..] } else { &[] };
            let inherited = class
                .parent_class
                .iter()
                .chain(supertypes.iter())
                .find_map(|&super_id| self.find_method(super_id, &method.name).ok());
            if let Some((parent_parameters, parent_return, parent_visibility, _, _)) = inherited {
                let same_types = method.parameters.len() == parent_parameters.len()
                    && method
                        .parameters
                        .iter()
                        .zip(&parent_parameters)
                        .all(|(left, right)| left.value_type == right.value_type);
                // `: static` matches only `: static` (they name different classes).
                let inherited_returns_static = class
                    .parent_class
                    .and_then(|parent| self.find_declaring_class(parent, &method.name))
                    .and_then(|owner| {
                        self.classes[owner as usize]
                            .methods
                            .iter()
                            .find(|candidate| candidate.name == method.name)
                            .map(|candidate| candidate.returns_static)
                    })
                    .unwrap_or(false);
                let same_return = if method.returns_static || inherited_returns_static {
                    method.returns_static == inherited_returns_static
                } else {
                    method.return_type == parent_return
                };
                if !same_types || !same_return {
                    return Err(at(method.line, method.column, format!(
                        "method '{}::{}' does not match the inherited signature",
                        class.name, method.name
                    )));
                }
                Self::check_keeps_optional(class, method, &parent_parameters)
                    .map_err(|error| at(method.line, method.column, error))?;
                if visibility_rank(method.visibility) < visibility_rank(parent_visibility) {
                    return Err(at(method.line, method.column, format!(
                        "method '{}::{}' cannot reduce inherited visibility",
                        class.name, method.name
                    )));
                }
            }
        }
        if class.is_interface {
            return Ok(());
        }

        if !class.is_abstract {
            if let Some(method) = class.methods.iter().find(|method| method.is_abstract) {
                return Err(at(method.line, method.column, format!(
                    "class '{}' must be abstract because method '{}' is abstract",
                    class.name, method.name
                )));
            }
            let mut ancestor = class.parent_class;
            while let Some(current_id) = ancestor {
                let current = &self.classes[current_id as usize];
                for method in current.methods.iter().filter(|method| method.is_abstract) {
                    if self.find_class_method(class_id, &method.name).is_some_and(|m| m.is_abstract) {
                        return Err(format!(
                            "concrete class '{}' must implement abstract method '{}'",
                            class.name, method.name
                        ));
                    }
                }
                ancestor = current.parent_class;
            }
        }

        // Every interface method must resolve to a public method of the
        // class (possibly inherited) with a matching signature.
        let mut interfaces = Vec::new();
        self.collect_interfaces(class_id, &mut interfaces);
        for interface_id in interfaces {
            let interface = &self.classes[interface_id as usize];
            for required in &interface.methods {
                match self.find_class_method(class_id, &required.name) {
                    Some(method) => {
                        if method.is_static {
                            return Err(format!(
                                "static method '{}::{}' can't implement interface method '{}::{}'",
                                class.name, required.name, interface.name, required.name
                            ));
                        }
                        let matches = method.return_type == required.return_type
                            && method.parameters.len() == required.parameters.len()
                            && method
                                .parameters
                                .iter()
                                .zip(&required.parameters)
                                .all(|(left, right)| left.value_type == right.value_type);
                        if !matches {
                            return Err(format!(
                                "method '{}::{}' does not match the signature declared by interface '{}'",
                                class.name, required.name, interface.name
                            ));
                        }
                        Self::check_keeps_optional(class, method, &required.parameters)?;
                        if method.visibility != Visibility::Public {
                            return Err(format!(
                                "method '{}::{}' must be public to implement interface '{}'",
                                class.name, required.name, interface.name
                            ));
                        }
                        if method.is_abstract && !class.is_abstract {
                            return Err(format!(
                                "class '{}' must implement method '{}' from interface '{}'",
                                class.name, required.name, interface.name
                            ));
                        }
                    }
                    None if !class.is_abstract => {
                        return Err(format!(
                            "class '{}' must implement method '{}' from interface '{}'",
                            class.name, required.name, interface.name
                        ));
                    }
                    None => {}
                }
            }
        }
        Ok(())
    }

    fn emit_expression(
        &mut self,
        expression: &Expr,
        variables: &HashMap<String, Value>,
        body: &mut String,
    ) -> Result<Value, String> {
        match expression {
            Expr::One => Err("internal error: '++' outside an increment".to_string()),
            Expr::InstanceOf(_, _, Some(name)) => Err(format!(
                "a name after instanceof ('{name}') can only be given in an if condition, on its own or joined by &&; elsewhere, check with instanceof and no name"
            )),
            Expr::InstanceOf(operand, target, None) => {
                let value = self.emit_borrowed(operand, None, variables, body)?;
                let target = *target;
                let target_name = self.classes[target as usize].name.clone();
                let ValueType::Class(static_class) = value.value_type else {
                    return Err(format!("instanceof needs an object, found {}", self.type_name(value.value_type)));
                };
                // Two classes, neither extending the other: never true.
                let (static_def, target_def) = (&self.classes[static_class as usize], &self.classes[target as usize]);
                if !static_def.is_interface
                    && !target_def.is_interface
                    && static_class != target
                    && !self.is_subclass(static_class, target)
                    && !self.is_subclass(target, static_class)
                {
                    return Err(format!(
                        "an object of class {} can never be a {target_name}: neither class extends the other",
                        self.classes[static_class as usize].name
                    ));
                }
                if !self.instanceof_tables.contains(&target) {
                    self.instanceof_tables.push(target);
                }
                // The class id is the object's first slot; for null, a stand-in
                // whose id has a 0 in every table, so there's no branch.
                let (is_null, object, class_id, entry, byte, result) = (
                    self.new_register(),
                    self.new_register(),
                    self.new_register(),
                    self.new_register(),
                    self.new_register(),
                    self.new_register(),
                );
                body.push_str(&format!(
                    "  {is_null} = icmp eq ptr {operand}, null\n  {object} = select i1 {is_null}, ptr @qiq_no_object, ptr {operand}\n  {class_id} = load i64, ptr {object}, align 8\n  {entry} = getelementptr inbounds [{count} x i8], ptr @qiq_is.{symbol}, i64 0, i64 {class_id}\n  {byte} = load i8, ptr {entry}, align 1\n  {result} = icmp ne i8 {byte}, 0\n",
                    operand = value.operand,
                    count = self.classes.len() + 1,
                    symbol = self.classes[target as usize].symbol,
                ));
                Ok(Value { value_type: ValueType::Bool, operand: result, slot: None })
            }
            Expr::Nullsafe(..) => self.emit_nullsafe(expression, None, variables, body),
            Expr::Downcast(operand, target) => {
                let value = self.emit_expression(operand, variables, body)?;
                Ok(Value { value_type: ValueType::Class(*target), operand: value.operand, slot: value.slot })
            }
            Expr::Number(value) => Ok(Value {
                value_type: ValueType::Int,
                operand: value.to_string(),
                slot: None,
            }),
            Expr::Decimal(value) => Ok(Value {
                value_type: ValueType::Double,
                operand: format!("{value:.17e}"),
                slot: None,
            }),
            Expr::Null => Ok(Value {
                value_type: ValueType::Null,
                operand: "null".to_string(),
                slot: None,
            }),
            Expr::Boolean(value) => Ok(Value {
                value_type: ValueType::Bool,
                operand: if *value { "true" } else { "false" }.to_string(),
                slot: None,
            }),
            Expr::Text(value) => Ok(Value {
                value_type: ValueType::String,
                operand: self.string_literal(value.as_bytes()),
                slot: None,
            }),
            Expr::Character(value) => {
                let mut bytes = [0; 4];
                let encoded = value.encode_utf8(&mut bytes).as_bytes();
                Ok(Value {
                    value_type: ValueType::Char,
                    operand: self.string_literal(encoded),
                    slot: None,
                })
            }
            Expr::Index(array_expression, index_expression) => {
                let borrow = std::mem::take(&mut self.borrow_read);
                let array = self.emit_expression(array_expression, variables, body)?;
                // The key is used by qiq_map_get as soon as it's evaluated.
                let index = self.emit_borrowed(index_expression, None, variables, body)?;
                if let Some(element_type) = self.element_of(array.value_type) {
                    if index.value_type != ValueType::Int {
                        return Err("array index must be an int".to_string());
                    }
                    let address = self.emit_element_address(&array, element_type, &index.operand, body);
                    let loaded = self.new_register();
                    body.push_str(&format!(
                        "  {loaded} = load {}, ptr {address}, align {}\n",
                        element_type.llvm(),
                        element_type.alignment()
                    ));
                    let value = Value {
                        value_type: element_type,
                        operand: loaded,
                        slot: None,
                    };
                    Ok(if borrow { value } else { self.retained(value, body) })
                } else if let ValueType::Map(type_id) = array.value_type {
                    let signature = *self
                        .map_types
                        .get(type_id as usize)
                        .ok_or_else(|| "invalid map type descriptor".to_string())?;
                    if index.value_type != signature.key_type {
                        return Err(format!(
                            "map key requires {}, found {}",
                            self.type_name(signature.key_type),
                            self.type_name(index.value_type)
                        ));
                    }
                    let key_bits = self.encode_payload(&index, body)?;
                    let loaded_bits = self.new_register();
                    body.push_str(&format!(
                        "  {loaded_bits} = call i64 @qiq_map_get(ptr {}, i64 {key_bits})\n",
                        array.operand
                    ));
                    self.emit_exception_check(body);
                    let value = self.decode_payload(signature.value_type, &loaded_bits, body)?;
                    Ok(if borrow { value } else { self.retained(value, body) })
                } else {
                    Err("indexed value is not an array or map".to_string())
                }
            }
            Expr::Member(object_expression, property_name) => {
                let borrow = std::mem::take(&mut self.borrow_read);
                let object = self.emit_expression(object_expression, variables, body)?;
                let class_id = match object.value_type {
                    ValueType::Class(class_id) => class_id,
                    _ => return Err("property access requires a class instance".to_string()),
                };
                self.emit_null_check(&object, object_expression, &format!("cannot read property '{property_name}' of null"), body);
                let (property_index, property_type, visibility, owner_class) =
                    self.find_property(class_id, property_name)?;
                self.check_visibility(visibility, owner_class, property_name)?;
                let address = self.new_register();
                body.push_str(&format!(
                    "  {address} = getelementptr inbounds i64, ptr {}, i64 {property_index}\n",
                    object.operand
                ));
                let encoded = self.new_register();
                body.push_str(&format!("  {encoded} = load i64, ptr {address}, align 8\n"));
                let value = self.decode_payload(property_type, &encoded, body)?;
                Ok(if borrow { value } else { self.retained(value, body) })
            }
            Expr::MethodCall(object_expression, method_name, arguments) => {
                // Only this call, not the ones in its receiver or arguments.
                let expectation = self.call_expectation.take();
                let object = self.emit_expression(object_expression, variables, body)?;
                let class_id = match object.value_type {
                    ValueType::Class(class_id) => class_id,
                    _ => return Err("method call requires a class instance".to_string()),
                };
                self.emit_null_check(&object, object_expression, &format!("cannot call method '{method_name}' on null"), body);
                // Rows read into the class the result is used as.
                if (matches!(method_name.as_str(), "queryAs" | "queryOneAs") && self.stdlib_class("Database") == Some(class_id))
                    || (matches!(method_name.as_str(), "nextAs" | "fetchAs") && self.stdlib_class("Cursor") == Some(class_id))
                {
                    return self.emit_query_as(object, method_name, arguments, expectation, variables, body);
                }
                let found = self.find_method(class_id, method_name);
                if found.is_err() {
                    // `object->callback(x)` with a property holding a function.
                    if let Ok((index, property_type @ ValueType::Function(_), visibility, owner)) = self.find_property(class_id, method_name) {
                        self.check_visibility(visibility, owner, method_name)?;
                        let at = self.new_register();
                        let bits = self.new_register();
                        body.push_str(&format!(
                            "  {at} = getelementptr inbounds i64, ptr {}, i64 {index}\n  {bits} = load i64, ptr {at}, align 8\n",
                            object.operand
                        ));
                        let callee = self.decode_payload(property_type, &bits, body)?;
                        let callee = self.retained(callee, body);
                        return self.emit_value_call(callee, arguments, variables, body);
                    }
                }
                let (parameters, return_type, visibility, method_owner, is_abstract) = found?;
                self.check_visibility(visibility, method_owner, method_name)?;
                self.use_class(class_id);
                self.use_class(method_owner);
                let owner_class = &self.classes[method_owner as usize];
                let returns_static = owner_class
                    .methods
                    .iter()
                    .any(|method| method.name == *method_name && method.returns_static);
                if owner_class.methods.iter().any(|method| method.name == *method_name && method.is_static) {
                    return Err(format!(
                        "method '{}::{method_name}' is static; call it with {}::{method_name}(...)",
                        owner_class.name, owner_class.name
                    ));
                }
                let binds_values = owner_class
                    .methods
                    .iter()
                    .any(|method| method.name == *method_name && method.binds_values);
                // Values after the parameters of a `...` method are handed to
                // the runtime separately (see Function::binds_values).
                let (arguments, values) = if binds_values && arguments.len() > parameters.len() {
                    arguments.split_at(parameters.len())
                } else {
                    (&arguments[..], &[][..])
                };
                let callee_name = format!("method '{method_name}'");
                let resolved = Self::resolve_arguments(&parameters, arguments, &callee_name)?;
                // An abstract method (declared by an interface or abstract class),
                // or one a subclass overrides, is looked up at run time in a
                // dispatch table indexed by the class id in the object's header,
                // so the object's own implementation runs. Others are called directly.
                let dynamic = is_abstract
                    || (visibility != Visibility::Private && self.is_overridden(method_owner, method_name));
                // A dispatched call that omits optional arguments goes through
                // adapters, so the running class's defaults apply.
                let mut supplied: Vec<usize> = resolved.iter().map(|(index, _)| *index).collect();
                supplied.sort_unstable();
                let adapter = (dynamic && supplied.len() < parameters.len()).then_some(supplied);
                let callee = if dynamic {
                    match &adapter {
                        Some(supplied) => {
                            let key = (method_owner, method_name.clone(), supplied.clone());
                            if !self.adapter_tables.contains(&key) {
                                self.adapter_tables.push(key);
                            }
                        }
                        None => {
                            let key = (method_owner, method_name.clone());
                            if !self.dispatch_tables.contains(&key) {
                                self.dispatch_tables.push(key);
                            }
                        }
                    }
                    let table = self.dispatch_table_name(method_owner, method_name, adapter.as_deref());
                    let class_tag = self.new_register();
                    let slot = self.new_register();
                    let function = self.new_register();
                    body.push_str(&format!(
                        "  {class_tag} = load i64, ptr {}, align 8\n  {slot} = getelementptr inbounds ptr, ptr @{table}, i64 {class_tag}\n  {function} = load ptr, ptr {slot}, align 8\n",
                        object.operand
                    ));
                    function
                } else {
                    format!("@qiq_{}_{method_name}", self.classes[method_owner as usize].symbol)
                };
                let mut operands = vec![format!("ptr {}", object.operand)];
                operands.extend(self.emit_arguments(
                    &parameters,
                    &resolved,
                    adapter.is_none(),
                    &callee_name,
                    &format!("argument type mismatch calling '{method_name}'"),
                    variables,
                    body,
                )?);
                if binds_values {
                    self.emit_bind_values(method_name, values, variables, body)?;
                }
                let operands = operands.join(", ");
                let result = if return_type == ValueType::Void {
                    body.push_str(&format!("  call void {callee}({operands})\n"));
                    "null".to_string()
                } else {
                    let result = self.new_register();
                    body.push_str(&format!(
                        "  {result} = call {} {callee}({operands})\n",
                        return_type.llvm()
                    ));
                    result
                };
                self.emit_exception_check(body);
                Ok(self.own(Value {
                    value_type: if returns_static {
                        ValueType::Class(class_id)
                    } else {
                        Self::call_result_type(return_type)
                    },
                    operand: result,
                    slot: None,
                }))
            }
            Expr::ClassName(target) => match *target {
                StaticTarget::Class { class_id, .. } => {
                    let name = self.classes[class_id as usize].name.clone();
                    Ok(Value { value_type: ValueType::String, operand: self.string_literal(name.as_bytes()), slot: None })
                }
                StaticTarget::Parent { parent, .. } => {
                    let name = self.classes[parent as usize].name.clone();
                    Ok(Value { value_type: ValueType::String, operand: self.string_literal(name.as_bytes()), slot: None })
                }
                StaticTarget::Late { .. } => {
                    let class_id = self.emit_called_class(body)?;
                    Ok(self.emit_class_name_of(&class_id, body))
                }
            },
            Expr::ClassOf(object_expression) => {
                if let Expr::Variable(name) = object_expression.as_ref() {
                    if !variables.contains_key(name) {
                        return Err(format!("'{name}' is neither a declared or imported class nor a variable{}", stdlib_hint(name)));
                    }
                }
                let object = self.emit_expression(object_expression, variables, body)?;
                if !matches!(object.value_type, ValueType::Class(_)) {
                    return Err(format!("::class needs an object or a class name, found {}", self.type_name(object.value_type)));
                }
                self.emit_null_check(&object, object_expression, "cannot read ::class of null", body);
                let class_id = self.new_register();
                body.push_str(&format!("  {class_id} = load i64, ptr {}, align 8\n", object.operand));
                Ok(self.emit_class_name_of(&class_id, body))
            }
            Expr::StaticCall(target, method_name, arguments) => {
                let target = *target;
                self.note_database_open(target, method_name, arguments);
                let class_id = Self::static_target_class(target);
                let class_name = self.classes[class_id as usize].name.clone();
                let Some(owner) = self.find_declaring_class(class_id, method_name) else {
                    // parent::__construct() with no parent constructor does nothing.
                    if matches!(target, StaticTarget::Parent { .. }) && method_name == "__construct" {
                        if self.current_method.as_deref() != Some("__construct") {
                            return Err("parent::__construct can only be called from a constructor".to_string());
                        }
                        if !arguments.is_empty() {
                            return Err(format!("parent class '{class_name}' has no constructor"));
                        }
                        return Ok(Value {
                            value_type: ValueType::Null,
                            operand: "null".to_string(),
                            slot: None,
                        });
                    }
                    return Err(format!("class '{class_name}' has no method '{method_name}'"));
                };
                self.use_class(owner);
                let owner_name = self.classes[owner as usize].name.clone();
                let method = self.classes[owner as usize]
                    .methods
                    .iter()
                    .find(|method| method.name == *method_name)
                    .cloned()
                    .expect("find_declaring_class found it");
                self.check_visibility(method.visibility, owner, method_name)?;
                let callee_name = format!("method '{owner_name}::{method_name}'");
                let mismatch = format!("argument type mismatch calling '{owner_name}::{method_name}'");
                let mut operands = Vec::new();
                let mut callee = format!("@qiq_{}_{method_name}", self.classes[owner as usize].symbol);
                if method.is_static {
                    // Static methods receive the class the call was made on.
                    let called = match target {
                        StaticTarget::Class { class_id, forwards: false } => class_id.to_string(),
                        _ => self.emit_called_class(body)?,
                    };
                    if let StaticTarget::Late { enclosing } = target {
                        let key = (enclosing, method_name.clone());
                        if !self.late_method_tables.contains(&key) {
                            self.late_method_tables.push(key);
                        }
                        let slot = self.new_register();
                        let function = self.new_register();
                        body.push_str(&format!(
                            "  {slot} = getelementptr inbounds ptr, ptr @qiq_static_dispatch.{}.{method_name}, i64 {called}\n  {function} = load ptr, ptr {slot}, align 8\n",
                            self.classes[enclosing as usize].symbol
                        ));
                        callee = function;
                    }
                    operands.push(format!("i64 {called}"));
                } else {
                    // An instance method: only self:: and parent:: may call one,
                    // on `this`, without dispatch.
                    match target {
                        StaticTarget::Class { forwards: false, .. } => {
                            return Err(format!(
                                "method '{owner_name}::{method_name}' is not static; call it on an object with ->"
                            ));
                        }
                        StaticTarget::Late { .. } => {
                            return Err(format!(
                                "static:: can only call static methods; '{owner_name}::{method_name}' is an instance method"
                            ));
                        }
                        _ => {}
                    }
                    if self.current_method_is_static {
                        return Err(format!(
                            "'this' is not available in static method '{}', so it can't call instance method '{owner_name}::{method_name}'",
                            self.current_method.as_deref().unwrap_or("")
                        ));
                    }
                    if method.name == "__construct" {
                        if !matches!(target, StaticTarget::Parent { .. }) || self.current_method.as_deref() != Some("__construct") {
                            return Err("parent::__construct can only be called from a constructor".to_string());
                        }
                    }
                    if method.is_abstract {
                        return Err(format!("cannot call abstract method '{owner_name}::{method_name}'"));
                    }
                    operands.push("ptr %this".to_string());
                }
                let resolved = Self::resolve_arguments(&method.parameters, arguments, &callee_name)?;
                operands.extend(self.emit_arguments(
                    &method.parameters,
                    &resolved,
                    true,
                    &callee_name,
                    &mismatch,
                    variables,
                    body,
                )?);
                let operands = operands.join(", ");
                let result = if method.return_type == ValueType::Void {
                    body.push_str(&format!("  call void {callee}({operands})\n"));
                    "null".to_string()
                } else {
                    let result = self.new_register();
                    body.push_str(&format!(
                        "  {result} = call {} {callee}({operands})\n",
                        method.return_type.llvm()
                    ));
                    result
                };
                self.emit_exception_check(body);
                // `: static` returns the class the call was made on, which is at
                // least the named class (or, for self/parent/static, the enclosing one).
                let value_type = if method.returns_static {
                    ValueType::Class(match target {
                        StaticTarget::Class { class_id, .. } => class_id,
                        StaticTarget::Parent { enclosing, .. } | StaticTarget::Late { enclosing } => enclosing,
                    })
                } else {
                    Self::call_result_type(method.return_type)
                };
                Ok(self.own(Value {
                    value_type,
                    operand: result,
                    slot: None,
                }))
            }
            Expr::StaticProperty(target, name) => {
                if let Some((owner, constant)) = self.find_constant(Self::static_target_class(*target), name) {
                    self.check_visibility(constant.visibility, owner, name)?;
                    let (literal, value_type) = self.constant_value(owner, &constant, 0)?;
                    return self.emit_expecting(&literal, Some(value_type), variables, body);
                }
                let (address, property) = self.emit_static_property_address(*target, name, body)?;
                let loaded = self.new_register();
                body.push_str(&format!(
                    "  {loaded} = load {}, ptr {address}, align 8\n",
                    property.value_type.llvm()
                ));
                let value = Value {
                    value_type: property.value_type,
                    operand: loaded,
                    slot: None,
                };
                Ok(self.retained(value, body))
            }
            Expr::NewStatic(enclosing, arguments) => {
                let enclosing = *enclosing;
                let enclosing_name = self.classes[enclosing as usize].name.clone();
                let constructor_of = |this: &Self, class_id: u32| -> Vec<Parameter> {
                    this.find_implementation(class_id, "__construct")
                        .and_then(|owner| {
                            this.classes[owner as usize]
                                .methods
                                .iter()
                                .find(|method| method.name == "__construct")
                                .map(|method| method.parameters.clone())
                        })
                        .unwrap_or_default()
                };
                let parameters = constructor_of(self, enclosing);
                // Every class the call might be made on must accept these arguments.
                for subclass in self.subtypes_of(enclosing) {
                    let theirs = constructor_of(self, subclass);
                    let same = theirs.len() == parameters.len()
                        && theirs.iter().zip(&parameters).all(|(a, b)| a.value_type == b.value_type);
                    if !same {
                        return Err(format!(
                            "new static in '{enclosing_name}' needs every subclass to take the same constructor arguments, but '{}' changes them",
                            self.classes[subclass as usize].name
                        ));
                    }
                }
                let callee_name = format!("constructor '{enclosing_name}'");
                let resolved = Self::resolve_arguments(&parameters, arguments, &callee_name)?;
                let operands = self.emit_arguments(
                    &parameters,
                    &resolved,
                    true,
                    &callee_name,
                    "constructor argument mismatch",
                    variables,
                    body,
                )?;
                if !self.new_static_tables.contains(&enclosing) {
                    self.new_static_tables.push(enclosing);
                }
                for subclass in self.subtypes_of(enclosing) {
                    self.use_class(subclass);
                }
                let called = self.emit_called_class(body)?;
                let slot = self.new_register();
                let factory = self.new_register();
                let object = self.new_register();
                body.push_str(&format!(
                    "  {slot} = getelementptr inbounds ptr, ptr @qiq_new_static.{}, i64 {called}\n  {factory} = load ptr, ptr {slot}, align 8\n  {object} = call ptr {factory}({})\n",
                    self.classes[enclosing as usize].symbol,
                    operands.join(", ")
                ));
                self.emit_exception_check(body);
                Ok(self.own(Value {
                    value_type: ValueType::Class(enclosing),
                    operand: object,
                    slot: None,
                }))
            }
            Expr::NewObject(class_id, arguments) => {
                let class = self.classes[*class_id as usize].name.clone();
                if self.classes[*class_id as usize].is_interface {
                    return Err(format!("cannot instantiate interface '{class}'"));
                }
                if self.classes[*class_id as usize].is_abstract {
                    return Err(format!("cannot instantiate abstract class '{class}'"));
                }
                for method in &self.classes[*class_id as usize].methods {
                    if method.is_abstract {
                        return Err(format!(
                            "class '{class}' must implement abstract method '{}' before instantiation",
                            method.name
                        ));
                    }
                }
                self.use_class(*class_id);
                let field_count = OBJECT_HEADER_FIELDS + self.class_field_count(*class_id)?;
                let object = self.new_register();
                body.push_str(&format!(
                    "  {object} = call nonnull ptr @qiq_object_new(i64 {field_count}, i64 {class_id})\n"
                ));
                if self.has_property_defaults(*class_id) {
                    body.push_str(&format!("  call void @qiq_defaults.{}(ptr {object})\n", self.classes[*class_id as usize].symbol));
                }
                self.own(Value {
                    value_type: ValueType::Class(*class_id),
                    operand: object.clone(),
                    slot: None,
                });
                // Constructors are inherited: use the nearest class that declares one.
                let constructor_owner = self.find_implementation(*class_id, "__construct");
                if let Some(owner) = constructor_owner {
                    let constructor = self.classes[owner as usize]
                        .methods
                        .iter()
                        .find(|method| method.name == "__construct")
                        .expect("find_implementation found a constructor");
                    self.check_visibility(constructor.visibility, owner, &class)?;
                    if constructor.return_type != ValueType::Void {
                        return Err(format!("constructor '{class}' must return void"));
                    }
                    let constructor_parameters = constructor.parameters.clone();
                    let callee_name = format!("constructor '{class}'");
                    let resolved = Self::resolve_arguments(&constructor_parameters, arguments, &callee_name)?;
                    let mut operands = vec![format!("ptr {object}")];
                    operands.extend(self.emit_arguments(
                        &constructor_parameters,
                        &resolved,
                        true,
                        &callee_name,
                        "constructor argument mismatch",
                        variables,
                        body,
                    )?);
                    body.push_str(&format!(
                        "  call void @qiq_{}___construct({})\n",
                        self.classes[owner as usize].symbol,
                        operands.join(", ")
                    ));
                    self.emit_exception_check(body);
                } else if !arguments.is_empty() {
                    return Err(format!("class '{class}' has no matching constructor"));
                }
                Ok(Value {
                    value_type: ValueType::Class(*class_id),
                    operand: object,
                    slot: None,
                })
            }
            Expr::Variable(name) => {
                if name == "this" && self.current_method_is_static && !variables.contains_key(name) {
                    return Err(format!(
                        "'this' is not available in static method '{}'",
                        self.current_method.as_deref().unwrap_or("")
                    ));
                }
                let Some(variable) = variables.get(name) else {
                    // A named function used as a value.
                    if self.module_functions[self.current_module].contains_key(name) {
                        return self.emit_function_reference(name, body);
                    }
                    if CONVERSION_FUNCTIONS.contains(&name.as_str()) {
                        return Err(format!(
                            "built-in function '{name}' can't be used as a value; wrap it in an anonymous function, such as fn(string s) => {name}(s)"
                        ));
                    }
                    return Err(format!("variable '{name}' is not declared"));
                };
                if let Some((cell, _)) = variable.slot.clone().and_then(|slot| self.cell_of(&slot, body)) {
                    // Read through the cell. A closure can change the variable
                    // during this statement, so the value is retained.
                    let value_type = variable.value_type;
                    let at = self.new_register();
                    let bits = self.new_register();
                    body.push_str(&format!(
                        "  {at} = getelementptr inbounds i64, ptr {cell}, i64 {OBJECT_HEADER_FIELDS}\n  {bits} = load i64, ptr {at}, align 8\n"
                    ));
                    let value = self.decode_payload(value_type, &bits, body)?;
                    return Ok(self.retained(value, body));
                }
                match &variable.slot {
                    Some(slot) => {
                        let value_type = variable.value_type;
                        let slot = slot.clone();
                        let loaded = self.new_register();
                        body.push_str(&format!(
                            "  {loaded} = load {}, ptr {slot}, align 8\n",
                            value_type.llvm()
                        ));
                        Ok(Value {
                            value_type,
                            operand: loaded,
                            slot: None,
                        })
                    }
                    None => Ok(variable.clone()),
                }
            }
            Expr::Call(name, arguments) if CONVERSION_FUNCTIONS.contains(&name.as_str()) => {
                self.emit_conversion(name, arguments, variables, body)
            }
            // A variable holding a function value: call it.
            Expr::Call(name, arguments)
                if variables.get(name).is_some_and(|variable| matches!(variable.value_type, ValueType::Function(_))) =>
            {
                let callee = self.emit_expression(&Expr::Variable(name.clone()), variables, body)?;
                self.emit_value_call(callee, arguments, variables, body)
            }
            Expr::Call(name, arguments) => {
                if !self.module_functions[self.current_module].contains_key(name) {
                    if let Some(variable) = variables.get(name) {
                        return Err(format!("'{name}' is a variable of type {}, not a function", self.type_name(variable.value_type)));
                    }
                }
                let symbol = self.module_functions[self.current_module].get(name).cloned().ok_or_else(|| {
                    if name == "strlen" {
                        "function 'strlen' is not declared; use len(text), which counts characters".to_string()
                    } else {
                        format!("function '{name}' is not declared{}", stdlib_hint(name))
                    }
                })?;
                // abs, min, max, clamp, and number_format from std/math work
                // on ints and doubles: compiled here for the types given.
                // (number_format is an extern, named for its C function; only
                // the standard library can declare externs.)
                let math = symbol.strip_prefix("qiq.stdlib.math.").or((symbol == "qiq_stdlib_number_format").then_some("number_format"));
                if let Some(function) = math {
                    if let Some(value) = self.emit_math_generic(function, name, arguments, variables, body)? {
                        return Ok(value);
                    }
                }
                if !self.called_functions.contains(&symbol) {
                    self.called_functions.push(symbol.clone());
                }
                let signature = self
                    .signatures
                    .get(&symbol)
                    .ok_or_else(|| format!("function '{name}' is not declared"))?;
                let parameters = signature.parameters.clone();
                let return_type = signature.return_type;
                let callee_name = format!("function '{name}'");
                let resolved = Self::resolve_arguments(&parameters, arguments, &callee_name)?;
                if let Some(function) = symbol.strip_prefix("qiq.stdlib.regex.") {
                    check_regex_literals(function, &resolved)?;
                }
                let operands = self.emit_arguments(
                    &parameters,
                    &resolved,
                    true,
                    &callee_name,
                    &format!("argument type mismatch calling '{name}'"),
                    variables,
                    body,
                )?;
                let arguments = operands.join(", ");
                let operand = if return_type == ValueType::Void {
                    body.push_str(&format!("  call void @{symbol}({arguments})\n"));
                    "null".to_string()
                } else {
                    let register = self.new_register();
                    body.push_str(&format!(
                        "  {register} = call {} @{symbol}({arguments})\n",
                        return_type.llvm()
                    ));
                    register
                };
                self.emit_exception_check(body);
                Ok(self.own(Value {
                    value_type: Self::call_result_type(return_type),
                    operand,
                    slot: None,
                }))
            }
            Expr::Binary(operator, left, right) => {
                // Operands are used by one instruction or runtime call (such
                // as qiq_string_concat) right after the right one is
                // evaluated, so it can always be borrowed, and the left one
                // when evaluating the right one can't change anything.
                let left = if is_pure(right) {
                    self.emit_borrowed(left, None, variables, body)?
                } else {
                    self.emit_expression(left, variables, body)?
                };
                let right_expression = right;
                let right = if matches!(**right, Expr::One) {
                    // `x++` and `x--`: 1 in x's type.
                    match left.value_type {
                        ValueType::Int => Value { value_type: ValueType::Int, operand: "1".to_string(), slot: None },
                        ValueType::Double => Value { value_type: ValueType::Double, operand: format!("{:.17e}", 1.0f64), slot: None },
                        other => return Err(format!("'++' and '--' work on int and double values, found {}", self.type_name(other))),
                    }
                } else {
                    self.emit_borrowed(right, None, variables, body)?
                };
                let _ = right_expression;
                let is_text = |value: &Value| matches!(value.value_type, ValueType::String | ValueType::Char);
                let (left, right) = if *operator == BinaryOp::Add && (is_text(&left) || is_text(&right)) {
                    let left = match self.emit_to_string_call(&left, variables, body)? {
                        Some(text) if is_text(&right) => text,
                        _ => left,
                    };
                    let right = match self.emit_to_string_call(&right, variables, body)? {
                        Some(text) if is_text(&left) => text,
                        _ => right,
                    };
                    (left, right)
                } else {
                    (left, right)
                };
                if *operator == BinaryOp::Add && is_text(&left) && is_text(&right) {
                    // Concatenation: any mix of strings and chars produces a string.
                    let register = self.new_register();
                    body.push_str(&format!(
                        "  {register} = call ptr @qiq_string_concat(ptr {}, ptr {})\n",
                        left.operand, right.operand
                    ));
                    self.emit_exception_check(body);
                    return Ok(self.own(Value {
                        value_type: ValueType::String,
                        operand: register,
                        slot: None,
                    }));
                }
                if operator.is_bitwise()
                    && (left.value_type != ValueType::Int || right.value_type != ValueType::Int)
                {
                    return Err(format!(
                        "'{}' requires two ints; found {} and {}",
                        operator.symbol(),
                        self.type_name(left.value_type),
                        self.type_name(right.value_type)
                    ));
                }
                if left.value_type != right.value_type
                    || !matches!(left.value_type, ValueType::Int | ValueType::Double)
                {
                    let also_text = if *operator == BinaryOp::Add {
                        ", or strings and chars to concatenate"
                    } else {
                        ""
                    };
                    let byte_hint = if left.value_type == ValueType::Byte || right.value_type == ValueType::Byte {
                        " (convert bytes with to_int)"
                    } else {
                        ""
                    };
                    return Err(format!(
                        "'{}' requires two ints or two doubles{also_text}; found {} and {}{byte_hint}",
                        operator.symbol(),
                        self.type_name(left.value_type),
                        self.type_name(right.value_type)
                    ));
                }
                let register = self.new_register();
                let (a, b) = (&left.operand, &right.operand);
                // The checked int operations, inline where they can't fail
                // (see emit_fast_path).
                if left.value_type == ValueType::Int
                    && matches!(operator, BinaryOp::Divide | BinaryOp::Modulo | BinaryOp::Power | BinaryOp::ShiftLeft | BinaryOp::ShiftRight)
                {
                    let constant = b.parse::<i64>().ok();
                    let (fast, slow, safe, condition) = match operator {
                        BinaryOp::Divide | BinaryOp::Modulo => {
                            let (instruction, function) = if *operator == BinaryOp::Divide { ("sdiv", "divide") } else { ("srem", "modulo") };
                            (
                                format!("{instruction} i64 {a}, {b}"),
                                format!("call i64 @qiq_int_{function}(i64 {a}, i64 {b})"),
                                // 0 is an error; INT64_MIN / -1 overflows the instruction.
                                constant.is_some_and(|value| value != 0 && value != -1),
                                format!("  %chk.z# = icmp eq i64 {b}, 0\n  %chk.m# = icmp eq i64 {b}, -1\n  %chk.bad# = or i1 %chk.z#, %chk.m#\n  %chk.ok# = xor i1 %chk.bad#, true\n"),
                            )
                        }
                        BinaryOp::ShiftLeft | BinaryOp::ShiftRight => {
                            let (instruction, function) = if *operator == BinaryOp::ShiftLeft { ("shl", "left") } else { ("ashr", "right") };
                            (
                                format!("{instruction} i64 {a}, {b}"),
                                format!("call i64 @qiq_int_shift_{function}(i64 {a}, i64 {b})"),
                                // Negative amounts are errors; 64 or more is
                                // undefined for the instruction.
                                constant.is_some_and(|value| (0..64).contains(&value)),
                                format!("  %chk.ok# = icmp ult i64 {b}, 64\n"),
                            )
                        }
                        _ => (
                            format!("call i64 @qiq_int_power_unchecked(i64 {a}, i64 {b})"),
                            format!("call i64 @qiq_int_power(i64 {a}, i64 {b})"),
                            // A negative exponent is an error.
                            constant.is_some_and(|value| value >= 0),
                            format!("  %chk.ok# = icmp sge i64 {b}, 0\n"),
                        ),
                    };
                    if safe {
                        body.push_str(&format!("  {register} = {fast}\n"));
                    } else {
                        self.emit_fast_path(&register, "i64", &condition, &fast, &slow, body);
                    }
                    return Ok(Value { value_type: ValueType::Int, operand: register, slot: None });
                }
                let instruction = if left.value_type == ValueType::Int {
                    match operator {
                        BinaryOp::Add => format!("add i64 {a}, {b}"),
                        BinaryOp::Subtract => format!("sub i64 {a}, {b}"),
                        BinaryOp::Multiply => format!("mul i64 {a}, {b}"),
                        // Checked in the runtime: division by zero is an error.
                        BinaryOp::Divide => format!("call i64 @qiq_int_divide(i64 {a}, i64 {b})"),
                        BinaryOp::Modulo => format!("call i64 @qiq_int_modulo(i64 {a}, i64 {b})"),
                        BinaryOp::Power => format!("call i64 @qiq_int_power(i64 {a}, i64 {b})"),
                        BinaryOp::BitAnd => format!("and i64 {a}, {b}"),
                        BinaryOp::BitOr => format!("or i64 {a}, {b}"),
                        BinaryOp::BitXor => format!("xor i64 {a}, {b}"),
                        // Checked in the runtime: LLVM shifts by >= 32 are undefined.
                        BinaryOp::ShiftLeft => format!("call i64 @qiq_int_shift_left(i64 {a}, i64 {b})"),
                        BinaryOp::ShiftRight => format!("call i64 @qiq_int_shift_right(i64 {a}, i64 {b})"),
                    }
                } else {
                    match operator {
                        BinaryOp::Add => format!("fadd double {a}, {b}"),
                        BinaryOp::Subtract => format!("fsub double {a}, {b}"),
                        BinaryOp::Multiply => format!("fmul double {a}, {b}"),
                        BinaryOp::Divide => format!("fdiv double {a}, {b}"),
                        BinaryOp::Modulo => format!("frem double {a}, {b}"),
                        BinaryOp::Power => format!("call double @llvm.pow.f64(double {a}, double {b})"),
                        BinaryOp::BitAnd
                        | BinaryOp::BitOr
                        | BinaryOp::BitXor
                        | BinaryOp::ShiftLeft
                        | BinaryOp::ShiftRight => unreachable!("bitwise operands are checked to be ints"),
                    }
                };
                body.push_str(&format!("  {register} = {instruction}\n"));
                if left.value_type == ValueType::Int
                    && matches!(
                        operator,
                        BinaryOp::Divide
                            | BinaryOp::Modulo
                            | BinaryOp::Power
                            | BinaryOp::ShiftLeft
                            | BinaryOp::ShiftRight
                    )
                {
                    self.emit_exception_check(body);
                }
                Ok(Value {
                    value_type: left.value_type,
                    operand: register,
                    slot: None,
                })
            }
            Expr::ArrayLiteral(elements) => self.emit_array_literal(elements, None, variables, body),
            Expr::NewArray(element_type, size_expression) => {
                if matches!(element_type, ValueType::Void | ValueType::Null) {
                    return Err(format!("arrays of {} are not supported", self.type_name(*element_type)));
                }
                let size = self.emit_expression(size_expression, variables, body)?;
                if size.value_type != ValueType::Int {
                    return Err(format!("array size must be an int, found {}", self.type_name(size.value_type)));
                }
                let array = self.new_register();
                body.push_str(&format!(
                    "  {array} = call ptr @qiq_array_new(i64 {}, i64 {}, i32 {})\n",
                    size.operand,
                    element_type.size_bytes(),
                    self.contents_flags(*element_type)
                ));
                // A negative size throws InvalidArgumentException.
                self.emit_exception_check(body);
                let array_type = self.array_of(*element_type);
                Ok(self.own(Value {
                    value_type: array_type,
                    operand: array,
                    slot: None,
                }))
            }
            Expr::MapLiteral(entries) => self.emit_map_literal(entries, None, variables, body),
            Expr::NamedArgument(name, _) => {
                Err(format!("named argument '{name}' is only valid in a call"))
            }
            Expr::Compare(operator, left, right) => {
                // An int literal on one side takes the other side's type when
                // that is a byte, so `data[0] == 137` compares bytes.
                let (left, right) = if matches!(**left, Expr::Number(_)) && !matches!(**right, Expr::Number(_)) {
                    let right = self.emit_expression(right, variables, body)?;
                    let expected = (right.value_type == ValueType::Byte).then_some(ValueType::Byte);
                    (self.emit_expecting(left, expected, variables, body)?, right)
                } else {
                    // Borrowed as for Binary: emit_comparison uses both at once.
                    let left = if is_pure(right) {
                        self.emit_borrowed(left, None, variables, body)?
                    } else {
                        self.emit_expression(left, variables, body)?
                    };
                    // A literal `[]` or `{}` takes the other side's type, as
                    // in `list == []`; `null` stays a null test.
                    let literal = matches!(**right, Expr::ArrayLiteral(_) | Expr::MapLiteral(_));
                    let expected = match left.value_type {
                        ValueType::Byte => Some(ValueType::Byte),
                        ValueType::Array(_) | ValueType::Map(_) if literal => Some(left.value_type),
                        _ => None,
                    };
                    let right = self.emit_borrowed(right, expected, variables, body)?;
                    (left, right)
                };
                self.emit_comparison(left, *operator, right, body)
            }
            Expr::Coalesce(left, right) => self.emit_coalesce(left, right, variables, body),
            Expr::Closure(closure) => self.emit_closure(closure, None, variables, body),
            Expr::CallValue(callee, arguments) => {
                let callee = self.emit_expression(callee, variables, body)?;
                self.emit_value_call(callee, arguments, variables, body)
            }
            Expr::Logical(operator, left, right) => {
                // Short-circuit: the right side runs only when it decides the
                // result. The result goes through a stack slot, so no phi needs
                // to know which block each side ended in.
                let symbol = if *operator == LogicalOp::And { "&&" } else { "||" };
                let left = self.emit_expression(left, variables, body)?;
                if left.value_type != ValueType::Bool {
                    return Err(format!("'{symbol}' requires bools, found {}", self.type_name(left.value_type)));
                }
                let label_id = self.next_label;
                self.next_label += 1;
                let prefix = if *operator == LogicalOp::And { "and" } else { "or" };
                let right_label = format!("{prefix}.rhs{label_id}");
                let end_label = format!("{prefix}.end{label_id}");
                let slot = self.new_register();
                self.entry_allocas.push_str(&format!("  {slot} = alloca i1, align 1\n"));
                let (if_true, if_false) = if *operator == LogicalOp::And {
                    (&right_label, &end_label)
                } else {
                    (&end_label, &right_label)
                };
                body.push_str(&format!(
                    "  store i1 {}, ptr {slot}, align 1\n  br i1 {}, label %{if_true}, label %{if_false}\n{right_label}:\n",
                    left.operand, left.operand
                ));
                self.open_scope();
                let right = self.emit_expression(right, variables, body)?;
                if right.value_type != ValueType::Bool {
                    return Err(format!("'{symbol}' requires bools, found {}", self.type_name(right.value_type)));
                }
                self.close_scope(body);
                let result = self.new_register();
                body.push_str(&format!(
                    "  store i1 {}, ptr {slot}, align 1\n  br label %{end_label}\n{end_label}:\n  {result} = load i1, ptr {slot}, align 1\n",
                    right.operand
                ));
                Ok(Value {
                    value_type: ValueType::Bool,
                    operand: result,
                    slot: None,
                })
            }
            Expr::Not(operand) => {
                let value = self.emit_expression(operand, variables, body)?;
                if value.value_type != ValueType::Bool {
                    return Err(format!("'!' requires a bool; found {}", self.type_name(value.value_type)));
                }
                let register = self.new_register();
                body.push_str(&format!("  {register} = xor i1 {}, true\n", value.operand));
                Ok(Value {
                    value_type: ValueType::Bool,
                    operand: register,
                    slot: None,
                })
            }
            Expr::BitNot(operand) => {
                let value = self.emit_expression(operand, variables, body)?;
                if value.value_type != ValueType::Int {
                    return Err(format!("'~' requires an int; found {}", self.type_name(value.value_type)));
                }
                let register = self.new_register();
                body.push_str(&format!("  {register} = xor i64 {}, -1\n", value.operand));
                Ok(Value {
                    value_type: ValueType::Int,
                    operand: register,
                    slot: None,
                })
            }
            Expr::Negate(operand) => {
                let value = self.emit_expression(operand, variables, body)?;
                let register = self.new_register();
                match value.value_type {
                    ValueType::Int => body.push_str(&format!("  {register} = sub i64 0, {}\n", value.operand)),
                    ValueType::Double => body.push_str(&format!("  {register} = fneg double {}\n", value.operand)),
                    other => {
                        return Err(format!(
                            "unary '-' requires an int or a double; found {}",
                            self.type_name(other)
                        ))
                    }
                }
                Ok(Value {
                    value_type: value.value_type,
                    operand: register,
                    slot: None,
                })
            }
        }
    }

    /// A string literal: a global with a static reference count (so it is
    /// never freed) in front of the bytes, as the runtime's QiqHeader.
    fn string_literal(&mut self, bytes: &[u8]) -> String {
        let name = format!("@.str{}", self.next_string);
        self.next_string += 1;
        let length = bytes.len() + 1;
        let layout = format!("{{ i64, i32, i32, [{length} x i8] }}");
        self.string_globals.push(format!(
            "{name} = private unnamed_addr constant {layout} {{ i64 -1, i32 {STRING_KIND}, i32 0, [{length} x i8] c\"{}\\00\" }}, align 8",
            llvm_escape(bytes)
        ));
        format!("getelementptr inbounds ({layout}, ptr {name}, i32 0, i32 3)")
    }

    fn encode_payload(&mut self, value: &Value, body: &mut String) -> Result<String, String> {
        let encoded = self.new_register();
        match value.value_type {
            ValueType::Int => body.push_str(&format!(
                "  {encoded} = add i64 {}, 0\n",
                value.operand
            )),
            ValueType::Double => body.push_str(&format!(
                "  {encoded} = bitcast double {} to i64\n",
                value.operand
            )),
            ValueType::Bool => body.push_str(&format!(
                "  {encoded} = zext i1 {} to i64\n",
                value.operand
            )),
            ValueType::Byte => body.push_str(&format!(
                "  {encoded} = zext i8 {} to i64\n",
                value.operand
            )),
            ValueType::String
            | ValueType::Char
            | ValueType::Array(_)
            | ValueType::Map(_)
            | ValueType::Class(_)
            | ValueType::Function(_) => body.push_str(&format!(
                "  {encoded} = ptrtoint ptr {} to i64\n",
                value.operand
            )),
            ValueType::Void | ValueType::Null => {
                return Err("null values cannot be stored in a map".to_string())
            }
        }
        Ok(encoded)
    }

    fn decode_payload(
        &mut self,
        value_type: ValueType,
        encoded: &str,
        body: &mut String,
    ) -> Result<Value, String> {
        let decoded = self.new_register();
        match value_type {
            ValueType::Int => body.push_str(&format!(
                "  {decoded} = add i64 {encoded}, 0\n"
            )),
            ValueType::Double => body.push_str(&format!(
                "  {decoded} = bitcast i64 {encoded} to double\n"
            )),
            ValueType::Bool => body.push_str(&format!(
                "  {decoded} = trunc i64 {encoded} to i1\n"
            )),
            ValueType::Byte => body.push_str(&format!(
                "  {decoded} = trunc i64 {encoded} to i8\n"
            )),
            ValueType::String
            | ValueType::Char
            | ValueType::Array(_)
            | ValueType::Map(_)
            | ValueType::Class(_)
            | ValueType::Function(_) => body.push_str(&format!(
                "  {decoded} = inttoptr i64 {encoded} to ptr\n"
            )),
            ValueType::Void | ValueType::Null => {
                return Err("map values cannot be void or null".to_string())
            }
        }
        Ok(Value {
            value_type,
            operand: decoded,
            slot: None,
        })
    }

    fn new_register(&mut self) -> String {
        let register = format!("%v{}", self.next_register);
        self.next_register += 1;
        register
    }
}

fn llvm_escape(bytes: &[u8]) -> String {
    let mut escaped = String::new();
    for byte in bytes {
        if (0x20..=0x7e).contains(byte) && *byte != b'"' && *byte != b'\\' {
            escaped.push(*byte as char);
        } else {
            escaped.push_str(&format!("\\{byte:02X}"));
        }
    }
    escaped
}

fn emit_llvm(program: &Program) -> Result<String, String> {
    let signatures = program
        .functions
        .iter()
        .map(|function| {
            (
                function.symbol.clone(),
                FunctionSignature {
                    parameters: function.parameters.clone(),
                    return_type: function.return_type,
                },
            )
        })
        .collect();
    Emitter::new(
        signatures,
        program.map_types.clone(),
        program.array_types.clone(),
        program.function_types.clone(),
        program.classes.clone(),
        &program.modules,
    )
    .emit_program(program)
}

/// Whether `statements` contain a `break` that leaves the loop they belong to
/// (breaks inside nested loops leave those loops instead).
/// How many values a literal sprintf format uses, following the same rules
/// as the runtime (qiq_sprintf): `%[argnum$][flags][width][.precision]specifier`.
fn sprintf_arguments_required(format: &str) -> Result<usize, String> {
    let characters: Vec<char> = format.chars().collect();
    let mut cursor = 0;
    let mut next = 0usize;
    let mut required = 0usize;
    let digits_at = |cursor: &mut usize| -> usize {
        let mut value = 0usize;
        while *cursor < characters.len() && characters[*cursor].is_ascii_digit() {
            value = value.saturating_mul(10).saturating_add(characters[*cursor] as usize - '0' as usize);
            *cursor += 1;
        }
        value
    };
    while cursor < characters.len() {
        if characters[cursor] != '%' {
            cursor += 1;
            continue;
        }
        cursor += 1;
        match characters.get(cursor) {
            None => return Err("missing format specifier at end of string".to_string()),
            Some('%') => {
                cursor += 1;
                continue;
            }
            _ => {}
        }
        let argument;
        if !characters[cursor].is_ascii_alphabetic() {
            let mut look = cursor;
            while look < characters.len() && characters[look].is_ascii_digit() {
                look += 1;
            }
            if characters.get(look) == Some(&'$') {
                let number = digits_at(&mut cursor);
                if number == 0 {
                    return Err("argument number specifier must be greater than zero".to_string());
                }
                argument = number - 1;
                cursor += 1;
            } else {
                argument = next;
                next += 1;
            }
            loop {
                match characters.get(cursor) {
                    Some(' ' | '0' | '-' | '+') => cursor += 1,
                    Some('\'') => {
                        if cursor + 1 >= characters.len() {
                            return Err("missing padding character".to_string());
                        }
                        cursor += 2;
                    }
                    _ => break,
                }
            }
            digits_at(&mut cursor);
            if characters.get(cursor) == Some(&'.') {
                cursor += 1;
                digits_at(&mut cursor);
            }
        } else {
            argument = next;
            next += 1;
        }
        if characters.get(cursor) == Some(&'l') {
            cursor += 1;
        }
        match characters.get(cursor) {
            None => return Err("missing format specifier at end of string".to_string()),
            Some(specifier) if "bcdeEfFgGosuxX".contains(*specifier) => cursor += 1,
            Some(specifier) => return Err(format!("unknown format specifier \"{specifier}\"")),
        }
        required = required.max(argument + 1);
    }
    Ok(required)
}

/// Whether values of this type are reference counted: strings, chars,
/// arrays, maps, and objects.
/// Whether evaluating `expression` can't run code that changes the program's
/// arrays, maps, objects, or variables, so a value borrowed before it stays
/// valid: literals, variables, and element and property reads of those (which
/// at most throw). See emit_borrowed.
fn is_pure(expression: &Expr) -> bool {
    match expression {
        Expr::Number(_) | Expr::Decimal(_) | Expr::Boolean(_) | Expr::Null | Expr::Text(_) | Expr::Character(_) | Expr::Variable(_) | Expr::One => true,
        Expr::Index(container, key) => is_pure(container) && is_pure(key),
        Expr::Member(object, _) => is_pure(object),
        _ => false,
    }
}

fn is_reference(value_type: ValueType) -> bool {
    matches!(
        value_type,
        ValueType::String
            | ValueType::Char
            | ValueType::Array(_)
            | ValueType::Map(_)
            | ValueType::Class(_)
            | ValueType::Function(_)
    )
}

/// The variables an anonymous function uses from the code around it: names
/// it reads or assigns (directly or through functions nested in it), minus
/// its parameters and the names it declares. `this` counts when the body
/// uses it (including through `parent::` and `self::` calls).
fn closure_free_names(closure: &Closure) -> std::collections::BTreeSet<String> {
    let mut used = std::collections::BTreeSet::new();
    let mut declared: std::collections::BTreeSet<String> = closure.parameters.iter().map(|parameter| parameter.name.clone()).collect();
    for statement in &closure.body {
        statement_names(statement, &mut used, &mut declared);
    }
    used.difference(&declared).cloned().collect()
}

/// The variables a function body's anonymous functions capture, which must
/// live in cells so the function and its closures share them.
fn captured_names(statements: &[Statement]) -> std::collections::BTreeSet<String> {
    let mut used = std::collections::BTreeSet::new();
    let mut declared = std::collections::BTreeSet::new();
    let mut captured = std::collections::BTreeSet::new();
    for statement in statements {
        statement_names(statement, &mut used, &mut declared);
    }
    // statement_names adds each nested closure's free names to `used`, and
    // marks them with a prefix in `declared` so they can be told apart.
    for name in &declared {
        if let Some(name) = name.strip_prefix('\u{1}') {
            captured.insert(name.to_string());
        }
    }
    captured
}

fn statement_names(statement: &Statement, used: &mut std::collections::BTreeSet<String>, declared: &mut std::collections::BTreeSet<String>) {
    let expressions = |expression: &Expr, used: &mut std::collections::BTreeSet<String>, declared: &mut std::collections::BTreeSet<String>| {
        expression_names(expression, used, declared)
    };
    match &statement.kind {
        StatementKind::Declare(_, name, expression) => {
            expressions(expression, used, declared);
            declared.insert(name.clone());
        }
        StatementKind::DeclareArray(_, name, initializer) => {
            if let ArrayInitializer::Existing(expression) = initializer {
                expressions(expression, used, declared);
            }
            declared.insert(name.clone());
        }
        StatementKind::DeclareMap(_, name, initializer) => {
            if let MapInitializer::Existing(expression) = initializer {
                expressions(expression, used, declared);
            }
            declared.insert(name.clone());
        }
        StatementKind::Assign(name, expression) => {
            used.insert(name.clone());
            expressions(expression, used, declared);
        }
        StatementKind::AssignIndex(a, b, c) => {
            for expression in [a, b, c] {
                expressions(expression, used, declared);
            }
        }
        StatementKind::AssignMember(object, _, value) => {
            expressions(object, used, declared);
            expressions(value, used, declared);
        }
        StatementKind::AssignStatic(target, _, value) => {
            if !matches!(target, StaticTarget::Class { forwards: false, .. }) {
                used.insert("this".to_string());
            }
            expressions(value, used, declared);
        }
        StatementKind::Print(values, _) => {
            for expression in values {
                expressions(expression, used, declared);
            }
        }
        StatementKind::Expression(expression) | StatementKind::Throw(expression) => expressions(expression, used, declared),
        StatementKind::Return(value) => {
            if let Some(expression) = value {
                expressions(expression, used, declared);
            }
        }
        StatementKind::Break | StatementKind::Continue => {}
        StatementKind::Try { body, catches, finally } => {
            for statement in body {
                statement_names(statement, used, declared);
            }
            for clause in catches {
                declared.insert(clause.name.clone());
                for statement in &clause.body {
                    statement_names(statement, used, declared);
                }
            }
            for statement in finally.iter().flatten() {
                statement_names(statement, used, declared);
            }
        }
        StatementKind::If { condition, then_body, else_body } => {
            expressions(condition, used, declared);
            for statement in then_body.iter().chain(else_body) {
                statement_names(statement, used, declared);
            }
        }
        StatementKind::Switch { subject, cases, default } => {
            expressions(subject, used, declared);
            for case in cases {
                for value in &case.values {
                    expressions(value, used, declared);
                }
                for statement in &case.body {
                    statement_names(statement, used, declared);
                }
            }
            for statement in default.iter().flatten() {
                statement_names(statement, used, declared);
            }
        }
        StatementKind::While { condition, body } => {
            expressions(condition, used, declared);
            for statement in body {
                statement_names(statement, used, declared);
            }
        }
        StatementKind::Foreach { iterable, key_name, value_name, body } => {
            expressions(iterable, used, declared);
            if let Some(key) = key_name {
                declared.insert(key.clone());
            }
            declared.insert(value_name.clone());
            for statement in body {
                statement_names(statement, used, declared);
            }
        }
    }
}

fn expression_names(expression: &Expr, used: &mut std::collections::BTreeSet<String>, declared: &mut std::collections::BTreeSet<String>) {
    let each = |expressions: &[&Expr], used: &mut std::collections::BTreeSet<String>, declared: &mut std::collections::BTreeSet<String>| {
        for expression in expressions {
            expression_names(expression, used, declared);
        }
    };
    match expression {
        Expr::Number(_) | Expr::Decimal(_) | Expr::Boolean(_) | Expr::Null | Expr::Text(_) | Expr::Character(_) | Expr::One => {}
        Expr::InstanceOf(operand, _, binding) => {
            expression_names(operand, used, declared);
            if let Some(name) = binding {
                declared.insert(name.clone());
            }
        }
        Expr::Downcast(operand, _) => expression_names(operand, used, declared),
        Expr::Nullsafe(object, placeholder, rest) => {
            expression_names(object, used, declared);
            declared.insert(placeholder.clone());
            expression_names(rest, used, declared);
        }
        Expr::Variable(name) => {
            used.insert(name.clone());
        }
        Expr::Call(name, arguments) => {
            // A call by name may be a call to a function value in a variable.
            used.insert(name.clone());
            each(&arguments.iter().collect::<Vec<_>>(), used, declared);
        }
        Expr::Binary(_, a, b) | Expr::Compare(_, a, b) | Expr::Logical(_, a, b) | Expr::Coalesce(a, b) | Expr::Index(a, b) => {
            each(&[a, b], used, declared)
        }
        Expr::Negate(a) | Expr::BitNot(a) | Expr::Not(a) | Expr::Member(a, _) | Expr::NewArray(_, a) | Expr::NamedArgument(_, a) => {
            each(&[a], used, declared)
        }
        Expr::MethodCall(object, _, arguments) | Expr::CallValue(object, arguments) => {
            expression_names(object, used, declared);
            each(&arguments.iter().collect::<Vec<_>>(), used, declared);
        }
        Expr::StaticCall(target, _, arguments) => {
            if !matches!(target, StaticTarget::Class { forwards: false, .. }) {
                used.insert("this".to_string());
            }
            each(&arguments.iter().collect::<Vec<_>>(), used, declared);
        }
        Expr::StaticProperty(target, _) => {
            if !matches!(target, StaticTarget::Class { forwards: false, .. }) {
                used.insert("this".to_string());
            }
        }
        // static::class reads the called class from `this`.
        Expr::ClassName(target) => {
            if matches!(target, StaticTarget::Late { .. }) {
                used.insert("this".to_string());
            }
        }
        Expr::ClassOf(object) => expression_names(object, used, declared),
        Expr::NewStatic(_, arguments) => {
            used.insert("this".to_string());
            each(&arguments.iter().collect::<Vec<_>>(), used, declared);
        }
        Expr::NewObject(_, arguments) | Expr::ArrayLiteral(arguments) => each(&arguments.iter().collect::<Vec<_>>(), used, declared),
        Expr::MapLiteral(entries) => {
            for (key, value) in entries {
                each(&[key, value], used, declared);
            }
        }
        Expr::Closure(closure) => {
            for name in closure_free_names(closure) {
                // Captured by a nested function: used here, and noted (with a
                // prefix no identifier has) for captured_names.
                declared.insert(format!("\u{1}{name}"));
                used.insert(name);
            }
        }
    }
}

/// Whether any statement assigns to the variable `name` with `name = ...`.
fn assigns(statements: &[Statement], name: &str) -> bool {
    statements.iter().any(|statement| match &statement.kind {
        StatementKind::Assign(target, _) => target == name,
        StatementKind::If { then_body, else_body, .. } => assigns(then_body, name) || assigns(else_body, name),
        StatementKind::While { body, .. } | StatementKind::Foreach { body, .. } => assigns(body, name),
        StatementKind::Switch { cases, default, .. } => {
            cases.iter().any(|case| assigns(&case.body, name))
                || default.as_deref().is_some_and(|body| assigns(body, name))
        }
        StatementKind::Try { body, catches, finally } => {
            assigns(body, name)
                || catches.iter().any(|clause| assigns(&clause.body, name))
                || finally.as_deref().is_some_and(|body| assigns(body, name))
        }
        _ => false,
    })
}

fn contains_break(statements: &[Statement]) -> bool {
    statements.iter().any(|statement| match &statement.kind {
        StatementKind::Break => true,
        StatementKind::If { then_body, else_body, .. } => contains_break(then_body) || contains_break(else_body),
        StatementKind::Try { body, catches, finally } => {
            contains_break(body)
                || catches.iter().any(|clause| contains_break(&clause.body))
                || finally.as_deref().is_some_and(contains_break)
        }
        _ => false,
    })
}

/// `declare` line for an extern function. C `bool` crosses the ABI as a
/// zero-extended byte, so `i1` values carry `zeroext`.
fn extern_declaration(function: &Function) -> String {
    // The attribute follows the type for parameters and precedes it for the return.
    let parameters: Vec<String> = function
        .parameters
        .iter()
        .map(|parameter| match parameter.value_type {
            ValueType::Bool => "i1 zeroext".to_string(),
            ValueType::Byte => "i8 zeroext".to_string(),
            other => other.llvm().to_string(),
        })
        .collect();
    let return_type = match function.return_type {
        ValueType::Bool => "zeroext i1".to_string(),
        ValueType::Byte => "zeroext i8".to_string(),
        other => other.llvm().to_string(),
    };
    format!(
        "declare {} @{}({})\n",
        return_type,
        function.symbol,
        parameters.join(", ")
    )
}

/// True when the tokens at `cursor` start an import: `import Name ...`.
/// `import` and `from` are contextual, so both remain usable as identifiers.
fn is_import_at(tokens: &[Token], cursor: usize) -> bool {
    matches!(tokens.get(cursor).map(|token| &token.kind), Some(TokenKind::Identifier(word)) if word == "import")
        && matches!(tokens.get(cursor + 1).map(|token| &token.kind), Some(TokenKind::Identifier(_)))
}

/// One `import A, B as C from "path";` declaration.
struct Import {
    /// Imported names, the alias each is bound to (`as`), and the line and
    /// column where each appears.
    names: Vec<(String, Option<String>, usize, usize)>,
    path: String,
}

/// Parses an import starting at `cursor`; returns it and the cursor after it.
fn parse_import(tokens: &[Token], mut cursor: usize) -> Result<(Import, usize), String> {
    let error_at = |token: &Token, message: &str| format!("{}:{}: {message}", token.line, token.column);
    cursor += 1;
    let mut names = Vec::new();
    loop {
        let token = &tokens[cursor];
        let name = match &token.kind {
            TokenKind::Identifier(name) => name.clone(),
            _ => return Err(error_at(token, "expected a name to import")),
        };
        cursor += 1;
        let mut alias = None;
        if matches!(tokens[cursor].kind, TokenKind::As) {
            cursor += 1;
            match &tokens[cursor].kind {
                TokenKind::Identifier(alias_name) => alias = Some(alias_name.clone()),
                _ => return Err(error_at(&tokens[cursor], "expected a name after 'as'")),
            }
            cursor += 1;
        }
        names.push((name, alias, token.line, token.column));
        if !matches!(tokens[cursor].kind, TokenKind::Comma) {
            break;
        }
        cursor += 1;
    }
    let token = &tokens[cursor];
    if !matches!(&token.kind, TokenKind::Identifier(word) if word == "from") {
        return Err(error_at(token, "expected 'from' after imported names"));
    }
    cursor += 1;
    let token = &tokens[cursor];
    let path = match &token.kind {
        TokenKind::Text(path) => path.clone(),
        _ => return Err(error_at(token, "expected a quoted file path after 'from'")),
    };
    cursor += 1;
    if !matches!(tokens[cursor].kind, TokenKind::Semicolon) {
        return Err(error_at(&tokens[cursor], "expected ';' after import"));
    }
    Ok((Import { names, path }, cursor + 1))
}

/// Removes `.` and `..` components without touching the file system, so the
/// same file reached through different relative paths is loaded once.
/// Splits an error that starts with a position into `"line:column: "` and
/// the message after it.
fn split_position(error: &str) -> Option<(&str, &str)> {
    let mut parts = error.splitn(3, ':');
    let is_number = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    let (line, column, rest) = (parts.next()?, parts.next()?, parts.next()?);
    if !is_number(line) || !is_number(column) || !rest.starts_with(' ') {
        return None;
    }
    let length = line.len() + column.len() + 3;
    Some((&error[..length], &error[length..]))
}

/// Prefixes an error with a position, unless it already has one.
fn at(line: usize, column: usize, error: String) -> String {
    if has_position(&error) || line == 0 {
        error
    } else {
        format!("{line}:{column}: {error}")
    }
}

/// Whether an error message already starts with a position, `line:column: `.
fn has_position(error: &str) -> bool {
    split_position(error).is_some()
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if matches!(normalized.components().next_back(), Some(Component::Normal(_))) {
                    normalized.pop();
                } else {
                    normalized.push("..");
                }
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

/// Resolves an import path relative to the importing file, adding `.qiq` when
/// the path has no extension.
fn resolve_import(importer: &Path, import_path: &str) -> PathBuf {
    let mut path = importer.parent().unwrap_or(Path::new("")).join(import_path);
    if path.extension().is_none() {
        path.set_extension("qiq");
    }
    normalize_path(&path)
}

/// Loads the entry module and everything it imports, depth-first, into one program.
struct Loader<'a> {
    read: &'a dyn Fn(&Path) -> Result<String, String>,
    modules: Vec<Module>,
    module_ids: HashMap<PathBuf, u32>,
    /// Modules currently being loaded, outermost first; used to report cycles.
    loading: Vec<PathBuf>,
    classes: Vec<ClassDef>,
    map_types: Vec<MapType>,
    array_types: Vec<ValueType>,
    function_types: Vec<FunctionType>,
    functions: Vec<Function>,
    statements: Vec<Statement>,
    /// Standard library exports, visible in every non-stdlib module.
    prelude_functions: HashMap<String, String>,
    prelude_classes: HashMap<String, u32>,
    stdlib_loaded: bool,
    /// Modules (qiq.mod packages). Package 0 is the main module, the one
    /// containing the entry file; its path is empty when there's no qiq.mod.
    packages: Vec<Package>,
    /// The main module's requirements: module path -> package index.
    required: HashMap<String, u32>,
    /// The main module's qiq.mod, for messages.
    mod_file: Option<String>,
    /// The main module's qiq.sum.
    sums: Sums,
}

/// A module: a directory of qiq files with a qiq.mod.
struct Package {
    path: String,
    root: PathBuf,
    /// The required version; None for the main module.
    version: Option<Version>,
    /// Points at a local directory with `replace`, rather than the cache.
    replaced: bool,
    /// Whether its qiq.mod has been read and its requirements checked.
    checked: bool,
}

impl<'a> Loader<'a> {
    fn new(read: &'a dyn Fn(&Path) -> Result<String, String>) -> Self {
        Self {
            read,
            modules: Vec::new(),
            module_ids: HashMap::new(),
            loading: Vec::new(),
            classes: Vec::new(),
            map_types: Vec::new(),
            array_types: Vec::new(),
            function_types: Vec::new(),
            functions: Vec::new(),
            statements: Vec::new(),
            prelude_functions: HashMap::new(),
            prelude_classes: HashMap::new(),
            stdlib_loaded: false,
            packages: vec![Package {
                path: String::new(),
                root: PathBuf::new(),
                version: None,
                replaced: false,
                checked: true,
            }],
            required: HashMap::new(),
            mod_file: None,
            sums: Sums::new(),
        }
    }

    /// Finds the main module's qiq.mod in the entry file's directory or the
    /// nearest directory above it (at most `max_up` levels), and sets up the
    /// modules it requires. Without one, the program is a module with no path.
    fn find_main_module(&mut self, entry: &Path, max_up: usize) -> Result<(), String> {
        let entry_directory = normalize_path(entry.parent().unwrap_or(Path::new("")));
        self.packages[0].root = entry_directory.clone();
        let mut directory = entry_directory;
        for _ in 0..=max_up {
            let file = directory.join(MOD_FILE);
            if let Ok(text) = (self.read)(&file) {
                let display = file.display().to_string();
                let parsed = modfile::parse(&text, &display)?;
                self.packages[0].path = parsed.module.clone();
                self.packages[0].root = directory.clone();
                for (path, version) in parsed.requires {
                    let replacement = parsed.replaces.iter().find(|(replaced, _)| *replaced == path);
                    let root = match replacement {
                        Some((_, target)) => normalize_path(&directory.join(target)),
                        None => modfile::cache_directory().join(format!("{path}@{version}")),
                    };
                    self.required.insert(path.clone(), self.packages.len() as u32);
                    self.packages.push(Package {
                        path,
                        root,
                        version: Some(version),
                        replaced: replacement.is_some(),
                        checked: false,
                    });
                }
                self.mod_file = Some(display);
                let sum_file = directory.join(SUM_FILE);
                if let Ok(text) = (self.read)(&sum_file) {
                    self.sums = fetch::parse_sums(&text, &sum_file.display().to_string())?;
                }
                return Ok(());
            }
            directory = normalize_path(&directory.join(".."));
        }
        Ok(())
    }

    /// Resolves an import made by the file `importer` in module `package`:
    /// a path in the importer's own module or in a required module, or else
    /// a path relative to the importer. Returns the file and its module.
    fn resolve(&mut self, importer: &Path, package: u32, import: &str) -> Result<(PathBuf, u32), String> {
        if import.starts_with(STDLIB_PREFIX) {
            let file = stdlib_file(import).ok_or_else(|| {
                if let Some((_, now)) = MOVED_STDLIB_MODULES.iter().find(|(old, _)| *old == import) {
                    return format!("the standard library module \"{import}\" moved to \"{now}\": import from there instead");
                }
                let paths: Vec<&str> = STDLIB_MODULES.iter().filter_map(|(_, path, _)| *path).collect();
                format!("the standard library has no module \"{import}\"; its modules are {}", paths.join(", "))
            })?;
            return Ok((Path::new(STDLIB_ROOT).join(file), package));
        }
        let explicit_relative = import.starts_with("./") || import.starts_with("../") || import.starts_with('/');
        let within = |module: &str| !module.is_empty() && (import == module || import.starts_with(&format!("{module}/")));
        let mut target = None;
        if !explicit_relative {
            if within(&self.packages[package as usize].path) {
                target = Some(package);
            } else {
                // The longest required module path that contains the import.
                target = self
                    .required
                    .iter()
                    .filter(|(module, _)| within(module))
                    .max_by_key(|(module, _)| module.len())
                    .map(|(_, &id)| id);
            }
        }
        let Some(target) = target else {
            let relative = resolve_import(importer, import);
            let first = import.split('/').next().unwrap_or("");
            let looks_like_module = !explicit_relative && first.contains('.') && !first.starts_with('.') && import.contains('/');
            if looks_like_module && (self.read)(&relative).is_err() {
                return Err(match &self.mod_file {
                    None => format!(
                        "\"{import}\" looks like a module path, but this program has no {MOD_FILE}; create one with 'qiq mod init <module path>' and require the module in it"
                    ),
                    Some(mod_file) => format!(
                        "\"{import}\" isn't in a required module; add 'require <module path> <version>' to {mod_file}"
                    ),
                });
            }
            return Ok((relative, package));
        };
        self.check_package(target)?;
        let module = &self.packages[target as usize];
        let rest = import[module.path.len()..].trim_start_matches('/');
        if target != package && rest.split('/').any(|segment| segment == "internal") {
            return Err(format!(
                "cannot import \"{import}\": files in an internal directory can only be imported from within {}",
                module.path
            ));
        }
        // The module path alone means the file named after its last element.
        let file = if rest.is_empty() { module.path.rsplit('/').next().unwrap_or("") } else { rest };
        let mut path = module.root.join(file);
        if path.extension().is_none() {
            path.set_extension("qiq");
        }
        Ok((normalize_path(&path), target))
    }

    /// Before a required module's first file loads: checks that it's there,
    /// that its qiq.mod names it, and that the main module requires
    /// everything it requires, at the same or a newer version.
    fn check_package(&mut self, id: u32) -> Result<(), String> {
        if self.packages[id as usize].checked {
            return Ok(());
        }
        self.packages[id as usize].checked = true;
        let main_mod = self.mod_file.clone().unwrap_or_else(|| MOD_FILE.to_string());
        let package = &self.packages[id as usize];
        let version = package.version.clone().map(|version| version.to_string()).unwrap_or_default();
        let name = format!("{} {version}", package.path);
        if !package.replaced {
            // Downloaded modules must match the checksum qiq get recorded.
            let version = package.version.clone().expect("required modules have versions");
            let key = (package.path.clone(), version.to_string());
            let Some(expected) = self.sums.get(&key).cloned() else {
                return Err(format!(
                    "{name} has no checksum in {SUM_FILE}; run 'qiq get {}@{version}' to download it and record one",
                    package.path
                ));
            };
            if !package.root.is_dir() {
                let path = package.path.clone();
                fetch::download(&path, &version, Some(&expected)).map_err(|error| format!("{error}"))?;
            }
        }
        let package = &self.packages[id as usize];
        let file = package.root.join(MOD_FILE);
        let Ok(text) = (self.read)(&file) else {
            return Ok(());
        };
        let parsed = modfile::parse(&text, &file.display().to_string())?;
        if parsed.module != package.path {
            return Err(format!(
                "{} declares module {}, but {main_mod} requires it as {}",
                file.display(),
                parsed.module,
                package.path
            ));
        }
        for (path, needed) in parsed.requires {
            match self.required.get(&path) {
                None => {
                    return Err(format!(
                        "{name} requires {path} {needed}; add 'require {path} {needed}' to {main_mod}"
                    ))
                }
                Some(&other) => {
                    let chosen = self.packages[other as usize].version.clone().expect("required modules have versions");
                    if chosen < needed {
                        return Err(format!(
                            "{name} requires {path} {needed} or newer, but {main_mod} requires {chosen}; change it to {needed}"
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    /// Loads every standard library module once and collects the prelude
    /// modules' exports into the prelude.
    fn load_stdlib(&mut self) -> Result<(), String> {
        if self.stdlib_loaded {
            return Ok(());
        }
        self.stdlib_loaded = true;
        for (name, import_path, _) in STDLIB_MODULES {
            let id = self.load(Path::new(STDLIB_ROOT).join(name), None, 0)?;
            if import_path.is_some() {
                continue;  // imported explicitly, not part of the prelude
            }
            let module = &self.modules[id as usize];
            for (function, symbol) in &module.exported_functions {
                // Names starting with "__" are internal to the standard library.
                if function.starts_with("__") {
                    continue;
                }
                if self.prelude_functions.insert(function.clone(), symbol.clone()).is_some() {
                    return Err(format!("standard library declares '{function}' more than once"));
                }
            }
            for (class, class_id) in &module.exported_classes {
                if self.prelude_classes.insert(class.clone(), *class_id).is_some() {
                    return Err(format!("standard library declares '{class}' more than once"));
                }
            }
        }
        Ok(())
    }

    fn into_program(self) -> Program {
        Program {
            functions: self.functions,
            classes: self.classes,
            statements: self.statements,
            map_types: self.map_types,
            array_types: self.array_types,
            function_types: self.function_types,
            modules: self.modules,
        }
    }

    /// Loads the module at `path`. `imported_at` locates the import that asked
    /// for it (`file:line:column`), used when the file cannot be read.
    fn load(&mut self, path: PathBuf, imported_at: Option<String>, package: u32) -> Result<u32, String> {
        if let Some(&id) = self.module_ids.get(&path) {
            if let Some(start) = self.loading.iter().position(|loading| *loading == path) {
                let cycle: Vec<String> = self.loading[start..]
                    .iter()
                    .chain(std::iter::once(&path))
                    .map(|path| path.display().to_string())
                    .collect();
                return Err(format!("import cycle: {}", cycle.join(" -> ")));
            }
            return Ok(id);
        }

        let id = self.modules.len() as u32;
        let display = path.display().to_string();
        let is_stdlib = path.starts_with(STDLIB_ROOT);
        // Symbol prefix that keeps names unique across modules: none for the
        // entry module, `stdlib.<name>` for the standard library, and
        // `<name>.<n>` for the n-th imported user module.
        let tag = if id == 0 {
            String::new()
        } else {
            let stem: String = path
                .file_stem()
                .map(|stem| stem.to_string_lossy().to_string())
                .unwrap_or_default()
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
                .collect();
            if is_stdlib {
                format!("stdlib.{stem}")
            } else {
                let user_index = self.modules.iter().filter(|module| !module.is_stdlib).count();
                format!("{stem}.{user_index}")
            }
        };
        self.modules.push(Module {
            path: display.clone(),
            is_stdlib,
            package,
            ..Module::default()
        });
        self.module_ids.insert(path.clone(), id);
        self.loading.push(path.clone());
        if !is_stdlib {
            self.load_stdlib()?;
        }

        let at = |error: String| if display.is_empty() { error } else { format!("{display}:{error}") };
        let source = (self.read)(&path).map_err(|error| match &imported_at {
            Some(location) => format!("{location}: {error}"),
            None => error,
        })?;
        let tokens = lex(&source).map_err(at)?;

        let mut cursor = 0;
        // Standard library modules import each other explicitly; every other
        // module starts with the prelude in scope.
        let (mut class_names, mut functions) = if is_stdlib {
            (HashMap::new(), HashMap::new())
        } else {
            (self.prelude_classes.clone(), self.prelude_functions.clone())
        };
        while is_import_at(&tokens, cursor) {
            let location = at(format!("{}:{}", tokens[cursor].line, tokens[cursor].column));
            let (import, next) = parse_import(&tokens, cursor).map_err(at)?;
            cursor = next;
            let (dependency_path, dependency_package) = if is_stdlib {
                (resolve_import(&path, &import.path), package)
            } else {
                self.resolve(&path, package, &import.path).map_err(|error| format!("{location}: {error}"))?
            };
            let dependency = self.load(dependency_path, Some(location), dependency_package)?;
            let module = &self.modules[dependency as usize];
            for (name, alias, line, column) in import.names {
                let bound = alias.clone().unwrap_or_else(|| name.clone());
                if !is_stdlib
                    && (self.prelude_classes.contains_key(&bound)
                        || self.prelude_functions.contains_key(&bound)
                        || CONVERSION_FUNCTIONS.contains(&bound.as_str()))
                {
                    return Err(at(match alias {
                        None => format!("{line}:{column}: '{name}' is built in and needs no import"),
                        Some(_) => format!("{line}:{column}: '{bound}' is built in; choose another name after 'as'"),
                    }));
                }
                let duplicate = class_names.contains_key(&bound) || functions.contains_key(&bound);
                if duplicate {
                    return Err(at(format!(
                        "{line}:{column}: '{bound}' is already imported; use 'import {name} as <other name>' to import it under another name"
                    )));
                }
                let hidden = module.is_stdlib && !is_stdlib && name.starts_with("__");
                let exported = !hidden
                    && (module.exported_classes.contains_key(&name) || module.exported_functions.contains_key(&name));
                if exported && module.package != package && module.private_names.contains(&name) {
                    return Err(at(format!(
                        "{line}:{column}: '{name}' is private to module {}",
                        self.packages[module.package as usize].path
                    )));
                }
                if let (true, Some(&class_id)) = (exported, module.exported_classes.get(&name)) {
                    class_names.insert(bound, class_id);
                } else if let (true, Some(symbol)) = (exported, module.exported_functions.get(&name)) {
                    functions.insert(bound, symbol.clone());
                } else {
                    let dependency = &self.packages[module.package as usize];
                    let message = if module.is_stdlib && !is_stdlib {
                        format!("'{}' does not declare '{name}'{}", import.path, stdlib_hint(&name))
                    } else if module.package != package && dependency.version.is_some() {
                        // A file of a required module: name the module and its
                        // version rather than the file in the cache.
                        let version = dependency.version.clone().expect("checked above");
                        let shown = if import.path == dependency.path {
                            format!("{} {version}", dependency.path)
                        } else {
                            format!("\"{}\" ({} {version})", import.path, dependency.path)
                        };
                        format!(
                            "{shown} does not declare '{name}'; if a newer version adds it, update with 'qiq get -u {}'",
                            dependency.path
                        )
                    } else {
                        format!("'{}' does not declare '{name}'", module.path)
                    };
                    return Err(at(format!("{line}:{column}: {message}")));
                }
            }
        }

        let mut parser = Parser {
            tokens,
            cursor,
            map_types: std::mem::take(&mut self.map_types),
            array_types: std::mem::take(&mut self.array_types),
            function_types: std::mem::take(&mut self.function_types),
            class_names,
            classes: std::mem::take(&mut self.classes),
            module: id,
            module_tag: tag,
            allow_extern: is_stdlib,
            current_class: None,
            current_parent: None,
            private_names: HashSet::new(),
            next_hidden: 0,
            promoted: Vec::new(),
        };
        let parsed = parser.parse_module();
        let private_names = std::mem::take(&mut parser.private_names);
        self.classes = parser.classes;
        self.map_types = parser.map_types;
        self.array_types = parser.array_types;
        self.function_types = parser.function_types;
        let (declared_functions, statements) = parsed.map_err(at)?;

        let mut exported_functions = HashMap::new();
        for function in declared_functions {
            let in_file = |error: String| at(format!("{}:{}: {error}", function.line, function.column));
            if function.name == "main" {
                return Err(in_file("function name 'main' is reserved".to_string()));
            }
            if exported_functions.contains_key(&function.name) {
                return Err(in_file(format!("function '{}' is already declared", function.name)));
            }
            if !is_stdlib
                && (self.prelude_functions.contains_key(&function.name)
                    || CONVERSION_FUNCTIONS.contains(&function.name.as_str()))
            {
                return Err(in_file(format!(
                    "cannot redeclare built-in function '{}'",
                    function.name
                )));
            }
            if functions.contains_key(&function.name) {
                return Err(in_file(format!(
                    "function '{}' conflicts with an imported function",
                    function.name
                )));
            }
            exported_functions.insert(function.name.clone(), function.symbol.clone());
            self.functions.push(function);
        }
        functions.extend(exported_functions.clone());
        if id == 0 {
            self.statements = statements;
        } else if let Some(statement) = statements.first() {
            return Err(at(format!(
                "{}:{}: imported files can only contain imports and declarations, not top-level statements",
                statement.line, statement.column
            )));
        }

        if !is_stdlib {
            if let Some(class) = self.classes.iter().find(|class| {
                class.module == id && self.prelude_classes.contains_key(&class.name)
            }) {
                return Err(at(format!("{}:{}: cannot redeclare built-in class '{}'", class.line, class.column, class.name)));
            }
        }
        let exported_classes = self
            .classes
            .iter()
            .enumerate()
            .filter(|(_, class)| class.module == id)
            .map(|(class_id, class)| (class.name.clone(), class_id as u32))
            .collect();
        let module = &mut self.modules[id as usize];
        module.functions = functions;
        module.exported_classes = exported_classes;
        module.exported_functions = exported_functions;
        module.private_names = private_names;
        self.loading.pop();
        Ok(id)
    }
}

/// Directory name under which standard library modules are loaded and shown
/// in error messages. It cannot collide with a real relative path.
const STDLIB_ROOT: &str = "<stdlib>";

/// Standard library modules, embedded in the compiler: file name, import
/// path, and source. Modules without an import path are the prelude, whose
/// names are available in every file; the others are imported, as in
/// `import File from "std/file";`. Set `QIQ_STDLIB` to a directory to load
/// these files from disk instead (for stdlib development).
const STDLIB_MODULES: &[(&str, Option<&str>, &str)] = &[
    ("string.qiq", None, include_str!("../stdlib/string.qiq")),
    ("exceptions.qiq", None, include_str!("../stdlib/exceptions.qiq")),
    ("file.qiq", Some("std/file"), include_str!("../stdlib/file.qiq")),
    ("date.qiq", Some("std/time"), include_str!("../stdlib/date.qiq")),
    ("memory.qiq", Some("std/memory"), include_str!("../stdlib/memory.qiq")),
    ("program.qiq", Some("std/os"), include_str!("../stdlib/program.qiq")),
    ("net.qiq", Some("std/net"), include_str!("../stdlib/net.qiq")),
    ("http.qiq", Some("std/net/http"), include_str!("../stdlib/http.qiq")),
    ("server.qiq", Some("std/net/http/server"), include_str!("../stdlib/server.qiq")),
    ("crypto.qiq", Some("std/crypto"), include_str!("../stdlib/crypto.qiq")),
    ("sql.qiq", Some("std/sql"), include_str!("../stdlib/sql.qiq")),
    ("regex.qiq", Some("std/regex"), include_str!("../stdlib/regex.qiq")),
    ("math.qiq", Some("std/math"), include_str!("../stdlib/math.qiq")),
];

/// Standard library modules that have moved, as (old path, new path), so
/// importing from an old path says where the module went.
const MOVED_STDLIB_MODULES: &[(&str, &str)] = &[("std/http", "std/net/http"), ("std/http/server", "std/net/http/server")];

/// The prefix of standard library import paths.
const STDLIB_PREFIX: &str = "std/";

/// The standard library file for an import path such as "std/file".
fn stdlib_file(import: &str) -> Option<&'static str> {
    STDLIB_MODULES
        .iter()
        .find(|(_, path, _)| *path == Some(import))
        .map(|(file, _, _)| *file)
}

/// For a name that isn't declared: if an importable standard library module
/// declares it, a hint saying how to import it.
fn stdlib_hint(name: &str) -> String {
    use std::sync::OnceLock;
    static NAMES: OnceLock<HashMap<String, &'static str>> = OnceLock::new();
    let names = NAMES.get_or_init(|| {
        let mut names = HashMap::new();
        for (_, path, source) in STDLIB_MODULES {
            let Some(path) = path else { continue };
            for line in source.lines() {
                let mut words = line.split(|c: char| !(c.is_alphanumeric() || c == '_')).filter(|word| !word.is_empty());
                let declared = loop {
                    match words.next() {
                        Some("extern" | "final" | "abstract") => continue,
                        Some("function" | "class" | "interface") => break words.next(),
                        _ => break None,
                    }
                };
                if let Some(declared) = declared.filter(|declared| !declared.starts_with("__")) {
                    names.insert(declared.to_string(), *path);
                }
            }
        }
        names
    });
    // PHP names qiq gives another name to.
    let renamed = match name {
        "password_hash" => Some("bcrypt_hash"),
        "password_verify" => Some("bcrypt_verify"),
        _ => None,
    };
    if let Some(qiq_name) = renamed {
        if let Some(path) = names.get(qiq_name) {
            return format!("; qiq's is {qiq_name}: import {qiq_name} from \"{path}\";");
        }
    }
    match names.get(name) {
        Some(path) => format!("; it's in the standard library: import {name} from \"{path}\";"),
        None => String::new(),
    }
}

fn read_stdlib(path: &Path) -> Result<String, String> {
    let relative = path.strip_prefix(STDLIB_ROOT).map_err(|_| "invalid stdlib path".to_string())?;
    if let Ok(directory) = std::env::var("QIQ_STDLIB") {
        let file = Path::new(&directory).join(relative);
        return std::fs::read_to_string(&file)
            .map_err(|error| format!("could not read {}: {error}", file.display()));
    }
    STDLIB_MODULES
        .iter()
        .find(|(name, _, _)| Path::new(name) == relative)
        .map(|(_, _, source)| source.to_string())
        .ok_or_else(|| format!("the standard library has no module {}", relative.display()))
}

/// Compiles the program whose entry file is `entry`, reading every file
/// (the entry and its imports) through `read`. Standard library modules are
/// read from the compiler itself.
fn compile_with_reader(
    entry: &Path,
    read: &dyn Fn(&Path) -> Result<String, String>,
) -> Result<String, String> {
    let read = |path: &Path| {
        if path.starts_with(STDLIB_ROOT) {
            read_stdlib(path)
        } else {
            read(path)
        }
    };
    let mut loader = Loader::new(&read);
    // How far up a qiq.mod may be: as far as the file system's root.
    let max_up = std::env::current_dir().map(|cwd| cwd.join(entry).components().count()).unwrap_or(0);
    loader.find_main_module(entry, max_up)?;
    loader.load(normalize_path(entry), None, 0)?;
    emit_llvm(&loader.into_program())
}

/// Compiles a qiq program from its entry file. Errors include the file path.
pub fn compile_file(path: &str) -> Result<String, String> {
    compile_with_reader(Path::new(path), &|path| {
        std::fs::read_to_string(path).map_err(|error| format!("could not read {}: {error}", path.display()))
    })
}

/// Compiles a qiq program from its entry file, using `source` as the entry
/// file's text instead of reading it, so an editor can check unsaved changes.
/// Imports are read from disk, relative to `path` as usual.
pub fn compile_file_with_source(path: &str, source: &str) -> Result<String, String> {
    let entry = normalize_path(Path::new(path));
    compile_with_reader(Path::new(path), &|file| {
        if normalize_path(file) == entry {
            Ok(source.to_string())
        } else {
            std::fs::read_to_string(file).map_err(|error| format!("could not read {}: {error}", file.display()))
        }
    })
}

/// Compiles a single source string. It cannot import other files.
pub fn compile_source(source: &str) -> Result<String, String> {
    compile_with_reader(Path::new(""), &|path| {
        if path.as_os_str().is_empty() {
            Ok(source.to_string())
        } else {
            Err(format!(
                "cannot import {} when compiling a source string",
                path.display()
            ))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{compile_source, compile_with_reader, lex, TokenKind};
    use std::collections::HashMap;
    use std::path::Path;

    /// The non-null entries of dispatch table `@table`, in class id order.
    /// Class ids depend on how many classes the standard library declares,
    /// so tests compare entries rather than whole tables.
    fn table_entries(ir: &str, table: &str) -> Vec<String> {
        let start = ir
            .find(&format!("@{table} = private unnamed_addr constant ["))
            .unwrap_or_else(|| panic!("no dispatch table {table}:\n{ir}"));
        let line = &ir[start..ir[start..].find('\n').map_or(ir.len(), |end| start + end)];
        let entries = &line[line.find("] [").unwrap() + 3..line.rfind(']').unwrap()];
        entries
            .split(", ")
            .filter(|entry| *entry != "ptr null")
            .map(|entry| entry.trim_start_matches("ptr ").to_string())
            .collect()
    }

    /// The IR of `main`.
    /// Whether `text` contains `prefix`, a register number, then `suffix`.
    fn register_between(text: &str, prefix: &str, suffix: &str) -> bool {
        text.match_indices(prefix).any(|(start, _)| {
            let rest = &text[start + prefix.len()..];
            let digits = rest.chars().take_while(char::is_ascii_digit).count();
            digits > 0 && rest[digits..].starts_with(suffix)
        })
    }

    fn main_ir(ir: &str) -> &str {
        &ir[ir.find("define i32 @main").unwrap()..]
    }

    /// Compiles `main.qiq` from an in-memory set of files.
    fn compile_files(files: &[(&str, &str)]) -> Result<String, String> {
        let files: HashMap<&str, &str> = files.iter().copied().collect();
        compile_with_reader(Path::new("main.qiq"), &|path| {
            let key = path.to_str().unwrap();
            files
                .get(key)
                .map(|source| source.to_string())
                .ok_or_else(|| format!("could not read {key}"))
        })
    }

    /// A main module that requires two modules replaced by local directories.
    fn module_files<'a>(main: &'a str, extra: &[(&'a str, &'a str)]) -> Vec<(&'a str, &'a str)> {
        let mut files = vec![
            (
                "qiq.mod",
                "module example.com/app\nrequire github.com/ada/web v1.2.0\nrequire github.com/lin/slug v0.3.0\nreplace github.com/ada/web => ../web\nreplace github.com/lin/slug => ../slug\n",
            ),
            ("../web/qiq.mod", "module github.com/ada/web\nrequire github.com/lin/slug v0.2.0\n"),
            (
                "../web/mux.qiq",
                "import match from \"github.com/ada/web/internal/match\"; private function clean(string p): string { return p; } class Mux { function ok(string p): bool { return match(clean(p)); } }",
            ),
            ("../web/internal/match.qiq", "function match(string p): bool { return p == \"/\"; }"),
            ("../slug/slug.qiq", "function slugify(string text): string { return text; }"),
            ("views/page.qiq", "import slugify from \"github.com/lin/slug\"; private function secret(): int { return 1; } function page(): string { return slugify(\"p\"); }"),
            ("main.qiq", main),
        ];
        files.extend_from_slice(extra);
        files
    }

    #[test]
    fn resolves_module_imports() {
        let ir = compile_files(&module_files(
            "import Mux as WebMux from \"github.com/ada/web/mux\"; import slugify from \"github.com/lin/slug\"; import page, secret from \"example.com/app/views/page\"; import page as home from \"views/page\"; WebMux m = new WebMux(); print(m->ok(\"/\"), slugify(\"x\"), page(), home(), secret());",
            &[],
        ))
        .unwrap();
        assert!(ir.contains("define i32 @main"));

        let error = |main: &str| compile_files(&module_files(main, &[])).unwrap_err();
        assert_eq!(
            error("import match from \"github.com/ada/web/internal/match\";"),
            "main.qiq:1:1: cannot import \"github.com/ada/web/internal/match\": files in an internal directory can only be imported from within github.com/ada/web"
        );
        assert_eq!(error("import clean from \"github.com/ada/web/mux\";"), "main.qiq:1:8: 'clean' is private to module github.com/ada/web");
        assert_eq!(
            error("import x from \"github.com/zed/log/x\";"),
            "main.qiq:1:1: \"github.com/zed/log/x\" isn't in a required module; add 'require <module path> <version>' to qiq.mod"
        );
        assert_eq!(
            error("import slugify from \"github.com/lin/slug\"; import page as slugify from \"views/page\";"),
            "main.qiq:1:51: 'slugify' is already imported; use 'import page as <other name>' to import it under another name"
        );

        // A dependency that needs a newer version than the main module requires.
        let mut files = module_files("import Mux from \"github.com/ada/web/mux\";", &[]);
        files[1] = ("../web/qiq.mod", "module github.com/ada/web\nrequire github.com/lin/slug v0.4.0\n");
        assert_eq!(
            compile_files(&files).unwrap_err(),
            "main.qiq:1:1: github.com/ada/web v1.2.0 requires github.com/lin/slug v0.4.0 or newer, but qiq.mod requires v0.3.0; change it to v0.4.0"
        );

        // Without a qiq.mod, private names are visible across files, and a
        // module-like path is an error.
        let ir = compile_files(&[
            ("util.qiq", "private function helper(): int { return 2; }"),
            ("main.qiq", "import helper from \"util\"; print(helper());"),
        ]);
        assert!(ir.is_ok());
        assert!(compile_files(&[("main.qiq", "import x from \"github.com/a/b/x\";")])
            .unwrap_err()
            .contains("looks like a module path, but this program has no qiq.mod"));
    }

    #[test]
    fn compiles_exec_and_shell() {
        let source = "import exec, shell, ProcessResult from \"std/os\";\n\
            ProcessResult r = exec([\"ls\", \"-l\"], cwd: \"/tmp\", env: {\"A\" => \"b\"}, timeout: 5);\n\
            print(r->exitCode, r->ok(), r->stdout, r->stderr);\n\
            try { print(shell(\"ls | wc -l\", input: \"x\")->check()->stdout); } catch (ProcessException e) { print(e->getCode()); }";
        let ir = compile_source(source).unwrap();
        assert!(ir.contains("@qiq_stdlib___process_run"));
        assert!(compile_source("import exec from \"std/os\"; exec(\"ls -l\");")
            .unwrap_err()
            .contains("expected string[]"));
    }

    #[test]
    fn compiles_regex_functions() {
        let source = "import preg_match, preg_match_all, preg_replace, preg_replace_callback, preg_split, preg_quote, preg_test, RegexMatch from \"std/regex\";\n\
            RegexMatch m = preg_match(`/(\\d+)/`, \"a1\");\n\
            if (m != null) { print(m->text, m->group(1), m->start, m->groups); }\n\
            print(len(preg_match_all(`/a/i`, \"Aa\")), preg_test(`/a/`, \"a\"), preg_quote(\"a.b\"));\n\
            print(preg_replace(`/a/`, \"$0$0\", \"a\"), preg_split(`/,/`, \"a,b\", skipEmpty: true));\n\
            print(preg_replace_callback(`/\\w/`, fn(RegexMatch x) => strtoupper(x->text), \"ab\"));\n\
            string open = `/(/`;\n\
            try { preg_match(open, \"\"); } catch (RegexException e) { print(e->getMessage()); }";
        let ir = compile_source(source).unwrap();
        assert!(ir.contains("@qiq_stdlib___regex_scan"));
        // A pattern without a subject is a type error.
        assert!(compile_source("import preg_match from \"std/regex\"; preg_match(\"a\");").is_err());
    }

    #[test]
    fn math_generics_keep_their_types() {
        let program = |body: &str| format!("import abs, min, max, clamp, number_format, number_format as nf from \"std/math\";\n{body}");
        // The result has the arguments' type, so these assignments type-check.
        let ok = program("int a = abs(-1); double b = abs(-1.5); int c = min(3, 1, 2); double d = max(1.5, 2.5); int e = min([1, 2]); double f = max([1.5]); int g = clamp(5, 0, 3); double h = clamp(0.5, 0.0, 1.0); string i = number_format(1234567) + nf(2.5, 1) + nf(3, 2, \",\", \".\");");
        let ir = compile_source(&ok).unwrap();
        let main = ir.split("define i32 @main").nth(1).unwrap();
        // Inline: no calls for abs, min, max, or clamp on values.
        for call in ["qiq.stdlib.math.abs", "qiq.stdlib.math.min(", "qiq.stdlib.math.max(", "qiq.stdlib.math.clamp("] {
            assert!(!main.contains(call), "{call}: {main}");
        }
        assert!(main.contains("@llvm.fabs.f64") && main.contains("@qiq_math_min_int(") && main.contains("@qiq_stdlib___number_format_int(") && main.contains("@qiq_stdlib_number_format("));
        let error = |body: &str| compile_source(&program(body)).unwrap_err();
        assert!(error("int x = abs(-1.5);").contains("double"));
        assert!(error("print(min(1, 2.5));").contains("min: the values must all be ints or all doubles, found int and double"));
        assert!(error("print(max(\"a\", \"b\"));").contains("max works on ints and doubles, found string"));
        assert!(error("print(min(1));").contains("min takes two or more values, or an int[] or double[], found int"));
        assert!(error("print(clamp(1, 2));").contains("clamp takes 3 values (x, low, high), found 2"));
        assert!(error("print(abs(x: 1));").contains("abs takes no named arguments"));
        // A min of your own is an ordinary function.
        assert!(compile_source("function min(string a, string b): string { return a; } print(min(\"x\", \"y\"));").is_ok());
    }

    #[test]
    fn println_is_print_with_lines() {
        let ir = compile_source("println(1, \"a\"); print(1, \"a\"); println(\"one\");").unwrap();
        let main = ir.split("define i32 @main").nth(1).unwrap();
        // Between println's values a newline; between print's a space.
        let first = main.find("@qiq_print_newline()").unwrap();
        let space = main.find("@qiq_print_space()").unwrap();
        assert!(first < space, "{main}");
        assert_eq!(compile_source("println(1)").unwrap_err(), "1:11: expected ';' after println statement");
        assert_eq!(compile_source("println 1;").unwrap_err(), "1:9: expected '(' after 'println'");
        // Whatever print accepts, println accepts (even a void call, as print does).
        for values in ["", "1", "1, 2.5, true", "[1], {\"a\" => 1}", "f()"] {
            let print = compile_source(&format!("function f(): void {{}} print({values});")).is_ok();
            let println = compile_source(&format!("function f(): void {{}} println({values});")).is_ok();
            assert_eq!(print, println, "{values}");
        }
    }

    #[test]
    fn lexes_fstrings() {
        // Text and expressions become ("text" + __interpolate(expression)).
        let kinds = |source: &str| -> Vec<String> {
            lex(source).unwrap().into_iter().map(|token| format!("{:?}", token.kind)).collect()
        };
        assert_eq!(kinds(r#"f"plain""#), ["Text(\"plain\")", "End"]);
        assert_eq!(kinds(r#"f"""#), ["Text(\"\")", "End"]);
        assert_eq!(
            kinds(r#"f"a{x}b""#),
            ["LeftParen", "Text(\"a\")", "Plus", "Identifier(\"__interpolate\")", "LeftParen", "Identifier(\"x\")", "RightParen", "Plus", "Text(\"b\")", "RightParen", "End"]
        );
        assert_eq!(
            kinds(r#"f"{x:.2f}""#),
            ["LeftParen", "Identifier(\"sprintf\")", "LeftParen", "Text(\"%.2f\")", "Comma", "Identifier(\"x\")", "RightParen", "RightParen", "End"]
        );
        // Braces, escapes, and quotes, colons, and braces inside the expression.
        assert_eq!(kinds(r#"f"{{a}} \t \"""#), ["Text(\"{a} \\t \\\"\")", "End"]);
        assert!(kinds(r#"f"{m["k"]}""#).contains(&"Text(\"k\")".to_string()));
        assert!(kinds(r#"f"{A::b}""#).contains(&"DoubleColon".to_string()));
        assert!(kinds(r#"f"{g(a: 1)}""#).contains(&"Colon".to_string()));
        assert!(kinds(r#"f"{len({"a" => 1})}""#).contains(&"Text(\"a\")".to_string()));
        // An identifier f followed by a space isn't an f-string.
        assert!(kinds(r#"f "x""#).starts_with(&["Identifier(\"f\")".to_string()]));

        let error = |source: &str| compile_source(source).unwrap_err();
        assert_eq!(error(r#"string s = f"{";"#), "1:14: '{' in an f-string without a closing '}'");
        assert_eq!(error(r#"string s = f"{x"#), "1:14: '{' in an f-string without a closing '}'");
        assert_eq!(error(r#"string s = f"a } b";"#), "1:16: single '}' in an f-string; write '}}' for a brace");
        assert_eq!(error(r#"string s = f"{ }";"#), "1:14: empty expression in an f-string: write {name}, or {{}} for braces");
        assert!(error(r#"string s = f"{1:,d}";"#).starts_with("1:14: invalid format spec ',d' in an f-string"));
        assert!(error(r#"string s = f"{1:%d}";"#).starts_with("1:14: invalid format spec '%d'"));
        assert_eq!(error(r#"string s = f"\q";"#), "1:14: unsupported string escape \\q");
        // Errors inside the expression point into it.
        assert_eq!(error(r#"int a = 1; string s = f"x {a @ 2}";"#), "1:30: unexpected character '@'");
        assert!(error(r#"string s = f"{[1, 2]}";"#).contains("an f-string can't include a value of type int[]"));
        assert!(error("class A {} A a = new A(); string s = f\"{a}\";").contains("unless its class declares `public function __toString(): string`"));
        assert!(error(r#"string s = f"{undeclared}";"#).contains("variable 'undeclared' is not declared"));
        // A format spec goes through sprintf's own checks.
        assert!(error(r#"string s = f"{1:q}";"#).starts_with("1:1: sprintf"), "{}", error(r#"string s = f"{1:q}";"#));
    }

    #[test]
    fn checks_regex_literals_at_compile_time() {
        let error = |call: &str| {
            compile_source(&format!("import preg_match, preg_match_all, preg_test, preg_replace, preg_split from \"std/regex\";\n{call};"))
                .unwrap_err()
        };
        assert_eq!(error("preg_match(`/(\\d+/`, \"x\")"), "2:1: invalid regular expression /(\\d+/: missing closing parenthesis at offset 4");
        assert_eq!(error("preg_test(`/x/q`, \"x\")"), "2:1: unknown flag 'q' in /x/q; the flags are i, m, s, x, u, U, D, A, n, and J");
        assert_eq!(error("preg_split(`abc`, \"x\")"), "2:1: the delimiter can't be a letter, digit, or backslash: write the pattern between delimiters, as in \"/abc/\"");
        assert_eq!(error("preg_match_all(`/abc`, \"x\")"), "2:1: no ending delimiter '/' in /abc");
        assert_eq!(error("preg_match(``, \"x\")"), "2:1: empty regular expression; write it between delimiters, as in `/\\d+/`");
        // Offsets count characters, before and in the pattern.
        assert_eq!(error("preg_match(`/é[z-a]/`, \"x\")"), "2:1: invalid regular expression /é[z-a]/: range out of order in character class at offset 5");
        // Named arguments, and the replacement of preg_replace.
        assert_eq!(error("preg_match(subject: \"x\", pattern: `/[/`)").contains("missing terminating ]"), true);
        assert_eq!(error("preg_replace(`/(a)(b)/`, \"$3\", \"ab\")"), "2:1: the replacement refers to group 3, but the pattern has 2 group(s)");
        assert_eq!(error("preg_replace(`/(?<x>a)/`, \"${y}\", \"a\")"), "2:1: the replacement refers to a group named 'y', which the pattern doesn't have");
        assert_eq!(error("preg_replace(`/a/`, \"${0\", \"a\")"), "2:1: the replacement has a '${' without a closing '}'");

        // Valid patterns and replacements, including ones the checks must
        // not misread.
        let fine = "import preg_match, preg_replace from \"std/regex\";\n\
            print(preg_match(`#^/a/(\\d+)$#i`, \"/a/1\"), preg_match(`{a{2}}`, \"aa\"), preg_match(`/\\//`, \"/\"));\n\
            print(preg_replace(`/(?<x>a)(b)/`, \"$$1 $0 $2 \\\\1 ${2} ${x} $12\", \"ab\"));";
        let fine = fine.replace("$12", "$1");
        assert!(compile_source(&fine).is_ok(), "{:?}", compile_source(&fine).err());
        // Patterns that aren't literals are checked when the program runs;
        // a function of your own named preg_match isn't checked at all.
        assert!(compile_source("import preg_match from \"std/regex\";\nstring p = `/(/`; print(preg_match(p, \"x\") == null);").is_ok());
        assert!(compile_source("function preg_match(string a, string b): bool { return true; } print(preg_match(`/(/`, \"x\"));").is_ok());
    }

    #[test]
    fn lexes_raw_strings() {
        let text = |source: &str| match lex(source).unwrap().into_iter().next().unwrap().kind {
            TokenKind::Text(value) => value,
            other => panic!("{other:?}"),
        };
        assert_eq!(text(r"`\d+\.\n`"), r"\d+\.\n");
        assert_eq!(text("`\"quoted\" and 'c' // not a comment /* nor this */`"), "\"quoted\" and 'c' // not a comment /* nor this */");
        assert_eq!(text("`two\nlines`"), "two\nlines");
        assert_eq!(text("``"), "");

        // Lines and columns after a multi-line raw string are still right.
        let after = compile_source("string s = `a\nb`; int x = ;").unwrap_err();
        assert!(after.starts_with("2:13:"), "{after}");
        let unterminated = compile_source("int x = 1;\nstring s = `never closed;").unwrap_err();
        assert!(unterminated.contains("2:12: unterminated raw string"), "{unterminated}");
        assert!(compile_source("string s = `C:\\Users\\ada` + \"\\n\"; print(s);").is_ok());
    }

    #[test]
    fn skips_block_comments() {
        let source = "/* a\n   block comment */ int x = 1 /* inline */ + 2; /** doc */ print(x); /* no nesting: /* */ // done";
        assert!(compile_source(source).is_ok());

        // Lines and columns after a multi-line comment are still right.
        let after = compile_source("/*\n\n*/ int x = ;").unwrap_err();
        assert!(after.starts_with("3:12:"), "{after}");

        let unterminated = compile_source("int x = 1;\n  /* never closed\nprint(x);").unwrap_err();
        assert!(unterminated.contains("2:3: unterminated comment"), "{unterminated}");

        // `/*` inside a string is not a comment, and `*/` alone is two operators.
        assert!(compile_source("string s = \"/* not a comment\"; print(s);").is_ok());
        assert!(compile_source("int x = 2 */ 1;").is_err());
    }

    #[test]
    fn compiles_function_return_into_variable() {
        let source = "function add(int a, int b): int { return a + b; } int result = add(5, 2); print(result);";
        let ir = compile_source(source).unwrap();
        assert!(ir.contains("define i64 @add(i64 %arg0, i64 %arg1)"));
        assert!(ir.contains("%v0 = call i64 @add(i64 5, i64 2)"));
        assert!(ir.contains("store i64 %arg0, ptr %v0"));
        assert!(ir.contains("%v4 = add i64 %v2, %v3\n  ret i64 %v4"));
    }

    #[test]
    fn supports_void_call_and_void_return() {
        let source = "function announce(string message): void { print(message); return; } announce(\"hello\");";
        let ir = compile_source(source).unwrap();
        assert!(ir.contains("define void @announce(ptr %arg0)"));
        assert!(ir.contains("call void @announce(ptr getelementptr"));
        assert!(ir.contains("ret void"));
    }

    #[test]
    fn checks_call_argument_types_and_return_types() {
        let wrong_argument = "function get(int n): int { return n; } int result = get(true);";
        assert!(compile_source(wrong_argument)
            .unwrap_err()
            .contains("argument type mismatch calling 'get': expected int, found bool"));

        let wrong_return = "function get(): int { return true; }";
        assert!(compile_source(wrong_return)
            .unwrap_err()
            .contains("return type mismatch: expected int, found bool"));
    }

    #[test]
    fn rejects_missing_non_void_return_and_value_use_of_void() {
        let missing_return = "function get(): int { int n = 2; }";
        assert!(compile_source(missing_return)
            .unwrap_err()
            .contains("non-void function must return a value"));

        let void_value = "function act(): void {} int n = act();";
        assert!(compile_source(void_value)
            .unwrap_err()
            .contains("cannot initialize int variable 'n' with null"));
    }

    #[test]
    fn compiles_numeric_and_string_if_else_comparisons() {
        let source = "int n = 3; if (n > 2) { print(\"yes\"); } else { print(\"no\"); } if (\"apple\" < \"banana\") { print(n); }";
        let ir = compile_source(source).unwrap();
        assert!(ir.contains("store i64 3, ptr %v0"));
        assert!(ir.contains("%v1 = load i64, ptr %v0, align 8\n  %v2 = icmp sgt i64 %v1, 2"));
        assert!(ir.contains("call i32 @qiq_string_compare(ptr getelementptr"));
        // The comparison returns a C int, compared with 0.
        assert!(ir.contains("icmp slt i32 %v"));
        assert!(ir.contains("br i1 %v"));
        assert!(ir.contains("if.then0:"));
        assert!(ir.contains("if.else0:"));
    }

    #[test]
    fn both_if_else_branches_can_return() {
        let source = "function max(int a, int b): int { if (a > b) { return a; } else { return b; } } int result = max(8, 3); print(result);";
        let ir = compile_source(source).unwrap();
        assert!(ir.contains("define i64 @max(i64 %arg0, i64 %arg1)"));
        assert!(ir.contains("%v5 = load i64, ptr %v0, align 8\n  ret i64 %v5"));
        assert!(ir.contains("%v6 = load i64, ptr %v1, align 8\n  ret i64 %v6"));
        assert!(!ir.contains("if.end0:"));
    }

    #[test]
    fn non_void_function_requires_both_conditional_paths_to_return() {
        let source = "function choose(bool condition): int { if (condition == true) { return 1; } else { print(0); } }";
        assert!(compile_source(source)
            .unwrap_err()
            .contains("non-void function must return a value on every path"));
    }

    #[test]
    fn supports_unicode_characters_in_print_and_functions() {
        let source = "function identity(char value): char { return value; } char letter = identity('é'); print(letter); if (letter == 'é') { print('界'); }";
        let ir = compile_source(source).unwrap();
        assert!(ir.contains("define ptr @identity(ptr %arg0)"));
        assert!(ir.contains("call ptr @identity(ptr getelementptr"));
        assert!(ir.contains("c\"\\C3\\A9\\00\""));
        assert!(ir.contains("c\"\\E7\\95\\8C\\00\""));
        assert!(ir.contains("call i32 @qiq_string_equality(ptr"));
    }

    #[test]
    fn supports_escaped_char_literals() {
        let source = "char newline = '\\n'; char quote = '\\''; print(newline); print(quote);";
        let ir = compile_source(source).unwrap();
        assert!(ir.contains("c\"\\0A\\00\""));
        assert!(ir.contains("c\"'\\00\""));
    }

    #[test]
    fn rejects_empty_and_multiple_scalar_char_literals() {
        assert!(compile_source("char empty = ''; ")
            .unwrap_err()
            .contains("char literal must contain exactly one character"));
        assert!(compile_source("char many = 'ab';")
            .unwrap_err()
            .contains("char literal must contain exactly one character"));
    }

    #[test]
    fn compiles_all_int_array_declaration_forms() {
        let source = "int[] reference; int[] values = [1, 2, 3]; int size = 5; int[] zeroes = new int[size];";
        let ir = compile_source(source).unwrap();
        // Literals and `new type[size]` both allocate a heap array with a length header.
        assert!(ir.contains("call ptr @qiq_array_new(i64 3, i64 8, i32 "), "{ir}");
        // The literal's elements start 8 bytes in, after the length.
        let literal = main_ir(&ir).split("call ptr @qiq_array_new(i64 3").next().unwrap().rsplit("  ").next().unwrap().trim_end_matches(" = ").to_string();
        assert!(ir.contains(&format!("getelementptr inbounds i8, ptr {literal}, i64 8")), "{ir}");
        assert!(ir.contains("store i64 1, ptr"));
        assert!(ir.contains("store i64 2, ptr"));
        assert!(ir.contains("store i64 3, ptr"));
        assert!(ir.contains("store i64 5, ptr"));
        assert!(ir.contains("call ptr @qiq_array_new(i64 %v"));
        assert!(!main_ir(&ir).contains("alloca ["));
        assert!(ir.contains("ret i64 0"));
    }

    #[test]
    fn rejects_non_int_array_elements_and_sizes() {
        let bad_element = "int[] values = [1, true];";
        assert!(compile_source(bad_element)
            .unwrap_err()
            .contains("array elements must all be int, found bool"));

        let bad_size = "bool size = true; int[] values = new int[size];";
        assert!(compile_source(bad_size)
            .unwrap_err()
            .contains("array size must be an int"));
    }

    #[test]
    fn supports_arrays_of_all_value_types_and_indexed_access() {
        let source = "int[] ints = [1]; double[] decimals = [1.5]; bool[] flags = [true]; string[] words = [\"hello\"]; char[] letters = ['é']; ints[0] = ints[0] + 1; decimals[0] = 2.5; flags[0] = false; words[0] = \"world\"; letters[0] = '界'; print(ints[0]); print(decimals[0]); print(flags[0]); print(words[0]); print(letters[0]);";
        let ir = compile_source(source).unwrap();
        // Element sizes: int 4, double 8, bool 1, string and char 8.
        assert!(ir.contains("call ptr @qiq_array_new(i64 1, i64 8, i32 "));
        assert!(ir.contains("call ptr @qiq_array_new(i64 1, i64 8, i32 "));
        assert!(ir.contains("call ptr @qiq_array_new(i64 1, i64 1, i32 "));
        // Every indexed read and write is bounds-checked: 11 in the source.
        assert_eq!(ir.matches("call void @qiq_array_check_index(").count(), 11, "{ir}");
        assert!(ir.contains("load i64, ptr"));
        assert!(ir.contains("load double, ptr"));
        assert!(ir.contains("load i1, ptr"));
        assert!(ir.contains("store ptr getelementptr"));
    }

    #[test]
    fn rejects_wrong_array_index_and_assigned_value_types() {
        assert!(compile_source("int[] values = [1]; values[true] = 2;")
            .unwrap_err()
            .contains("array index must be an int"));
        assert!(compile_source("double[] values = [1.0]; values[0] = 2;")
            .unwrap_err()
            .contains("array element assignment requires double, found int"));
        assert!(compile_source("int[] values = [1]; bool result = values[0];")
            .unwrap_err()
            .contains("cannot initialize bool variable 'result' with int"));
    }

    #[test]
    fn passes_arrays_to_functions_for_indexed_reads() {
        let source = "function first(double[] values): double { return values[0]; } double[] data = [2.5]; double result = first(data); print(result);";
        let ir = compile_source(source).unwrap();
        assert!(ir.contains("define double @first(ptr %arg0)"));
        assert!(ir.contains("call void @qiq_array_check_index(ptr %arg0, i64 0)"), "{ir}");
        assert!(ir.contains("load double, ptr"));
        assert!(ir.contains("call double @first(ptr %v"));
    }

    #[test]
    fn supports_typed_map_initializers_updates_and_lookups() {
        let source = "map[string, string] words = {\"hello\" => \"World\"}; words[\"hello\"] = \"updated\"; string result = words[\"hello\"]; print(result);";
        let ir = compile_source(source).unwrap();
        assert!(ir.contains("call ptr @qiq_map_new(i32 3, i32 1)"), "{ir}");
        assert!(ir.contains("call void @qiq_map_set(ptr"));
        assert!(ir.contains("call i64 @qiq_map_get(ptr"));
        assert!(ir.contains("inttoptr i64"));
    }

    #[test]
    fn rejects_map_key_and_value_type_mismatches() {
        assert!(compile_source("map[string, int] values = {true => 1};")
            .unwrap_err()
            .contains("map key requires string, found bool"));
        assert!(compile_source("map[string, int] values = {\"key\" => true};")
            .unwrap_err()
            .contains("map value requires int, found bool"));
        assert!(compile_source("map[string, int] values; values[\"key\"] = true;")
            .unwrap_err()
            .contains("map value requires int, found bool"));
    }

    #[test]
    fn supports_nested_map_values_and_array_values() {
        let source = "map[string, int] inner = {\"n\" => 3}; map[string, map[string, int]] outer = {\"inner\" => inner}; int[] items = [4]; map[int, int[]] arrays = {1 => items}; int result = outer[\"inner\"][\"n\"] + arrays[1][0];";
        let ir = compile_source(source).unwrap();
        assert!(ir.matches("call ptr @qiq_map_new").count() == 3);
        assert!(ir.contains("call i64 @qiq_map_get"));
    }

    #[test]
    fn map_types_are_stable_across_function_signatures() {
        let source = "function echo(map[string, int] values): map[string, int] { return values; } map[string, int] original = {\"n\" => 1}; map[string, int] copied = echo(original);";
        let ir = compile_source(source).unwrap();
        assert!(ir.contains("define ptr @echo(ptr %arg0)"));
        assert!(ir.contains("call ptr @echo(ptr"));
    }

    #[test]
    fn emits_foreach_loops_for_arrays_strings_and_maps() {
        let source = "int[] items = [4, 5]; foreach(items as index => value) { print(index); print(value); } string text = \"é\"; foreach(text as position => character) { print(position); print(character); } map[string, int] scores = {\"Ada\" => 10}; foreach(scores as name => score) { print(name); print(score); }";
        let ir = compile_source(source).unwrap();
        assert!(ir.contains("call i64 @qiq_array_length(ptr %v"));
        assert!(ir.contains("foreach.test1:"));
        assert!(ir.contains("call i64 @qiq_string_len(ptr"));
        assert!(ir.contains("call ptr @qiq_string_char_at(ptr"));
        assert!(ir.contains("call i64 @qiq_map_key_at(ptr"));
        assert!(ir.contains("call i64 @qiq_map_value_at(ptr"));
    }

    #[test]
    fn iterates_array_parameters_using_their_stored_length() {
        let source = "function show(int[] values): void { foreach(values as value) { print(value); } } int[] items = [1, 2]; show(items);";
        let ir = compile_source(source).unwrap();
        assert!(ir.contains("define void @show(ptr %arg0)"));
        assert!(ir.contains("call i64 @qiq_array_length(ptr %arg0)"), "{ir}");
        assert!(ir.contains("call void @show(ptr %v"));
        assert!(ir.contains("icmp ult i64 %v"));
    }

    #[test]
    fn rejects_foreach_over_unsupported_values() {
        assert!(compile_source("int value = 2; foreach(value as item) { print(item); }")
            .unwrap_err()
            .contains("foreach requires an array, map, or string"));
    }

    #[test]
    fn break_and_continue_are_limited_to_foreach_loops() {
        assert!(compile_source("break;")
            .unwrap_err()
            .contains("break is only valid inside a loop"));
        assert!(compile_source("continue;")
            .unwrap_err()
            .contains("continue is only valid inside a loop"));
    }

    #[test]
    fn nested_loop_control_targets_the_innermost_loop() {
        let source = "int[] items = [1, 2, 3]; foreach(items as outer) { foreach(items as inner) { if (inner == 1) { continue; } if (inner == 2) { break; } print(outer); } }";
        let ir = compile_source(source).unwrap();
        // Each foreach's length check takes a label id first: outer loop 1, inner loop 3.
        assert!(ir.contains("br label %foreach.increment3"), "{ir}");
        assert!(ir.contains("br label %foreach.end3"));
        assert!(ir.contains("foreach.increment1:"));
        assert!(ir.contains("foreach.end1:"));
    }

    #[test]
    fn foreach_bindings_use_regular_identifiers() {
        assert!(compile_source("int[] values = [1]; foreach(values as $item) { print($item); }")
            .unwrap_err()
            .contains("unexpected character '$'"));
    }

    #[test]
    fn compiles_class_constructor_properties_and_methods() {
        let source = "class Person { private string name; function __construct(string name) { this->name = name; } function getName(): string { return this->name; } } Person person = new Person(\"Alias\"); string name = person->getName(); print(name);";
        let ir = compile_source(source).unwrap();
        assert!(ir.contains("define void @qiq_Person___construct(ptr %this, ptr %arg0)"));
        assert!(ir.contains("define ptr @qiq_Person_getName(ptr %this)"));
        assert!(ir.contains("call void @qiq_Person___construct(ptr"));
        assert!(ir.contains("call ptr @qiq_Person_getName(ptr"));
        assert!(ir.contains("getelementptr inbounds i64, ptr"));
    }

    #[test]
    fn enforces_private_and_protected_access_defaults() {
        let private_property = "class Secret { string value; } Secret secret = new Secret(); print(secret->value);";
        assert!(compile_source(private_property)
            .unwrap_err()
            .contains("cannot access non-public member 'value'"));

        let private_method = "class Secret { private function reveal(): string { return \"hidden\"; } } Secret secret = new Secret(); print(secret->reveal());";
        assert!(compile_source(private_method)
            .unwrap_err()
            .contains("cannot access non-public member 'reveal'"));

        let class_access = "class Secret { protected string value; function read(): string { return this->value; } }";
        assert!(compile_source(class_access).is_ok());
    }

    #[test]
    fn checks_constructor_arguments_and_class_method_visibility() {
        let wrong_constructor = "class Person { function __construct(int age) {} } Person p = new Person(\"wrong\");";
        assert!(compile_source(wrong_constructor)
            .unwrap_err()
            .contains("constructor argument mismatch: expected int, found string"));

        let private_method = "class Person { private function age(): int { return 1; } } Person p = new Person(); int age = p->age();";
        assert!(compile_source(private_method)
            .unwrap_err()
            .contains("cannot access non-public member 'age'"));

        let private_constructor = "class Secret { private function __construct() {} } Secret secret = new Secret();";
        assert!(compile_source(private_constructor)
            .unwrap_err()
            .contains("cannot access non-public member 'Secret'"));
    }

    #[test]
    fn supports_inheritance_parent_constructor_and_protected_members() {
        let source = "class Person { protected string name; function __construct(string name) { this->name = name; } function getName(): string { return this->name; } } class Employee extends Person { private int id; function __construct(string name, int id) { parent::__construct(name); this->id = id; } function getId(): int { return this->id; } function inheritedName(): string { return this->name; } } Employee employee = new Employee(\"Ada\", 7); Person person = employee; print(employee->getName()); print(employee->getId()); print(person->getName()); print(employee->inheritedName());";
        let ir = compile_source(source).unwrap();
        assert!(ir.contains("define void @qiq_Employee___construct(ptr %this, ptr %arg0, i64 %arg1)"));
        assert!(ir.contains("call void @qiq_Person___construct(ptr %this, ptr %arg0)"), "{ir}");
        assert!(ir.contains("getelementptr inbounds i64, ptr %this, i64 2"));
        assert!(ir.contains("call ptr @qiq_Person_getName(ptr"));
    }

    #[test]
    fn inheritance_respects_private_access_and_parent_call_context() {
        let private_parent_member = "class Base { private int value; } class Child extends Base { function read(): int { return this->value; } }";
        assert!(compile_source(private_parent_member)
            .unwrap_err()
            .contains("cannot access non-public member 'value'"));

        assert!(compile_source("parent::__construct();")
            .unwrap_err()
            .contains("parent:: can only be used inside a class"));
        assert!(compile_source("class A { function __construct() { parent::__construct(); } }")
            .unwrap_err()
            .contains("parent:: requires a class that extends another"));
        assert!(compile_source("class A { function __construct() {} } class B extends A { function m(): void { parent::__construct(); } }")
            .unwrap_err()
            .contains("parent::__construct can only be called from a constructor"));
        assert!(compile_source("class A {} class B extends A { function __construct() { parent(); } }")
            .unwrap_err()
            .contains("write parent::__construct(...) to call the parent constructor"));

        assert!(compile_source("class Base {} class Child extends Base { function __construct() { parent::__construct(); } }")
            .is_ok());
    }

    #[test]
    fn requires_superclass_to_be_declared_before_subclass() {
        assert!(compile_source("class Child extends Base {} class Base {}")
            .unwrap_err()
            .contains("parent class 'Base' must be declared first"));
    }

    #[test]
    fn supports_abstract_class_and_protected_abstract_method() {
        let source = "abstract class AbstractClass { abstract protected function getValue(); } class ConcreteClass extends AbstractClass { protected function getValue(): void { print(7); } public function display(): void { this->getValue(); } } ConcreteClass value = new ConcreteClass(); value->display();";
        let ir = compile_source(source).unwrap();
        assert!(ir.contains("define void @qiq_ConcreteClass_getValue(ptr %this)"));
        assert!(!ir.contains("@qiq_AbstractClass_getValue"));
        assert!(ir.contains("call void @qiq_ConcreteClass_getValue(ptr %this)"));
    }

    #[test]
    fn rejects_instantiating_abstract_classes_and_incomplete_subclasses() {
        let abstract_instance = "abstract class Base { abstract protected function run(); } Base value = new Base();";
        assert!(compile_source(abstract_instance)
            .unwrap_err()
            .contains("cannot instantiate abstract class 'Base'"));

        let missing_override = "abstract class Base { abstract protected function run(); } class Child extends Base {}";
        assert!(compile_source(missing_override)
            .unwrap_err()
            .contains("must implement abstract method 'run'"));

        let concrete_abstract_member = "class Base { abstract protected function run(); }";
        assert!(compile_source(concrete_abstract_member)
            .unwrap_err()
            .contains("must be abstract because method 'run' is abstract"));
    }

    #[test]
    fn abstract_method_overrides_must_match_signature_and_visibility() {
        let wrong_signature = "abstract class Base { abstract protected function run(int value); } class Child extends Base { protected function run(string value): void {} }";
        assert!(compile_source(wrong_signature)
            .unwrap_err()
            .contains("does not match the inherited signature"));

        let reduced_visibility = "abstract class Base { abstract protected function run(); } class Child extends Base { private function run(): void {} }";
        assert!(compile_source(reduced_visibility)
            .unwrap_err()
            .contains("cannot reduce inherited visibility"));
    }
    #[test]
    fn supports_interfaces_and_interface_typed_values() {
        let source = "interface Template { function setVariable(string name, string var); function getHtml(string template): string; } class Page implements Template { function setVariable(string name, string var): void { print(var); } function getHtml(string template): string { return template; } } function show(Template template): void { print(1); } Page page = new Page(); page->setVariable(\"a\", \"b\"); print(page->getHtml(\"<p></p>\")); show(page); Template template = page;";
        let ir = compile_source(source).unwrap();
        assert!(ir.contains("@qiq_Page_getHtml"));
        assert!(!ir.contains("@qiq_Template_getHtml"));
    }

    #[test]
    fn supports_multiple_inherited_and_extended_interfaces() {
        let multiple = "interface A { function a(); } interface B { function b(): int; } class C implements A, B { function a(): void {} function b(): int { return 1; } }";
        assert!(compile_source(multiple).is_ok());
        let extended = "interface A { function a(); } interface B extends A { function b(); } class C implements B { function a(): void {} function b(): void {} } A value = new C();";
        assert!(compile_source(extended).is_ok());
        let inherited = "interface A { function a(); } class Base { function a(): void {} } class Child extends Base implements A {} A value = new Child();";
        assert!(compile_source(inherited).is_ok());
        let deferred = "interface A { function a(); } abstract class Base implements A {} class Child extends Base { function a(): void {} } A value = new Child();";
        assert!(compile_source(deferred).is_ok());
    }

    #[test]
    fn rejects_incomplete_or_mismatched_interface_implementations() {
        let missing = "interface A { function a(); } class C implements A {}";
        assert!(compile_source(missing)
            .unwrap_err()
            .contains("class 'C' must implement method 'a' from interface 'A'"));
        let missing_via_abstract = "interface A { function a(); } abstract class Base implements A {} class Child extends Base {}";
        assert!(compile_source(missing_via_abstract)
            .unwrap_err()
            .contains("class 'Child' must implement method 'a' from interface 'A'"));
        let wrong_signature = "interface A { function a(int value); } class C implements A { function a(string value): void {} }";
        assert!(compile_source(wrong_signature)
            .unwrap_err()
            .contains("does not match the signature declared by interface 'A'"));
        let not_public = "interface A { function a(); } class C implements A { protected function a(): void {} }";
        assert!(compile_source(not_public)
            .unwrap_err()
            .contains("must be public to implement interface 'A'"));
    }

    #[test]
    fn rejects_invalid_interface_declarations_and_uses() {
        let cases = [
            ("interface A { int value; }", "interfaces can only declare method signatures and constants"),
            ("interface A { function a() {} }", "expected ';' after abstract method signature"),
            ("interface A { private function a(); }", "interface methods must be public"),
            ("interface A { function __construct(); }", "interfaces cannot declare constructors"),
            ("interface A {} class C extends A {}", "cannot extend interface 'A'; use implements"),
            ("class B {} class C implements B {}", "'B' is a class, not an interface"),
            ("class C implements A {} interface A {}", "interface 'A' must be declared first"),
            ("interface A {} A value = new A();", "cannot instantiate interface 'A'"),
        ];
        for (source, message) in cases {
            let error = compile_source(source).unwrap_err();
            assert!(error.contains(message), "{source}: expected '{message}', got '{error}'");
        }
    }
    #[test]
    fn objects_store_their_class_id_in_a_header_slot() {
        let source = "class A {} class B { private int value; function __construct() { this->value = 3; } } B b = new B();";
        let ir = compile_source(source).unwrap();
        let class_b = ir.find("c\"B\\00\"").and_then(|_| {
            // @.class.name<id> holds "B".
            let line = ir.lines().find(|line| line.contains("c\"B\\00\""))?;
            line.strip_prefix("@.class.name")?.split(' ').next()?.parse::<u32>().ok()
        });
        let class_b = class_b.expect("class B has an id");
        assert!(ir.contains(&format!("call nonnull ptr @qiq_object_new(i64 2, i64 {class_b})")), "{ir}");
        assert!(ir.contains("getelementptr inbounds i64, ptr %this, i64 1"));
    }

    #[test]
    fn interface_and_abstract_calls_use_dispatch_tables() {
        let interface_call = "interface A { function a(): int; } class X implements A { function a(): int { return 1; } } class Y implements A { function a(): int { return 2; } } function run(A value): void { print(value->a()); } run(new X()); run(new Y());";
        let ir = compile_source(interface_call).unwrap();
        assert_eq!(table_entries(&ir, "qiq_dispatch_A_a"), ["@qiq_X_a", "@qiq_Y_a"]);
        assert!(ir.contains("getelementptr inbounds ptr, ptr @qiq_dispatch_A_a, i64"));

        let abstract_call = "abstract class Base { abstract protected function name(): string; function show(): void { print(this->name()); } } class Child extends Base { protected function name(): string { return \"child\"; } } Child child = new Child(); child->show();";
        let ir = compile_source(abstract_call).unwrap();
        assert_eq!(table_entries(&ir, "qiq_dispatch_Base_name"), ["@qiq_Child_name"]);

        let inherited = "interface A { function a(); } class Base { function a(): void {} } class Child extends Base implements A {} function run(A value): void { value->a(); } run(new Child());";
        let ir = compile_source(inherited).unwrap();
        // Base doesn't implement A, so only Child's slot is filled (with Base's method).
        assert_eq!(table_entries(&ir, "qiq_dispatch_A_a"), ["@qiq_Base_a"]);
    }

    #[test]
    fn concrete_calls_stay_direct() {
        let source = "interface A { function a(); } class X implements A { function a(): void {} } X x = new X(); x->a();";
        let ir = compile_source(source).unwrap();
        assert!(ir.contains("call void @qiq_X_a(ptr"));
        assert!(!ir.contains("qiq_dispatch"));
    }
    #[test]
    fn scalar_variables_and_parameters_can_be_reassigned() {
        let source = "int n = 1; n = 2; double d = 1.5; d = d + 1.0; bool b = true; b = false; string s = \"a\"; s = \"b\"; char c = 'x'; c = 'y'; function bump(int value): int { value = value + 1; return value; } n = bump(n); print(n);";
        let ir = compile_source(source).unwrap();
        assert!(ir.contains("store i64 2, ptr %v0"));
        assert!(ir.contains("store i64 %arg0, ptr %v0"));
        assert!(ir.contains("store i1 false, ptr"));
    }

    #[test]
    fn reassignment_inside_blocks_updates_the_outer_variable() {
        let source = "int total = 0; int[] values = [1, 2, 3]; foreach(values as value) { total = total + value; } if (total > 5) { total = 100; } print(total);";
        let ir = compile_source(source).unwrap();
        // One slot for `total` (the foreach counter is also an i64 slot).
        let slots = main_ir(&ir).lines().filter(|line| line.contains("= alloca i64") && !line.contains("foreach.slot")).count();
        assert_eq!(slots, 1, "slot allocated once:\n{ir}");
        assert!(ir.contains("define i32 @main(i32 %argc, ptr %argv) {\nentry:\n  call void @qiq_set_arguments(i32 %argc, ptr %argv)\n  %v0 = alloca i64"));
    }

    #[test]
    fn rejects_invalid_reassignments() {
        let cases = [
            ("n = n + 1;", "variable 'n' is not declared"),
            ("int n = 1; n = \"text\";", "cannot assign string to int variable 'n'"),
            ("int n = 1; n = 1.5;", "cannot assign double to int variable 'n'"),
            ("int[] values = [1]; foreach(values as value) { value = 2; }", "foreach and catch variable 'value' cannot be reassigned"),
        ];
        for (source, message) in cases {
            let error = compile_source(source).unwrap_err();
            assert!(error.contains(message), "{source}: expected '{message}', got '{error}'");
        }
    }

    #[test]
    fn assignment_to_undeclared_name_infers_its_type() {
        let source = "num = 123; num = num + 1; ratio = 1.5; ready = true; name = \"qiq\"; letter = 'q'; print(num);";
        let ir = compile_source(source).unwrap();
        assert!(ir.contains("%v0 = alloca i64"));
        assert!(ir.contains("alloca double"));
        assert!(ir.contains("alloca i1"));
        assert_eq!(main_ir(&ir).matches("alloca ptr").count(), 2);

        let objects = "class Point { function x(): int { return 4; } } point = new Point(); print(point->x()); function make(): int { local = 2; return local; } made = make();";
        assert!(compile_source(objects).is_ok());

        let cases = [
            ("num = 123; num = \"text\";", "cannot assign string to int variable 'num'"),
            ("int flag = 1; if (flag > 0) { inner = 5; } print(inner);", "variable 'inner' is not declared"),
        ];
        for (source, message) in cases {
            let error = compile_source(source).unwrap_err();
            assert!(error.contains(message), "{source}: expected '{message}', got '{error}'");
        }
    }

    #[test]
    fn void_calls_evaluate_to_null() {
        let source = "function act(): void {} class Logger { function log(): void {} } act(); result = act(); print(result); print(act()); if (result == null) { print(1); } Logger logger = new Logger(); logged = logger->log(); result = act();";
        let ir = compile_source(source).unwrap();
        assert!(ir.contains("@.str.null = private unnamed_addr constant [5 x i8] c\"null\\00\""));
        assert!(ir.contains("call i32 (ptr, ...) @printf(ptr @.fmt.string, ptr @.str.null)"));
        assert!(ir.contains("store ptr null, ptr %v0"));
        assert!(ir.contains("icmp eq ptr"));
    }

    #[test]
    fn void_functions_may_return_null_explicitly() {
        let source = "function act(): void {} function early(int n): void { if (n > 0) { return null; } return act(); }";
        let ir = compile_source(source).unwrap();
        assert!(ir.contains("define void @early(i64 %arg0)"));
        assert!(compile_source("function act(): void { return 1; }")
            .unwrap_err()
            .contains("void function cannot return a value"));
    }

    #[test]
    fn null_is_not_assignable_to_other_types() {
        let cases = [
            ("function act(): void {} string s = act();", "cannot initialize string variable 's' with null"),
            ("function f(int n): void {} function act(): void {} f(act());", "expected int, found null"),
            ("function get(): int { return null; }", "return type mismatch: expected int, found null"),
            ("function act(): void {} x = 5; if (x == act()) { print(1); }", "comparison requires matching types, found int and null"),
            ("function act(): void {} if (act() < null) { print(1); }", "null values support only == and != comparisons"),
            ("function act(): void {} n = act(); n = 5;", "cannot assign int to null variable 'n'"),
            ("x = 5; x;", "only a function call can be used as a statement"),
        ];
        for (source, message) in cases {
            let error = compile_source(source).unwrap_err();
            assert!(error.contains(message), "{source}: expected '{message}', got '{error}'");
        }
    }

    #[test]
    fn imports_functions_classes_and_interfaces_across_files() {
        let ir = compile_files(&[
            ("main.qiq", "import Shape from \"shapes/shape\"; import Square from \"shapes/square\"; import area from \"util\"; Square square = new Square(3); print(area(square));"),
            ("shapes/shape.qiq", "interface Shape { function area(): int; }"),
            ("shapes/square.qiq", "import Shape from \"shape\"; class Square implements Shape { private int side; function __construct(int side) { this->side = side; } function area(): int { return this->side + this->side; } }"),
            ("util.qiq", "import Shape from \"./shapes/../shapes/shape\"; function area(Shape shape): int { return shape->area(); }"),
        ])
        .unwrap();
        // shapes/shape.qiq is reached through three different paths but loaded once.
        assert_eq!(ir.matches("@qiq_dispatch_shape.1.Shape_area = ").count(), 1);
        assert_eq!(table_entries(&ir, "qiq_dispatch_shape.1.Shape_area"), ["@qiq_square.2.Square_area"]);
        assert!(ir.contains("define i64 @qiq.util.3.area(ptr %arg0)"));
        assert!(ir.contains("call i64 @qiq.util.3.area(ptr"));
        assert!(ir.contains("call void @qiq_square.2.Square___construct(ptr"));
    }

    #[test]
    fn same_names_in_different_files_do_not_clash() {
        let ir = compile_files(&[
            ("main.qiq", "import first from \"a\"; import second from \"b\"; function helper(): int { return 0; } class Item {} print(first()); print(second()); print(helper());"),
            ("a.qiq", "function helper(): int { return 1; } class Item {} function first(): int { return helper(); }"),
            ("b.qiq", "function helper(): int { return 2; } class Item {} function second(): int { return helper(); }"),
        ])
        .unwrap();
        assert!(ir.contains("define i64 @helper()"));
        assert!(ir.contains("define i64 @qiq.a.1.helper()"));
        assert!(ir.contains("define i64 @qiq.b.2.helper()"));
    }

    #[test]
    fn subclasses_can_extend_imported_classes() {
        let ir = compile_files(&[
            ("main.qiq", "import Base from \"base\"; class Child extends Base { function __construct() { parent::__construct(5); } } Child child = new Child(); print(child->value());"),
            ("base.qiq", "class Base { protected int stored; function __construct(int stored) { this->stored = stored; } function value(): int { return this->stored; } }"),
        ])
        .unwrap();
        assert!(ir.contains("call void @qiq_base.1.Base___construct(ptr %this"));
        assert!(ir.contains("call i64 @qiq_base.1.Base_value(ptr"));
    }

    #[test]
    fn rejects_invalid_imports() {
        let cases: &[(&[(&str, &str)], &str)] = &[
            (&[("main.qiq", "import a from \"a\";"), ("a.qiq", "import b from \"b\"; function a(): void {}"), ("b.qiq", "import a from \"a\"; function b(): void {}")],
                "import cycle: a.qiq -> b.qiq -> a.qiq"),
            (&[("main.qiq", "import missing from \"a\";"), ("a.qiq", "function a(): void {}")],
                "main.qiq:1:8: 'a.qiq' does not declare 'missing'"),
            (&[("main.qiq", "import a from \"nope\";")],
                "main.qiq:1:1: could not read nope.qiq"),
            (&[("main.qiq", "import a from \"a\";"), ("a.qiq", "function a(): void {} print(1);")],
                "a.qiq:1:23: imported files can only contain imports and declarations"),
            (&[("main.qiq", "function f(): void {} import a from \"a\";"), ("a.qiq", "function a(): void {}")],
                "main.qiq:1:23: imports must appear before other declarations"),
            (&[("main.qiq", "import a, a from \"a\";"), ("a.qiq", "function a(): void {}")],
                "main.qiq:1:11: 'a' is already imported"),
            (&[("main.qiq", "import a from \"a\"; function a(): void {}"), ("a.qiq", "function a(): void {}")],
                "main.qiq:1:29: function 'a' conflicts with an imported function"),
            (&[("main.qiq", "import a from \"a\"; Hidden h = new Hidden();"), ("a.qiq", "function a(): void {} class Hidden {}")],
                "main.qiq:1:20: type 'Hidden' is not declared or imported"),
            (&[("main.qiq", "import a from \"a\"; print(hidden());"), ("a.qiq", "function a(): void {} function hidden(): int { return 1; }")],
                "main.qiq:1:20: function 'hidden' is not declared"),
            (&[("main.qiq", "import a from \"a\";"), ("a.qiq", "function a(): int { return \"x\"; }")],
                "a.qiq:1:21: function 'a': return type mismatch"),
            (&[("main.qiq", "import a from \"a\";"), ("a.qiq", "function a(): int {\n  return 1 +;\n}")],
                "a.qiq:2:13: expected a literal"),
            (&[("main.qiq", "import b from \"b\";"), ("a.qiq", "function a(): void {}"), ("b.qiq", "import a from \"a\"; function b(): void {}"), ("c.qiq", "")],
                ""),
        ];
        for (files, message) in cases {
            let result = compile_files(files);
            if message.is_empty() {
                assert!(result.is_ok(), "{files:?}: {result:?}");
                continue;
            }
            let error = result.unwrap_err();
            assert!(error.contains(message), "{files:?}: expected '{message}', got '{error}'");
        }
    }

    #[test]
    fn imports_are_not_re_exported_and_keywords_stay_contextual() {
        // main can't reach `a` through b: b only exports what it declares.
        let error = compile_files(&[
            ("main.qiq", "import a from \"b\";"),
            ("a.qiq", "function a(): void {}"),
            ("b.qiq", "import a from \"a\"; function b(): void {}"),
        ])
        .unwrap_err();
        assert!(error.contains("'b.qiq' does not declare 'a'"), "{error}");

        assert!(compile_source("import = 1; from = 2; print(import + from);").is_ok());
        assert!(compile_source("import a from \"a\";")
            .unwrap_err()
            .contains("cannot import a.qiq when compiling a source string"));
    }

    #[test]
    fn stdlib_functions_are_global_and_emitted_only_when_used() {
        let unused = compile_source("print(1);").unwrap();
        // Methods of standard library classes (File's helpers) are always in
        // the IR; LLVM removes them when the program doesn't use them.
        let stdlib_functions: Vec<&str> = unused
            .lines()
            .filter(|line| line.contains("@qiq_stdlib_") && !line.contains("@qiq_stdlib___"))
            .collect();
        assert!(stdlib_functions.is_empty(), "{stdlib_functions:?}");
        assert!(!unused.contains("str_starts_with"));

        let used = compile_source("print(str_repeat(\"héllo\", 2)); print(strtoupper(\"é\"));").unwrap();
        assert!(used.contains("declare ptr @qiq_stdlib_str_repeat(ptr, i64)"));
        assert!(used.contains("declare ptr @qiq_stdlib_strtoupper(ptr)"));
        assert!(!used.contains("@qiq_stdlib_substr"));
        assert!(used.contains("call ptr @qiq_stdlib_str_repeat(ptr"));
        assert!(compile_source("int n = strlen(\"abc\");").unwrap_err().contains("use len(text)"));

        // A stdlib function written in qiq is emitted with the externs it calls.
        let qiq_body = compile_source("function check(): void { if (str_starts_with(\"ab\", \"a\") == true) { print(1); } } check();").unwrap();
        assert!(qiq_body.contains("define i1 @qiq.stdlib.string.str_starts_with(ptr %arg0, ptr %arg1)"));
        assert!(qiq_body.contains("declare ptr @qiq_stdlib_substr(ptr, i64, i64)"));
        assert!(!qiq_body.contains("@qiq_stdlib_str_contains"));
    }

    #[test]
    fn stdlib_functions_are_available_in_imported_files() {
        let ir = compile_files(&[
            ("main.qiq", "import shout from \"util\"; print(shout(\"hé\"));"),
            ("util.qiq", "function shout(string text): string { return strtoupper(text) + \"!\"; }"),
        ])
        .unwrap();
        assert!(ir.contains("call ptr @qiq_stdlib_strtoupper(ptr"));
    }

    #[test]
    fn string_literals_have_static_reference_counts() {
        let ir = compile_source("print(\"hi\");").unwrap();
        assert!(ir.contains("@.str0 = private unnamed_addr constant { i64, i32, i32, [3 x i8] } { i64 -1, i32 1902706689, i32 0, [3 x i8] c\"hi\\00\" }, align 8"), "{ir}");
    }

    #[test]
    fn temporaries_are_released_at_the_end_of_their_statement() {
        let main = main_ir(&compile_source("string a = \"x\"; print(len(a + \"y\")); int n = 1;").unwrap()).to_string();
        let concat = main.find("@qiq_string_concat").unwrap();
        let printed = main.find("@printf").unwrap();
        let concat_register = main[..concat].rsplit("  ").next().unwrap().trim_end_matches(" = call ptr ");
        // Released after the print, before the next statement's store.
        let release = main.find(&format!("call void @qiq_release(ptr {concat_register})")).unwrap();
        assert!(printed < release && release < main.find("store i64 1").unwrap(), "{main}");
    }

    #[test]
    fn assignments_release_the_previous_value() {
        let main = main_ir(&compile_source("string s = \"a\"; s = s + \"b\";").unwrap()).to_string();
        // The new string is handed over (no retain), and the old one released.
        assert!(!main.contains("@qiq_retain"), "{main}");
        assert_eq!(main.matches("call void @qiq_release(").count(), 2, "{main}");
        let property = compile_source("class Box { public string label; } Box b = new Box(); string s = \"x\"; b->label = s;").unwrap();
        // A variable's value is retained when stored elsewhere.
        assert!(main_ir(&property).contains("call void @qiq_retain(ptr %v"), "{property}");
    }

    #[test]
    fn parameters_are_borrowed_unless_reassigned() {
        let ir = compile_source("function a(string s): int { return len(s); } function b(string s): int { s = s + \"!\"; return len(s); } int n = a(\"x\") + b(\"y\");").unwrap();
        let a = &ir[ir.find("define i64 @a(").unwrap()..];
        let a = &a[..a.find("\n}").unwrap()];
        assert!(!a.contains("@qiq_retain") && !a.contains("@qiq_release"), "{a}");
        let b = &ir[ir.find("define i64 @b(").unwrap()..];
        let b = &b[..b.find("\n}").unwrap()];
        assert!(b.contains("call void @qiq_retain(ptr %arg0)"), "{b}");
        // Released before both the normal return and the exception return.
        assert_eq!(b.matches("%release").count() / 2, 2, "{b}");
    }

    #[test]
    fn exceptions_release_temporaries_of_code_inside_the_handler() {
        let main = main_ir(&compile_source("function f(string s): int { return 1; } int n = 0; try { n = f(\"a\" + \"b\"); } catch (Exception e) { n = 2; }").unwrap()).to_string();
        assert!(main.contains("label %cleanup"), "{main}");
        let cleanup = &main[main.find("\ncleanup").unwrap()..];
        assert!(cleanup.contains("call void @qiq_release(ptr %v") && cleanup.contains("br label %try.dispatch0"), "{main}");
    }

    #[test]
    fn objects_record_which_fields_hold_references() {
        let ir = compile_source("class Node { public int id; public string name; public Node next; public bool done; } class Child extends Node { public int[] items; }").unwrap();
        // '2' marks fields that can be part of a cycle (objects), '1' other
        // references (strings, arrays of numbers).
        let node = ir.lines().position(|line| line.contains("c\"0120\\00\"")).is_some();
        let child = ir.lines().position(|line| line.contains("c\"01201\\00\"")).is_some();
        assert!(node && child, "{ir}");
        assert!(ir.contains("@qiq_class_reference_fields = unnamed_addr constant ["), "{ir}");
    }

    #[test]
    fn arrays_and_maps_record_whether_they_can_be_in_cycles() {
        let ir = compile_source("class Node { public Node next; } int[] a = [1]; string[] b = [\"x\"]; Node[] c = [new Node()]; Node[][] d = [[new Node()]]; map[string, Node] e = {}; map[string, string[]] f = {};").unwrap();
        assert!(ir.contains("@qiq_array_new(i64 1, i64 8, i32 0)"), "{ir}");
        assert!(ir.contains("@qiq_array_new(i64 1, i64 8, i32 1)"), "{ir}");
        assert_eq!(ir.matches("@qiq_array_new(i64 1, i64 8, i32 5)").count(), 3, "{ir}");
        assert!(ir.contains("@qiq_map_new(i32 3, i32 5)") && ir.contains("@qiq_map_new(i32 3, i32 1)"), "{ir}");
    }

    #[test]
    fn null_can_be_stored_wherever_a_reference_is_expected() {
        let source = "class Node { public Node next; public string name; function __construct(Node next = null) { this->next = next; } } \
            Node n = null; n = new Node(); n->next = null; string s = null; int[] xs = null; map[string, Node] m = {\"a\" => null}; \
            Node[] ns = [null]; ns[0] = null; m[\"b\"] = null; function f(string t): string { return null; } string r = f(null); Node k = new Node(null);";
        let ir = compile_source(source).unwrap();
        assert!(ir.contains("store ptr null, ptr"), "{ir}");
        for (source, error) in [
            ("int n = null;", "cannot initialize int variable 'n' with null"),
            ("class A { public bool done; } A a = new A(); a->done = null;", "property 'done' requires bool, found null"),
            ("function f(): void { this = null; }", "cannot assign to 'this'"),
        ] {
            assert!(compile_source(source).unwrap_err().contains(error), "{source}");
        }
    }

    #[test]
    fn object_variables_can_be_reassigned_within_their_declared_type() {
        let ir = compile_source("interface Shape {} class Square implements Shape {} class Circle implements Shape {} \
            Shape shape = new Square(); shape = new Circle(); Square sq = new Square(); sq = new Square(); \
            function walk(Shape s): void { s = new Circle(); }").unwrap();
        // A reassigned object parameter gets a slot that holds a reference.
        assert!(ir.contains("call void @qiq_retain(ptr %arg0)"), "{ir}");
        let error = compile_source("interface Shape {} class Square implements Shape {} class Circle implements Shape {} Square sq = new Square(); sq = new Circle();").unwrap_err();
        assert!(error.contains("cannot assign Circle to Square variable 'sq'"), "{error}");
    }

    #[test]
    fn overridden_methods_dispatch_on_the_object_class() {
        let ir = compile_source("class A { function f(): int { return 1; } function g(): int { return 2; } private function h(): int { return 3; } function callH(): int { return this->h(); } } \
            class B extends A { function f(): int { return 4; } private function h(): int { return 5; } } \
            A a = new B(); int x = a->f() + a->g() + a->callH();").unwrap();
        // f is overridden: looked up in a table. g isn't, and h is private: direct.
        assert!(ir.contains("@qiq_dispatch_A_f = private unnamed_addr constant"), "{ir}");
        assert!(main_ir(&ir).contains("call i64 @qiq_A_g(ptr"), "{ir}");
        assert!(ir.contains("call i64 @qiq_A_h(ptr %this)"), "{ir}");
    }

    #[test]
    fn sprintf_passes_typed_values_to_the_runtime() {
        let main = main_ir(&compile_source("byte b = 7; string s = sprintf(\"%d %s %.2f %c %d\", 1, \"x\", 2.5, 'y', b);").unwrap()).to_string();
        // One type character per value, and the values in an i64 array.
        assert!(main.contains("alloca [5 x i64]"), "{main}");
        assert!(main.contains("call ptr @qiq_sprintf(ptr getelementptr"), "{main}");
        assert!(main.contains(", i64 5)"), "{main}");
        let ir = compile_source("string s = sprintf(\"%d\", 1);").unwrap();
        assert!(ir.contains("c\"i\\00\""), "{ir}");
    }

    #[test]
    fn sprintf_checks_literal_formats() {
        assert_eq!(super::sprintf_arguments_required("%s %5.2f %%"), Ok(2));
        assert_eq!(super::sprintf_arguments_required("%2$s %1$s %2$s"), Ok(2));
        assert_eq!(super::sprintf_arguments_required("%'*-10s %+05d"), Ok(2));
        for (source, error) in [
            ("string s = sprintf(\"%s %s\", 1);", "the format uses 2 values, but 1 is given"),
            ("string s = sprintf(\"%z\", 1);", "unknown format specifier \"z\""),
            ("string s = sprintf(\"50%\");", "missing format specifier at end of string"),
            ("string s = sprintf(\"%0$s\", 1);", "argument number specifier must be greater than zero"),
            ("string s = sprintf(\"%s\", [1]);", "sprintf value 1 must be an int, double, bool, byte, string, or char, found int[]"),
            ("string s = sprintf(1);", "sprintf format must be a string, found int"),
            ("int n = strlen(\"abc\");", "use len(text)"),
        ] {
            let message = compile_source(source).unwrap_err();
            assert!(message.contains(error), "{source}: {message}");
        }
        // A format that isn't a literal is checked when the program runs.
        assert!(compile_source("string f = \"%s %s\"; string s = sprintf(f, 1);").is_ok());
    }

    #[test]
    fn json_decode_takes_its_type_from_the_context() {
        let ir = compile_source("class User { public string name; public int[] ids; private string secret; } \
            User u = json_decode(\"{}\"); map[string, User[]] m = json_decode(\"{}\"); \
            function f(int[] xs): void {} f(json_decode(\"[1]\")); string s = json_encode(u, true);").unwrap();
        // Public fields only, with their slots and descriptors.
        assert!(ir.contains("c\"4|name:1:s;ids:2:ai;\\00\""), "{ir}");
        assert!(ir.contains("@qiq_class_json = unnamed_addr constant ["), "{ir}");
        let main = main_ir(&ir);
        assert!(main.contains("call i64 @qiq_json_decode(ptr getelementptr"), "{main}");
        assert!(main.contains("call ptr @qiq_json_encode(ptr getelementptr") && main.contains("i1 zeroext true)"), "{main}");
        // Descriptors name the class to build: map[string, User[]] is "msao<id>;".
        assert!(ir.contains("i8] c\"msao"), "{ir}");
    }

    #[test]
    fn json_decode_checks_its_target() {
        for (source, error) in [
            ("interface Shape {} Shape s = json_decode(\"{}\");", "json_decode can't create Shape, which is an interface"),
            ("abstract class A {} class B { public A[] items; } B b = json_decode(\"{}\");", "json_decode can't create A, which is abstract"),
            ("x = json_decode(\"1\");", "json_decode needs to know the type to decode into"),
            ("int n = json_decode(5);", "json_decode expects a string, found int"),
            ("string s = json_encode(1, 2);", "json_encode's second argument (pretty) must be a bool"),
            ("function json_encode(int n): string { return \"\"; }", "cannot redeclare built-in function 'json_encode'"),
        ] {
            let message = compile_source(source).unwrap_err();
            assert!(message.contains(error), "{source}: {message}");
        }
        // Classes that refer to themselves are fine; private fields of abstract types aren't decoded.
        assert!(compile_source("abstract class A {} class Node { public Node next; private A hidden; } Node n = json_decode(\"{}\");").is_ok());
    }

    #[test]
    fn foreach_over_a_map_skips_removed_entries() {
        let main = main_ir(&compile_source("map[string, int] m = {\"a\" => 1}; foreach (m as k => v) { print(v); }").unwrap()).to_string();
        assert!(main.contains("call i64 @qiq_map_iteration_end(ptr"), "{main}");
        assert!(main.contains("call i64 @qiq_map_version(ptr"), "{main}");
        assert!(main.contains("call i1 @qiq_map_entry_live(ptr"), "{main}");
        assert!(register_between(&main, "label %foreach.entry", ", label %foreach.increment"), "{main}");
    }

    #[test]
    fn map_functions_work_with_any_key_and_value_types() {
        let main = main_ir(&compile_source("map[string, int[]] m = {}; bool a = map_key_exists(\"k\", m); string[] ks = map_keys(m); int[][] vs = map_values(m); bool b = in_map([1], m); unset(m[\"k\"]);").unwrap()).to_string();
        assert!(main.contains("call zeroext i1 @qiq_map_has(ptr"), "{main}");
        assert!(main.contains("call ptr @qiq_map_keys(ptr") && main.contains(", i64 8, i32 1)"), "{main}");
        // map_values of int[] values: elements are references that can't cycle.
        assert!(main.contains("call ptr @qiq_map_values(ptr"), "{main}");
        assert!(main.contains("call zeroext i1 @qiq_map_contains_value(ptr"), "{main}");
        assert!(main.contains("call void @qiq_map_remove(ptr"), "{main}");
        for (source, error) in [
            ("int[] xs = [1]; unset(xs[0]);", "unset can't remove array elements"),
            ("map[string, int] m = {}; bool b = map_key_exists(1, m);", "map key requires string, found int"),
            ("map[string, int] m = {}; bool b = in_map(\"a\", m);", "in_map: this map's values are int, found string"),
            ("int[] k = map_keys([1]);", "map_keys expects a map, found int[]"),
            ("map[string, int] m = {}; unset(m);", "unset expects a map entry"),
        ] {
            let message = compile_source(source).unwrap_err();
            assert!(message.contains(error), "{source}: {message}");
        }
    }

    #[test]
    fn coalesce_reads_without_throwing_and_evaluates_the_default_lazily() {
        let main = main_ir(&compile_source("map[string, int] m = {}; int a = m[\"k\"] ?? 5; int[] xs = [1]; int b = xs[3] ?? 0; string s = null; string t = s ?? \"d\";").unwrap()).to_string();
        assert!(main.contains("call zeroext i1 @qiq_map_find(ptr"), "{main}");
        assert!(!main.contains("@qiq_map_get") && !main.contains("@qiq_array_check_index"), "{main}");
        assert!(main.contains("coalesce.missing0:\n  store i64 5"), "{main}");
        for (source, error) in [
            ("int n = 1; int m = n ?? 2;", "'??' needs a map or array element, or a value that can be null"),
            ("map[string, int] m = {}; int x = m[\"a\"] ?? \"s\";", "the right side of '??' must be int, found string"),
        ] {
            let message = compile_source(source).unwrap_err();
            assert!(message.contains(error), "{source}: {message}");
        }
        // Right-associative, below ||.
        assert!(compile_source("map[string, bool] m = {}; bool b = m[\"a\"] ?? m[\"b\"] ?? false || true;").is_ok());
    }

    #[test]
    fn arrays_grow_and_shrink_in_place() {
        let main = main_ir(&compile_source("string[] xs; array_push(xs, \"a\", \"b\"); array_unshift(xs, \"z\"); string last = array_pop(xs); array_shift(xs); string[] cut = array_splice(xs, 0, 1, [\"q\"]); bool[] flags = [true]; array_push(flags, false);").unwrap()).to_string();
        // Declared without a value: an empty array.
        assert!(main.contains("call ptr @qiq_array_new(i64 0, i64 8, i32 1)"), "{main}");
        assert_eq!(main.matches("call void @qiq_array_push(ptr").count(), 3, "{main}");
        assert!(main.contains("call void @qiq_array_insert(ptr ") && main.contains(", i64 8, i64 0, i64 "), "{main}");
        assert!(main.contains("call i64 @qiq_array_pop(ptr") && main.contains("call i64 @qiq_array_shift(ptr"), "{main}");
        assert!(main.contains("call ptr @qiq_array_splice(ptr") && main.contains("i1 zeroext true, ptr %v"), "{main}");
        // Bools are 1-byte elements.
        assert!(main.contains(", i64 1, i64 "), "{main}");
        for (source, error) in [
            ("int[] xs; array_push(xs, \"a\");", "array_push adds int values to this array, found string"),
            ("map[string, int] m = {}; array_push(m, 1);", "array_push expects an array, found map[string, int]"),
            ("int[] xs; array_push(xs);", "array_push expects an array and at least one value"),
            ("int[] xs; int[] r = array_splice(xs, \"1\");", "array_splice offset must be an int"),
            ("int[] xs; int[] r = array_splice(xs, 0, 1, [\"a\"]);", "array elements must all be int, found string"),
            ("int[] xs; string[] ys = [\"a\"]; int[] r = array_splice(xs, 0, 1, ys);", "array_splice replacement must be int[], found string[]"),
            ("int x = 1; x;", "only a function call can be used as a statement"),
        ] {
            let message = compile_source(source).unwrap_err();
            assert!(message.contains(error), "{source}: {message}");
        }
    }

    #[test]
    fn foreach_over_an_array_checks_its_current_length() {
        let main = main_ir(&compile_source("int[] xs = [1]; foreach (xs as x) { array_push(xs, 2); }").unwrap()).to_string();
        // The bound is loaded from the array before every element.
        assert!(register_between(&main, "%foreach.length", " = load i64, ptr %v"), "{main}");
        assert!(main.contains("icmp ult i64 %v") && main.contains(", %foreach.length"), "{main}");
        // Elements are found through the array's data pointer.
        assert!(main.contains(".at = getelementptr inbounds i8, ptr %v") && main.contains(", i64 8\n  %v"), "{main}");
    }

    #[test]
    fn foreach_over_an_int_range_counts_without_an_array() {
        let main = main_ir(&compile_source("foreach (range(1, 10, 2) as i => n) { print(n); }").unwrap()).to_string();
        assert!(main.contains("call i64 @qiq_range_plan(i64 1, i64 10, i64 2, i1 zeroext true, ptr %range.step"), "{main}");
        assert!(!main.contains("@qiq_range_int") && !main.contains("@qiq_array_new"), "{main}");
        // Other ranges, and range used as a value, build arrays.
        let ir = compile_source("foreach (range(0.0, 1.0, 0.5) as d) { print(d); } foreach (range('a', 'c') as c) { print(c); } int[] xs = range(5, 1);").unwrap();
        assert!(ir.contains("call ptr @qiq_range_double(double ") && ir.contains("call ptr @qiq_range_char(ptr "), "{ir}");
        assert!(ir.contains("call ptr @qiq_range_int(i64 5, i64 1, i64 1, i1 zeroext false)"), "{ir}");
        // range(count) counts max(count, 0) times from 0, without the checks
        // a range between two numbers needs, or an array.
        let counting = main_ir(&compile_source("int n = 3; foreach (range(n) as i) { print(i); }").unwrap()).to_string();
        assert!(counting.contains("icmp sgt i64") && counting.contains("select i1"), "{counting}");
        assert!(!counting.contains("@qiq_range_plan") && !counting.contains("@qiq_range_count") && !counting.contains("@qiq_array_new"), "{counting}");
        assert!(compile_source("int[] xs = range(4);").unwrap().contains("call ptr @qiq_range_count(i64 4)"));
        // Ints mixed with doubles make a double range.
        assert!(compile_source("double[] d = range(0, 1, 0.25);").unwrap().contains("sitofp i64 0 to double"));
        for (source, error) in [
            ("int[] r = range();", "range expects a count, or start, end, and an optional step; found 0 arguments"),
            ("int[] r = range(2.5);", "range(count) counts from 0 up to count - 1 and needs an int, found double"),
            ("char[] r = range('a', 5);", "range of chars needs two chars, found char and int"),
            ("int[] r = range(\"a\", \"b\");", "range expects ints, doubles, or two chars, found string"),
            ("int[] r = range(0, 1, 0.5);", "cannot initialize int[] variable 'r' with double[]"),
        ] {
            let message = compile_source(source).unwrap_err();
            assert!(message.contains(error), "{source}: {message}");
        }
    }

    #[test]
    fn standard_library_modules_are_imported() {
        // Names from imported modules are free for programs to use themselves.
        assert!(compile_source("class File {} function time(): int { return 1; } File f = new File(); print(time());").is_ok());
        assert!(compile_source("import time as now from \"std/time\"; import Router from \"std/net/http/server\"; print(now()); Router r = new Router();").is_ok());
        let moved = compile_source("import http_get from \"std/http\";").unwrap_err();
        assert!(moved.contains("the standard library module \"std/http\" moved to \"std/net/http\""), "{moved}");
        assert_eq!(
            compile_source("print(time());").unwrap_err(),
            "1:1: function 'time' is not declared; it's in the standard library: import time from \"std/time\";"
        );
        assert_eq!(
            compile_source("Socket s = null;").unwrap_err(),
            "1:1: type 'Socket' is not declared or imported; it's in the standard library: import Socket from \"std/net\";"
        );
        assert_eq!(compile_source("import __socket_close from \"std/net\";").unwrap_err(), "1:8: 'std/net' does not declare '__socket_close'");
        // std/crypto: hashing, with its names free when it isn't imported.
        let crypto = compile_source("import sha1, hash_hmac, hash_bytes, bin2hex from \"std/crypto\"; string a = sha1(\"x\"); string b = hash_hmac(\"sha256\", \"m\", \"k\"); string c = bin2hex(hash_bytes(\"md5\", [1]));").unwrap();
        let base64 = compile_source("import base64_encode, base64_decode_bytes from \"std/crypto\"; byte[] b = base64_decode_bytes(base64_encode(\"x\"));").unwrap();
        assert!(base64.contains("@qiq_stdlib_base64_encode(") && base64.contains("@qiq_stdlib_base64_decode_bytes("), "{base64}");
        assert_eq!(
            compile_source("string h = password_hash(\"pw\");").unwrap_err(),
            "1:1: function 'password_hash' is not declared; qiq's is bcrypt_hash: import bcrypt_hash from \"std/crypto\";"
        );
        let passwords = compile_source("import bcrypt_hash, bcrypt_verify from \"std/crypto\"; string h = bcrypt_hash(\"pw\"); bool ok = bcrypt_verify(\"pw\", h);").unwrap();
        // The default cost (12) is filled in at the call.
        assert!(passwords.contains("@qiq_stdlib_bcrypt_hash(ptr") && passwords.contains(", i64 12)") && passwords.contains("@qiq_stdlib_bcrypt_verify("), "{passwords}");
        for function in ["qiq_stdlib_sha1", "qiq_stdlib_hash_hmac", "qiq_stdlib_hash_bytes", "qiq_stdlib_bin2hex"] {
            assert!(crypto.contains(&format!("@{function}(")), "{function}: {crypto}");
        }
        assert!(compile_source("function hash(string s): int { return len(s); } int n = hash(\"x\");").is_ok());
        assert_eq!(
            compile_source("print(md5(\"x\"));").unwrap_err(),
            "1:1: function 'md5' is not declared; it's in the standard library: import md5 from \"std/crypto\";"
        );
        assert!(compile_source("import x from \"std/nothing\";").unwrap_err().contains("the standard library has no module \"std/nothing\""));
        // The prelude (strings, exceptions, built-ins) needs no import.
        assert!(compile_source("print(strtoupper(\"a\"), len(\"ab\")); throw new InvalidArgumentException(\"x\");").is_ok());
    }

    #[test]
    fn programs_receive_their_command_line() {
        let ir = compile_source("import argv, exit, getenv from \"std/os\"; string[] args = argv(); string home = getenv(\"HOME\") ?? \"\"; if (len(args) > 1) { exit(2); }").unwrap();
        assert!(ir.contains("define i32 @main(i32 %argc, ptr %argv) {\nentry:\n  call void @qiq_set_arguments(i32 %argc, ptr %argv)"), "{ir}");
        assert!(ir.contains("declare ptr @qiq_stdlib_argv()") && ir.contains("declare ptr @qiq_stdlib_getenv(ptr)"), "{ir}");
        assert!(ir.contains("call void @qiq_stdlib_exit(i64 2)"), "{ir}");
        // exit's status is optional.
        assert!(compile_source("import exit from \"std/os\"; exit();").unwrap().contains("call void @qiq_stdlib_exit(i64 0)"));
        // Without the import, the name is free.
        assert!(compile_source("function exit(int n): void {} exit(1);").is_ok());
        assert!(compile_source("import exit from \"std/os\"; function exit(int n): void {}").unwrap_err().contains("conflicts with an imported function"));
    }

    #[test]
    fn else_if_chains() {
        let source = "function grade(int n): string { if (n >= 90) { return \"A\"; } else if (n >= 80) { return \"B\"; } else if (n >= 70) { return \"C\"; } else { return \"F\"; } }";
        // Every path returns, so the function needs no final return.
        let ir = compile_source(source).unwrap();
        assert_eq!(ir.matches("icmp sge i64 %v").count(), 3, "{ir}");
        assert!(compile_source("int n = 1; if (n == 1) { print(1); } else if (n == 2) { print(2); }").is_ok());
        assert!(compile_source("int n = 1; if (n == 1) { print(1); } else print(2);").unwrap_err().contains("expected '{' before else body"));
    }

    #[test]
    fn http_and_sockets_are_in_the_standard_library() {
        let ir = compile_source("import Response, http_get, http_build_query from \"std/net/http\"; import Socket from \"std/net\"; Response r = http_get(\"https://example.com/\", headers: {\"Accept\" => \"text/html\"}, timeout: 5); string q = http_build_query({\"a\" => \"b c\"}); Socket s = Socket::connect(\"example.com\", 80); s->close();").unwrap();
        assert!(ir.contains("declare i64 @qiq_stdlib___socket_connect(ptr, i64, i1 zeroext, i64)"), "{ir}");
        assert!(ir.contains("define ptr @qiq.stdlib.http."), "{ir}");
        for (source, error) in [
            ("int h = __socket_connect(\"x\", 1, false, 1);", "function '__socket_connect' is not declared"),
            ("import Socket from \"std/net\"; Socket s = new Socket(1);", "cannot access non-public member"),
            ("import Response, http_get from \"std/net/http\"; Response r = http_get(5);", "argument type mismatch calling 'http_get'"),
        ] {
            let message = compile_source(source).unwrap_err();
            assert!(message.contains(error), "{source}: {message}");
        }
    }

    #[test]
    fn anonymous_functions_capture_variables_in_cells() {
        let ir = compile_source("int count = 0; function(): void inc = function(): void { count = count + 1; }; inc(); print(count);").unwrap();
        // count lives in a cell, shared by main and the closure.
        assert!(ir.contains("define void @qiq.closure.0(ptr %env)"), "{ir}");
        assert!(ir.contains("%capture0.at = getelementptr inbounds i64, ptr %env, i64 2"), "{ir}");
        let main = main_ir(&ir);
        assert!(main.contains("store ptr @qiq.closure.0, ptr"), "{main}");
        assert!(main.contains("call void @qiq_check_callable(ptr"), "{main}");
        // Variables no closure uses stay in stack slots.
        let plain = compile_source("int n = 1; print(n);").unwrap();
        assert!(!main_ir(&plain).contains("@qiq_object_new"), "{plain}");
    }

    #[test]
    fn function_types_and_arrow_functions() {
        let ir = compile_source("function apply(function(int): int f, int x): int { return f(x); } int y = apply(fn(int v) => v * 2, 3); twice = fn(int v) => v * 2; (function(): int)[] list = [fn() => 1];").unwrap();
        assert!(ir.contains("define i64 @apply(ptr %arg0, i64 %arg1)"), "{ir}");
        assert!(ir.contains("define i64 @qiq.closure.0(ptr %env, i64 %arg0)"), "{ir}");
        assert!(ir.contains("define i64 @qiq.closure.2(ptr %env)"), "{ir}");
        // Named functions as values go through an adapter.
        let reference = compile_source("function sq(int x): int { return x * x; } function(int): int f = sq;").unwrap();
        assert!(reference.contains("define i64 @qiq.reference.sq(ptr %env, i64 %arg0)"), "{reference}");
        for (source, error) in [
            ("function(int): int f = fn(int x) => x; int y = f();", "this function(int): int takes 1 argument, found 0"),
            ("function(int): int f = fn(string x) => 1;", "cannot initialize function(int): int variable 'f' with function(string): int"),
            ("int x = 5; x(1);", "'x' is a variable of type int, not a function"),
            ("function(int): int f = fn(int x) => x; f(x: 1);", "function values take no named arguments"),
            ("f = fn(int x = 1) => x;", "anonymous function parameters can't have defaults"),
            ("int n = len;", "built-in function 'len' can't be used as a value"),
            ("function(int): int f = function(int x): int { return \"s\"; };", "anonymous function: return type mismatch"),
        ] {
            let message = compile_source(source).unwrap_err();
            assert!(message.contains(error), "{source}: {message}");
        }
        // A function named fn still works: only `fn(...) =>` is an arrow function.
        assert!(compile_source("function fn(int x): int { return x; } int y = fn(2);").is_ok());
    }

    #[test]
    fn class_names() {
        // ClassName::class is a constant; value::class reads the object's class id.
        let ir = compile_source("interface S {} class C implements S {} S s = new C(); string a = C::class; string b = s::class;").unwrap();
        assert!(ir.contains("c\"C\\00\" }"), "{ir}");
        assert!(main_ir(&ir).contains("@qiq_class_name_strings, i64"), "{ir}");
        assert!(ir.contains("@qiq_class_name_strings = private unnamed_addr constant ["), "{ir}");
        // Programs without value::class have no table.
        assert!(!compile_source("class C {} print(C::class);").unwrap().contains("@qiq_class_name_strings"));
        for (source, error) in [
            ("int n = 5; print(n::class);", "::class needs an object or a class name, found int"),
            ("print(Nope::class);", "'Nope' is neither a declared or imported class nor a variable"),
            ("print(self::class);", "self:: can only be used inside a class"),
            ("class A { function f(): string { return parent::class; } }", "parent:: requires a class that extends another"),
        ] {
            let message = compile_source(source).unwrap_err();
            assert!(message.contains(error), "{source}: {message}");
        }
    }

    #[test]
    fn datetime_in_queries_json_and_query_as() {
        // A DateTime query value goes as text, through DateTime::__sqlText.
        let ir = compile_source("import Database from \"std/sql\"; import DateTime from \"std/time\"; class E { function __construct(public DateTime at) {} } \
            Database db = Database::open(\"sqlite::memory:\"); db->execute(\"insert into e values (?)\", DateTime::of(2026, 1, 1)); E[] es = db->queryAs(\"select at from e\"); print(json_encode(es));").unwrap();
        assert!(main_ir(&ir).contains("___sqlText("), "{ir}");
        assert!(ir.contains("c\"T\\00\""), "{ir}");
        // JSON writes it as text, with the formatter linked in only then.
        assert!(ir.contains("c\"!T") && ir.contains("@qiq_json_datetime_format = constant ptr @qiq_stdlib___datetime_format"), "{ir}");
        assert!(compile_source("print(json_encode([1]));").unwrap().contains("@qiq_json_datetime_format = constant ptr null"));
        let message = compile_source("import DateTime from \"std/time\"; class B { public DateTime at; } B b = json_decode(\"{}\");").unwrap_err();
        assert!(message.contains("json_decode can't create a DateTime yet"), "{message}");
        // A Url is written as its text.
        let url = compile_source("import Url from \"std/net\"; print(json_encode([Url::parse(\"/a\")]));").unwrap();
        assert!(url.contains("c\"!S"), "{url}");
        let message = compile_source("import Url from \"std/net\"; Url[] u = json_decode(\"[]\");").unwrap_err();
        assert!(message.contains("json_decode can't create a Url yet: decode it as a string and read that with Url::parse"), "{message}");
    }

    #[test]
    fn query_as_reads_rows_into_the_expected_class() {
        let ir = compile_source("import Database from \"std/sql\"; class U { function __construct(public int id, public string name = \"\") {} } \
            Database db = Database::open(\"sqlite::memory:\"); U[] all = db->queryAs(\"select * from u where id > ?\", 1); U one = db->queryOneAs(\"select * from u\"); \
            function f(Database db): U[] { return db->queryAs(\"select 1\"); }").unwrap();
        // One reader per class and shape, shared by both array calls.
        assert_eq!(ir.matches("define ptr @qiq.rows_to.").count(), 1, "{ir}");
        assert_eq!(ir.matches("define ptr @qiq.row_to.").count(), 1, "{ir}");
        // A cursor's nextAs and fetchAs use the same readers.
        let cursor = compile_source("import Database, Cursor from \"std/sql\"; class U { function __construct(public int id) {} } \
            Database db = Database::open(\"sqlite::memory:\"); Cursor c = db->cursor(\"select 1 as id\"); U one = c->nextAs(); U[] some = c->fetchAs(2); U again = db->queryOneAs(\"select 1 as id\");").unwrap();
        assert_eq!(cursor.matches("define ptr @qiq.row_to.").count(), 1, "{cursor}");
        assert_eq!(cursor.matches("define ptr @qiq.rows_to.").count(), 1, "{cursor}");
        assert!(main_ir(&ir).contains("call void @qiq_bind_values(ptr"), "{ir}");
        for (source, error) in [
            ("foreach (db->queryAs(\"select 1\") as u) {}", "queryAs needs to know the class to read rows into"),
            ("int[] n = db->queryAs(\"select 1\");", "queryAs reads rows into an array of objects, such as User[], not int[]"),
            ("class A { function __construct(string[] tags) {} } A[] a = db->queryAs(\"select 1\");", "queryAs can't read the constructor's parameter 'tags' of A, a string[], from a column"),
            ("abstract class B {} B b = db->queryOneAs(\"select 1\");", "queryOneAs can't create B, which is abstract"),
            ("Cursor c = db->cursor(\"select 1\"); foreach (c->fetchAs(5) as x) {}", "fetchAs needs to know the class to read rows into"),
            ("Cursor c = db->cursor(\"select 1\"); int n = c->nextAs();", "nextAs reads rows into an object, not int"),
        ] {
            let source = format!("import Database, Cursor from \"std/sql\"; Database db = Database::open(\"x\"); {source}");
            let message = compile_source(&source).unwrap_err();
            assert!(message.contains(error), "{source}: {message}");
        }
    }

    #[test]
    fn query_values_and_acyclic_classes() {
        // Values after a `...` method's parameters go to the runtime in one
        // call before the method runs: here an int, a string, and a null.
        let ir = compile_source("import Database from \"std/sql\"; Database db = Database::open(\"sqlite::memory:\"); int n = db->execute(\"select ?, ?, ?\", 1, \"a\", null);").unwrap();
        assert!(main_ir(&ir).contains("call void @qiq_bind_values(ptr"), "{ir}");
        assert!(ir.contains("c\"isn\\00\""), "{ir}");
        // Only the network drivers a Database::open can reach are linked in.
        assert!(ir.contains("@qiq_sql_postgres = constant ptr null") && ir.contains("@qiq_sql_mysql = constant ptr null"), "{ir}");
        let postgres = compile_source("import Database from \"std/sql\"; Database db = Database::open(\"postgres://u@db/app\");").unwrap();
        assert!(postgres.contains("@qiq_sql_postgres = constant ptr @qiq_postgres_driver") && postgres.contains("@qiq_sql_mysql = constant ptr null"));
        let computed = compile_source("import Database from \"std/sql\"; string url = \"sqlite::memory:\"; Database db = Database::open(url);").unwrap();
        assert!(computed.contains("@qiq_sql_postgres = constant ptr @qiq_postgres_driver") && computed.contains("@qiq_sql_mysql = constant ptr @qiq_mysql_driver"));
        for (source, error) in [
            ("import Database from \"std/sql\"; Database db = Database::open(\"x\"); db->execute(\"select ?\", [1, 2]);", "value 1 for the query must be an int, double, bool, byte, string, char, byte[], DateTime, or null, found int[]"),
            ("class A { function f(string s, ...): void {} }", "'...' parameters are only for standard library methods"),
        ] {
            let message = compile_source(source).unwrap_err();
            assert!(message.contains(error), "{source}: {message}");
        }
        // A class nothing extends whose fields can't reach an object isn't
        // tracked for cycles, nor is one holding such objects; one that can
        // reach itself, or that is extended, is.
        let ir = compile_source("class Leaf { public string s; public int[] n; } class Holder { public Leaf leaf; public Leaf[] leaves; } class Node { public Node next; } class Base { public string s; } class Sub extends Base {} Holder h = new Holder(); Node n = new Node(); Base b = new Sub();").unwrap();
        // The field mask of a class: '2' for a field that can be part of a
        // cycle, '1' for another reference, '0' for the rest.
        let mask = |name: &str| -> String {
            let declaration = ir
                .lines()
                .find(|line| line.starts_with("@.class.name") && line.contains(&format!("c\"{name}\\00\"")))
                .unwrap();
            let index = &declaration["@.class.name".len()..declaration.find(' ').unwrap()];
            let line = ir.lines().find(|line| line.starts_with(&format!("@.class.fields{index} "))).unwrap();
            line.split("c\"").nth(1).unwrap().split('\\').next().unwrap().to_string()
        };
        assert_eq!(mask("Leaf"), "11", "{ir}");
        assert_eq!(mask("Holder"), "11", "{ir}");
        assert_eq!(mask("Node"), "2", "{ir}");
    }

    #[test]
    fn promotion_readonly_nullsafe_and_to_string() {
        // A promoted parameter is a property set before the constructor's body.
        let ir = compile_source("class P { function __construct(public readonly int x, private string name = \"p\") { print(this->x); } } P p = new P(3); print(p->x);").unwrap();
        assert!(ir.contains("define void @qiq_P___construct("), "{ir}");
        // A null receiver throws instead of crashing; ?-> branches around the chain.
        let ir = compile_source("class A { public string s = \"a\"; } A a = null; print(a?->s ?? \"none\");").unwrap();
        assert!(main_ir(&ir).contains("nullsafe.null"), "{ir}");
        let ir = compile_source("class A { public int n; } A a = null; print(a->n);").unwrap();
        assert!(main_ir(&ir).contains("cannot read property 'n' of null") || ir.contains("cannot read property 'n' of null"), "{ir}");
        // __toString is called by +.
        let ir = compile_source("class M { function __toString(): string { return \"m\"; } } print(\"x\" + new M());").unwrap();
        assert!(main_ir(&ir).contains("@qiq_M___toString("), "{ir}");
        for (source, error) in [
            ("class P { function __construct(public readonly int x) {} } P p = new P(1); p->x = 2;", "property 'x' is readonly: only P's constructor can set it"),
            ("class P { public readonly int x; function __construct() { this->x = 1; } function f(): void { this->x = 2; } }", "property 'x' is readonly"),
            ("class P { public readonly int x = 1; }", "a readonly property can't have a default"),
            ("class P { public static readonly int x; }", "a static property can't be readonly"),
            ("class P { function f(public int x): void {} }", "only a constructor's parameters can be properties too"),
            ("class A { public int n; } A a = null; int n = a?->n;", "add a default with ??"),
            ("class A { public int n; } A a = null; int n = a?->n ?? \"x\";", "'??' after '?->' needs a default of type int, found string"),
            ("class A { public int n; } A a = null; a?->n = 1;", "can't assign through '?->'"),
            ("class A { function __toString(): int { return 1; } }", "__toString must be declared `public function __toString(): string`"),
            ("class A {} string s = to_string(new A());", "needs the class to declare `public function __toString(): string`"),
        ] {
            let message = compile_source(source).unwrap_err();
            assert!(message.contains(error), "{source}: {message}");
        }
    }

    #[test]
    fn instanceof_constants_defaults_and_object_identity() {
        let ir = compile_source("interface S {} class C implements S { public int n = 7; } class D extends C {} S s = new D(); if (s instanceof C c) { print(c->n); } bool any = s instanceof D;").unwrap();
        // instanceof reads the class id and looks it up in a table, with a
        // stand-in for null: no branch.
        assert!(ir.contains("@qiq_is.C = private unnamed_addr constant [") && ir.contains("@qiq_no_object = private unnamed_addr constant i64"), "{ir}");
        assert!(main_ir(&ir).contains("select i1") && main_ir(&ir).contains("getelementptr inbounds ["), "{ir}");
        // Defaults are stored by one function per class, before the constructor.
        assert!(ir.contains("define void @qiq_defaults.D(ptr %object)") && main_ir(&ir).contains("call void @qiq_defaults.D(ptr"), "{ir}");
        // Constants are their value.
        let constants = main_ir(&compile_source("class A { const N = 41; } int x = A::N + 1;").unwrap()).to_string();
        assert!(constants.contains("add i64 41, 1") || constants.contains("i64 42") || constants.contains("41"), "{constants}");
        // Objects compare by identity.
        assert!(main_ir(&compile_source("class A {} A a = new A(); A b = a; bool same = a == b;").unwrap()).contains("icmp eq ptr"));
        for (source, error) in [
            ("class A {} class B {} A a = new A(); bool b = a instanceof B;", "an object of class A can never be a B"),
            ("class A {} A a = new A(); while (a instanceof A x) { break; }", "a name after instanceof ('x') can only be given in an if condition"),
            ("class A {} A a = new A(); if (a instanceof A x) { print(1); } print(x);", "variable 'x' is not declared"),
            ("class A { const X = 1; } A::X = 2;", "can't assign to constant A::X"),
            ("class A { const X = [1]; }", "constant 'X' must be a literal"),
            ("class A { const X = A::Y; const Y = A::X; }", "is defined in terms of itself"),
            ("class A { const X = 1; } class B extends A { const X = 2; }", "constant 'X' is already declared in 'A'"),
            ("class A { public int x = to_int(\"3\"); }", "the default for 'x' must be a constant"),
            ("class A { public string[] x = [1]; }", "the default for 'x' must be string, found int"),
            ("class A {} class B {} A a = new A(); B b = new B(); bool c = a == b;", "comparison requires related types"),
        ] {
            let message = compile_source(source).unwrap_err();
            assert!(message.contains(error), "{source}: {message}");
        }
        // A function returning from both branches of an if with bindings returns.
        assert!(compile_source("class A { public int n; } function f(A a, bool ok): int { if (ok && a instanceof A x && x->n > 0) { return 1; } else { return 0; } }").is_ok());
    }

    #[test]
    fn increments_and_compound_assignments_are_assignments() {
        // `x++` and `x op= v` compile exactly like the assignment written out.
        for (short, long) in [
            ("int i = 0; i++;", "int i = 0; i = i + 1;"),
            ("int i = 0; --i;", "int i = 0; i = i - 1;"),
            ("double d = 0.5; d++;", "double d = 0.5; d = d + 1.0;"),
            ("int i = 3; i <<= 2;", "int i = 3; i = i << 2;"),
            ("string s = \"a\"; s += \"b\";", "string s = \"a\"; s = s + \"b\";"),
            ("int[] a = [1]; a[0] *= 3;", "int[] a = [1]; a[0] = a[0] * 3;"),
            ("map[string, int] m = {}; m[\"k\"]++;", "map[string, int] m = {}; m[\"k\"] = m[\"k\"] + 1;"),
        ] {
            assert_eq!(main_ir(&compile_source(short).unwrap()), main_ir(&compile_source(long).unwrap()), "{short}");
        }
        // A target part that could run code is evaluated once.
        let ir = compile_source("function f(): int { return 0; } int[] a = [1]; a[f()] += 1; a[f()]++;").unwrap();
        assert_eq!(main_ir(&ir).matches("call i64 @f(").count(), 2, "{ir}");
        for (source, error) in [
            ("int i = 0; print(i++);", "'++' and '--' are statements in qiq"),
            ("int i = 0; int[] a = [1]; a[i++] = 5;", "'++' and '--' are statements in qiq"),
            ("int i = 0; int n = i++;", "'++' and '--' are statements in qiq"),
            ("int i = 0; int n = ++i;", "'++' and '--' are statements in qiq"),
            ("int i = 0; i += i++;", "'++' and '--' are statements in qiq"),
            ("string s = \"a\"; s++;", "'++' and '--' work on int and double values, found string"),
            ("byte y = 1; y--;", "'++' and '--' work on int and double values, found byte"),
            ("function f(): int { return 1; } f()++;", "1:33: only variables, indexed elements, or properties can be incremented or decremented"),
            ("int n = 1; n += 1.5;", "'+' requires two ints or two doubles"),
            ("string s = \"a\"; s -= \"b\";", "'-' requires two ints or two doubles; found string and string"),
        ] {
            let message = compile_source(source).unwrap_err();
            assert!(message.contains(error), "{source}: {message}");
        }
        // `5 - -3` and `x - -1` still subtract a negative number.
        assert!(compile_source("int x = 5 - -3; x = x - -1;").is_ok());
    }

    #[test]
    fn arrays_and_maps_compare_by_contents() {
        let main = main_ir(&compile_source("int[] a = [1]; int[] b = [1]; bool same = a == b; bool different = a != []; map[string, int[]] m = {}; bool maps = m == {\"k\" => [1]};").unwrap()).to_string();
        assert_eq!(main.matches("call zeroext i1 @qiq_values_equal(ptr").count(), 3, "{main}");
        // != negates the result; the descriptor says what to compare.
        assert!(main.contains("xor i1 %v"), "{main}");
        let ir = compile_source("map[string, int[]] m = {}; bool maps = m == {};").unwrap();
        assert!(ir.contains("c\"msai\\00\""), "{ir}");
        // Comparing with null is still a pointer test.
        let null_test = main_ir(&compile_source("int[] a = [1]; bool none = a == null;").unwrap()).to_string();
        assert!(!null_test.contains("@qiq_values_equal") && null_test.contains("icmp eq ptr"), "{null_test}");
        for (source, error) in [
            ("int[] a = [1]; int[] b = [2]; bool c = a < b;", "int[] values support only == and != comparisons"),
            ("Exception[] a = []; Exception[] b = []; bool c = a == b;", "Exception[] values can't be compared: objects and functions in them can only be compared with null"),
            ("map[string, function(): void] a = {}; bool c = a == a;", "can't be compared: objects and functions in them"),
            ("int[] a = [1]; string[] b = [\"1\"]; bool c = a == b;", "comparison requires matching types, found int[] and string[]"),
            ("Exception e = new Exception(\"x\"); bool c = e < e;", "Exception values support only == and != comparisons"),
            ("class A {} class B {} A a = new A(); B b = new B(); bool c = a == b;", "comparison requires related types, found A and B"),
        ] {
            let message = compile_source(source).unwrap_err();
            assert!(message.contains(error), "{source}: {message}");
        }
    }

    #[test]
    fn nullable_values_compare_with_null() {
        for element in ["string", "char", "int[]", "map[string, int]", "Exception"] {
            let source = format!("{element}[] values = new {element}[1]; print(values[0] == null, null != values[0]);");
            let ir = compile_source(&source).unwrap();
            assert!(ir.contains("icmp eq ptr %v") && ir.contains("icmp ne ptr null, %v"), "{source}\n{ir}");
        }
        assert!(compile_source("string s = \"a\"; print(s < null);")
            .unwrap_err()
            .contains("null supports only == and != comparisons"));
        assert!(compile_source("int n = 1; print(n == null);")
            .unwrap_err()
            .contains("comparison requires matching types, found int and null"));
        // String equality is null-safe and can't throw; ordering can.
        let main = main_ir(&compile_source("string s = \"b\"; print(s == \"a\"); print(s < \"a\");").unwrap()).to_string();
        let equality = main.find("@qiq_string_equality").unwrap();
        let ordering = main.find("@qiq_string_compare").unwrap();
        assert!(!main[equality..ordering].contains("@qiq_pending_exception"), "{main}");
        assert!(main[ordering..].contains("@qiq_pending_exception"), "{main}");
    }

    #[test]
    fn file_class_hides_its_runtime_helpers() {
        let ir = compile_source("import File from \"std/file\"; File f = File::open(\"a.txt\", \"w\"); f->write(\"hi\"); f->close();").unwrap();
        assert!(ir.contains("declare i64 @qiq_stdlib___file_open(ptr, ptr)"));
        assert!(ir.contains("declare void @qiq_stdlib___file_write(i64, ptr)"));
        assert!(compile_source("int h = __file_open(\"a\", \"r\");")
            .unwrap_err()
            .contains("function '__file_open' is not declared"));
        assert!(compile_source("import File from \"std/file\"; File f = new File(3);").unwrap_err().contains("cannot access non-public member"));
        assert!(compile_source("import File from \"std/file\"; class File {}").unwrap_err().contains("class 'File' is already imported or built in"));
        assert!(compile_source("import File from \"std/file\"; class LogFile extends File {}").unwrap_err().contains("cannot extend final class 'File'"));
    }

    #[test]
    fn len_measures_strings_arrays_and_maps() {
        let ir = compile_source("int[] xs = [1, 2]; map[string, int] m = {}; print(len(\"hé\"), len(xs), len(m));").unwrap();
        assert!(ir.contains("call i64 @qiq_len_string(ptr"), "{ir}");
        assert!(ir.contains("call i64 @qiq_len_array(ptr"), "{ir}");
        assert!(ir.contains("call i64 @qiq_len_map(ptr"), "{ir}");
        // The result is an int, so it infers and combines like one.
        assert!(compile_source("n = len(\"abc\") + 1; int[] xs = new int[n];").is_ok());
        assert!(compile_source("print(len(5));")
            .unwrap_err()
            .contains("len expects a string, array, or map, found int"));
        assert!(compile_source("function len(string s): int { return 0; }")
            .unwrap_err()
            .contains("cannot redeclare built-in function 'len'"));
        // Variables named `len` don't hide the function.
        assert!(compile_source("int len = 2; print(len + len(\"ab\"));").is_ok());
    }

    #[test]
    fn date_functions_default_the_timestamp_to_now() {
        // `timestamp` defaults to the smallest int, which the runtime reads as "now".
        let ir = compile_source("import date, gmdate, sleep from \"std/time\"; print(date(\"Y\")); print(gmdate(\"Y\", 0)); sleep(0);").unwrap();
        assert!(ir.contains("declare ptr @qiq_stdlib_date(ptr, i64)"));
        assert!(ir.contains("i64 -9223372036854775808)"), "{ir}");
        assert!(ir.contains("declare void @qiq_stdlib_sleep(i64)"));
        let named = compile_source("import mktime from \"std/time\"; int t = mktime(0, year: 2026, month: 11, day: 19); print(t);").unwrap();
        assert!(named.contains("call i64 @qiq_stdlib_mktime(i64 0, i64 0, i64 0, i64 11, i64 19, i64 2026)"), "{named}");
        assert!(compile_source("function time(): int { return 0; }").is_ok());
    }

    #[test]
    fn built_in_functions_cannot_be_redeclared_or_imported() {
        assert!(compile_source("function strtoupper(string text): string { return text; }")
            .unwrap_err()
            .contains("cannot redeclare built-in function 'strtoupper'"));
        let imported = compile_files(&[
            ("main.qiq", "import substr from \"util\";"),
            ("util.qiq", "function helper(): void {}"),
        ])
        .unwrap_err();
        assert!(imported.contains("main.qiq:1:8: 'substr' is built in and needs no import"), "{imported}");
        let in_other_file = compile_files(&[
            ("main.qiq", "import helper from \"util\";"),
            ("util.qiq", "function helper(): void {} function trim(string text): string { return text; }"),
        ])
        .unwrap_err();
        assert!(in_other_file.contains("util.qiq:1:37: cannot redeclare built-in function 'trim'"), "{in_other_file}");
    }

    #[test]
    fn extern_functions_are_limited_to_the_standard_library() {
        assert!(compile_source("extern function shout(string text): string;")
            .unwrap_err()
            .contains("1:1: extern functions can only be declared in the standard library"));
        // `extern` is contextual: still usable as a name.
        assert!(compile_source("extern = 1; print(extern);").is_ok());
    }

    #[test]
    fn plus_concatenates_strings_and_chars() {
        let ir = compile_source("string a = \"x\" + \"y\"; string b = a + 'é'; string c = 'a' + 'b'; print(b + c);").unwrap();
        assert_eq!(ir.matches("call ptr @qiq_string_concat(ptr").count(), 4);
        assert!(ir.contains("declare ptr @qiq_string_concat(ptr, ptr)"));

        for (source, found) in [
            ("string s = \"n\" + 1;", "found string and int"),
            ("int n = 1 + 1.5;", "found int and double"),
            ("bool b = true + false;", "found bool and bool"),
        ] {
            let error = compile_source(source).unwrap_err();
            assert!(error.contains("'+' requires two ints or two doubles, or strings and chars to concatenate"), "{error}");
            assert!(error.contains(found), "{source}: {error}");
        }
        assert!(compile_source("char c = 'a' + 'b';")
            .unwrap_err()
            .contains("cannot initialize char variable 'c' with string"));
    }

    #[test]
    fn arithmetic_operators_compile_for_ints_and_doubles() {
        let ints = compile_source("int a = 7; int b = 2; print(a - b); print(a * b); print(a / b); print(a % b); print(a ** b); print(-a);").unwrap();
        for instruction in ["= sub i64 %v", "= mul i64 %v", "call i64 @qiq_int_divide(i64", "call i64 @qiq_int_modulo(i64", "call i64 @qiq_int_power(i64", "= sub i64 0, %v"] {
            assert!(ints.contains(instruction), "missing {instruction}");
        }
        let doubles = compile_source("double a = 7.0; double b = 2.0; print(a - b); print(a * b); print(a / b); print(a % b); print(a ** b); print(-a);").unwrap();
        for instruction in ["= fsub double", "= fmul double", "= fdiv double", "= frem double", "call double @llvm.pow.f64(double", "= fneg double"] {
            assert!(doubles.contains(instruction), "missing {instruction}");
        }
    }

    #[test]
    fn checked_int_operations_are_inline() {
        // A constant divisor other than 0 and -1 can't fail: no runtime call.
        let constant = compile_source("int x = 10; print(x % 7, x / 3, x << 2, x >> 63, x ** 3);").unwrap();
        for instruction in ["srem i64 %v", "sdiv i64 %v", "shl i64 %v", "ashr i64 %v", "call i64 @qiq_int_power_unchecked(i64 %v"] {
            assert!(constant.contains(instruction), "missing {instruction}: {constant}");
        }
        for call in ["@qiq_int_modulo(", "@qiq_int_divide(", "@qiq_int_shift_left(", "@qiq_int_shift_right(", "@qiq_int_power("] {
            assert!(!constant.split("define i32 @main").nth(1).unwrap().contains(call), "{call} in main: {constant}");
        }
        // Otherwise the instruction is the fast path, and the runtime (which
        // throws) the slow one.
        let variable = compile_source("int x = 10; int y = 3; print(x % y, x / y, x << y, x ** y);").unwrap();
        for both in [["srem i64", "@qiq_int_modulo("], ["sdiv i64", "@qiq_int_divide("], ["shl i64", "@qiq_int_shift_left("], ["@qiq_int_power_unchecked(", "@qiq_int_power(i64"]] {
            assert!(variable.contains(both[0]) && variable.contains(both[1]), "{both:?}: {variable}");
        }
        // Array indexes are checked inline; only a failing check calls the runtime.
        let index = compile_source("int[] a = [1, 2, 3]; int i = 1; print(a[i]);").unwrap();
        assert!(index.contains("icmp ult i64") && index.contains("call void @qiq_array_check_index("), "{index}");
        // Conversions too.
        let conversions = compile_source("double d = 2.5; int n = 65; print(to_int(d), to_byte(n));").unwrap();
        assert!(conversions.contains("fptosi double") && conversions.contains("trunc i64"), "{conversions}");
    }

    #[test]
    fn arithmetic_precedence_and_associativity() {
        // print((2 + 3) * 4): the add happens before the multiply.
        let grouped = compile_source("int a = 2; print((a + 3) * 4);").unwrap();
        assert!(grouped.contains("%v2 = add i64 %v1, 3\n  %v3 = mul i64 %v2, 4"), "{grouped}");
        let ungrouped = compile_source("int a = 2; print(a + 3 * 4);").unwrap();
        assert!(ungrouped.contains("%v2 = mul i64 3, 4\n  %v3 = add i64 %v1, %v2"), "{ungrouped}");
        // 10 - 4 - 3 is (10 - 4) - 3.
        let left = compile_source("int a = 10; print(a - 4 - 3);").unwrap();
        assert!(left.contains("%v2 = sub i64 %v1, 4\n  %v3 = sub i64 %v2, 3"), "{left}");
        // 2 ** 3 ** 2 is 2 ** (3 ** 2).
        let right = compile_source("print(2 ** 3 ** 2);").unwrap();
        let inner = right.find("%v0 = call i64 @qiq_int_power_unchecked(i64 3, i64 2)").expect("inner power");
        let outer = right.find("@qiq_int_power_unchecked(i64 2, i64 %v0)").expect("outer power");
        assert!(inner < outer, "{right}");
        // -2 ** 2 is -(2 ** 2); a plain negative literal is folded.
        let negated_power = compile_source("print(-2 ** 2);").unwrap();
        let power = negated_power.find("%v0 = call i64 @qiq_int_power_unchecked(i64 2, i64 2)").expect("power");
        let negate = negated_power.find("%v1 = sub i64 0, %v0").expect("negate");
        assert!(power < negate, "{negated_power}");
        let literal = compile_source("print(-5); print(2 ** -1);").unwrap();
        assert!(literal.contains("i64 -5)"));
        assert!(literal.contains("@qiq_int_power(i64 2, i64 -1)"));
    }

    #[test]
    fn integer_literal_range_includes_the_smallest_int() {
        // ints are 64-bit: the range is -9223372036854775808 to 9223372036854775807.
        assert!(compile_source("int n = -9223372036854775808;").unwrap().contains("store i64 -9223372036854775808"));
        assert!(compile_source("int n = 9223372036854775807;").unwrap().contains("store i64 9223372036854775807"));
        assert!(compile_source("int n = 2147483648;").is_ok());
        assert!(compile_source("int n = 9223372036854775808;")
            .unwrap_err()
            .contains("1:9: integer literal is outside the int range"));
        assert!(compile_source("int n = -9223372036854775809;")
            .unwrap_err()
            .contains("integer literal is outside the int range"));
    }

    #[test]
    fn arithmetic_rejects_mixed_and_non_numeric_operands() {
        let cases = [
            ("int n = 1 - 1.5;", "'-' requires two ints or two doubles; found int and double"),
            ("string s = \"a\" * 2;", "'*' requires two ints or two doubles; found string and int"),
            ("string s = \"a\" - \"b\";", "'-' requires two ints or two doubles; found string and string"),
            ("bool b = true % false;", "'%' requires two ints or two doubles; found bool and bool"),
            ("double d = 2.0 ** 2;", "'**' requires two ints or two doubles; found double and int"),
            ("string s = -\"a\";", "unary '-' requires an int or a double; found string"),
            ("int n = (1 + 2;", "expected ')' after expression"),
        ];
        for (source, message) in cases {
            let error = compile_source(source).unwrap_err();
            assert!(error.contains(message), "{source}: expected '{message}', got '{error}'");
        }
    }

    #[test]
    fn less_equal_and_greater_equal_for_numbers_and_strings() {
        let ir = compile_source("int n = 1; double d = 1.0; string s = \"a\"; if (n <= 2) { print(1); } if (n >= 0) { print(1); } if (d <= 2.0) { print(1); } if (d >= 0.5) { print(1); } if (s <= \"b\") { print(1); } if (s >= \"a\") { print(1); }").unwrap();
        for predicate in ["icmp sle i64", "icmp sge i64", "fcmp ole double", "fcmp oge double"] {
            assert!(ir.contains(predicate), "missing {predicate}");
        }
        assert_eq!(ir.matches("call i32 @qiq_string_compare").count(), 2);
        assert!(ir.contains("icmp sle i64 %v") && ir.contains(", 0\n"));
    }

    #[test]
    fn logical_operators_short_circuit() {
        let and = compile_source("function f(): bool { return true; } bool a = true; if (a && f()) { print(1); }").unwrap();
        assert!(and.contains("br i1 %v1, label %and.rhs0, label %and.end0\nand.rhs0:\n  %v3 = call i1 @f()"), "{and}");
        let or = compile_source("function f(): bool { return true; } bool a = true; if (a || f()) { print(1); }").unwrap();
        assert!(or.contains("br i1 %v1, label %or.end0, label %or.rhs0\nor.rhs0:\n  %v3 = call i1 @f()"), "{or}");
        // The result slot is allocated once, in the entry block.
        assert!(or.contains("entry:\n  call void @qiq_set_arguments(i32 %argc, ptr %argv)\n  %v0 = alloca i1, align 8\n  %v2 = alloca i1, align 1\n"), "{or}");
    }

    #[test]
    fn logical_precedence_and_bool_expressions() {
        // a || b && c is a || (b && c): the && is evaluated inside the || right side.
        let ir = compile_source("bool a = true; bool b = true; bool c = false; bool r = a || b && c;").unwrap();
        let or_rhs = ir.find("or.rhs").unwrap();
        let and_rhs = ir.find("and.rhs").unwrap();
        assert!(or_rhs < and_rhs, "{ir}");
        // Comparisons are expressions everywhere, not just in if.
        let stored = compile_source("int n = 3; bool small = n <= 5; print(small); print(n == 3);").unwrap();
        assert!(stored.contains("icmp sle i64"));
        assert!(stored.contains("icmp eq i64"));
        assert!(compile_source("bool ready = true; if (ready) { print(1); }").is_ok());
    }

    #[test]
    fn rejects_invalid_conditions() {
        let cases = [
            ("int n = 1; if (n) { print(1); }", "if condition must be a bool, found int"),
            ("int n = 1; if (n > 0 && 5) { print(1); }", "'&&' requires bools, found int"),
            ("int n = 1; if (1 || n > 0) { print(1); }", "'||' requires bools, found int"),
            ("int n = 1; if (0 < n < 5) { print(1); }", "comparisons cannot be chained; combine them with && or ||"),
            ("bool a = true; if (a >= false) { print(1); }", "bool values support only == and != comparisons"),
            ("int n = 1; if (n <= 1.0) { print(1); }", "comparison requires matching types, found int and double"),
            ("int n = 1; if (n & 1) { print(1); }", "if condition must be a bool, found int"),
        ];
        for (source, message) in cases {
            let error = compile_source(source).unwrap_err();
            assert!(error.contains(message), "{source}: expected '{message}', got '{error}'");
        }
    }

    #[test]
    fn bitwise_operators_compile_for_ints() {
        let ir = compile_source("int a = 12; int b = 10; print(a & b); print(a | b); print(a ^ b); print(~a); print(a << b); print(a >> b);").unwrap();
        for instruction in [
            "= and i64 %v",
            "= or i64 %v",
            "= xor i64 %v",
            ", -1\n",
            "call i64 @qiq_int_shift_left(i64",
            "call i64 @qiq_int_shift_right(i64",
        ] {
            assert!(ir.contains(instruction), "missing {instruction}");
        }
    }

    #[test]
    fn bitwise_precedence() {
        // x & 1 == 0 is (x & 1) == 0.
        let ir = compile_source("int x = 6; bool even = x & 1 == 0;").unwrap();
        assert!(ir.contains("%v2 = and i64 %v1, 1\n  %v3 = icmp eq i64 %v2, 0"), "{ir}");
        // 1 << 2 + 1 is 1 << (2 + 1).
        let shift = compile_source("print(1 << 2 + 1);").unwrap();
        let sum = shift.find("%v0 = add i64 2, 1\n").expect("sum");
        let shifted = shift.find("shl i64 1, %v0").expect("shift");
        assert!(sum < shifted, "{shift}");
        // 1 | 6 ^ 3 & 2 is 1 | (6 ^ (3 & 2)).
        let levels = compile_source("print(1 | 6 ^ 3 & 2);").unwrap();
        assert!(levels.contains("%v0 = and i64 3, 2\n  %v1 = xor i64 6, %v0\n  %v2 = or i64 1, %v1"), "{levels}");
        // ~ binds like unary minus: ~a + 1 is (~a) + 1.
        let not = compile_source("int a = 5; print(~a + 1);").unwrap();
        assert!(not.contains("%v2 = xor i64 %v1, -1\n  %v3 = add i64 %v2, 1"), "{not}");
    }

    #[test]
    fn bitwise_operators_require_ints() {
        let cases = [
            ("print(1.5 & 1);", "'&' requires two ints; found double and int"),
            ("bool b = true; print(b | false);", "'|' requires two ints; found bool and bool"),
            ("print(\"a\" ^ \"b\");", "'^' requires two ints; found string and string"),
            ("print(2.0 << 1);", "'<<' requires two ints; found double and int"),
            ("print(~2.0);", "'~' requires an int; found double"),
        ];
        for (source, message) in cases {
            let error = compile_source(source).unwrap_err();
            assert!(error.contains(message), "{source}: expected '{message}', got '{error}'");
        }
    }

    #[test]
    fn omitted_arguments_are_filled_with_defaults_at_the_call() {
        let ir = compile_source("function f(int a, int b = 5, double c = -1.5, bool d = true, string e = \"x\", char g = 'y'): void {} f(1); f(1, 2);").unwrap();
        assert!(ir.contains("call void @f(i64 1, i64 5, double -1.50000000000000000e0, i1 true, ptr getelementptr inbounds ({ i64, i32, i32, [2 x i8] }, ptr @.str0"), "{ir}");
        assert!(ir.contains("call void @f(i64 1, i64 2, double -1.50000000000000000e0, i1 true"));
        // Methods and constructors use the defaults of the method that runs.
        let methods = compile_source("class A { function __construct(int n = 3) {} function m(int x = 7): void {} } A a = new A(); a->m();").unwrap();
        assert!(methods.contains("call void @qiq_A___construct(ptr %v0, i64 3)"));
        assert!(register_between(&methods, "call void @qiq_A_m(ptr %v", ", i64 7)"), "{methods}");
        let parent = compile_source("class A { function __construct(int n = 3) {} } class B extends A { function __construct() { parent::__construct(); } }").unwrap();
        assert!(parent.contains("call void @qiq_A___construct(ptr %this, i64 3)"));
    }

    #[test]
    fn dispatched_calls_use_the_running_class_defaults() {
        let ir = compile_source("interface T { function r(int x = 1): int; } class A implements T { function r(int x = 2): int { return x; } } class B implements T { function r(int x = 3): int { return x; } } function show(T t): void { print(t->r()); print(t->r(9)); } show(new A()); show(new B());").unwrap();
        assert_eq!(table_entries(&ir, "qiq_dispatch_T_r.0"), ["@qiq_A_r.0", "@qiq_B_r.0"]);
        assert!(ir.contains("@qiq_dispatch_T_r = private"));
        assert!(ir.contains("define i64 @qiq_A_r.0(ptr %this) {\nentry:\n  %result = call i64 @qiq_A_r(ptr %this, i64 2)"));
        assert!(ir.contains("define i64 @qiq_B_r.0(ptr %this) {\nentry:\n  %result = call i64 @qiq_B_r(ptr %this, i64 3)"));
        // A call passing every argument doesn't need adapters.
        let full = compile_source("interface T { function r(int x = 1): int; } class A implements T { function r(int x = 2): int { return x; } } function show(T t): void { print(t->r(9)); }").unwrap();
        assert!(!full.contains(".0("));
    }

    #[test]
    fn rejects_invalid_optional_parameters() {
        let cases = [
            ("function bad(int a = 1, int b): void {}", "1:30: required parameter 'b' cannot follow optional parameter 'a'"),
            ("function wrong(int count = \"ten\"): void {}", "default for 'count' must be int, found string"),
            ("function wrong(double d = 1): void {}", "default for 'd' must be double, found int"),
            ("function computed(int size = len(\"abc\")): void {}", "default for 'size' must be a literal"),
            ("function computed(int size = 1 + 2): void {}", "default for 'size' must be a literal"),
            ("function n(int count = null): void {}", "default for 'count' must be int, found null"),
            ("function g(string n, string h = \"Hi\"): void {} g();", "function 'g' expects 1 to 2 argument(s), found 0"),
            ("function g(string n, string h = \"Hi\"): void {} g(\"a\", \"b\", \"c\");", "function 'g' expects 1 to 2 argument(s), found 3"),
            ("class A { function __construct(int n, int m = 1) {} } A a = new A();", "constructor 'A' expects 1 to 2 argument(s), found 0"),
            ("interface T { function r(bool e = true); } class P implements T { function r(bool e): void {} }", "method 'P::r' cannot make optional parameter 'e' required"),
            ("class A { function m(int x = 1): void {} } class B extends A { function m(int x): void {} }", "method 'B::m' cannot make optional parameter 'x' required"),
        ];
        for (source, message) in cases {
            let error = compile_source(source).unwrap_err();
            assert!(error.contains(message), "{source}: expected '{message}', got '{error}'");
        }
        // Making a required parameter optional in an override is allowed.
        assert!(compile_source("class A { function m(int x): void {} } class B extends A { function m(int x = 5): void {} } B b = new B(); b->m();").is_ok());
    }

    #[test]
    fn constructor_array_arguments_are_single_pointers() {
        let ir = compile_source("class Bag { function __construct(int[] values) {} } class Sub extends Bag { function __construct(int[] values) { parent::__construct(values); } } int[] items = [1, 2, 3]; Bag bag = new Bag(items);").unwrap();
        assert!(ir.contains("define void @qiq_Bag___construct(ptr %this, ptr %arg0)"));
        // The array parameter isn't reassigned, so it's passed on as is.
        assert!(ir.contains("call void @qiq_Bag___construct(ptr %this, ptr %arg0)"), "{ir}");
        assert!(main_ir(&ir).contains("call void @qiq_Bag___construct(ptr %v"), "{ir}");
    }

    #[test]
    fn named_arguments_fill_parameters_by_name() {
        let ir = compile_source("function f(int a, int b = 5, int c = 7): void {} f(1, c: 9); f(c: 3, a: 2);").unwrap();
        assert!(ir.contains("call void @f(i64 1, i64 5, i64 9)"), "{ir}");
        assert!(ir.contains("call void @f(i64 2, i64 5, i64 3)"));
        let constructor = compile_source("class A { function __construct(int n = 1, int m = 2) {} function go(int x = 3, int y = 4): void {} } A a = new A(m: 8); a->go(y: 6);").unwrap();
        assert!(constructor.contains("call void @qiq_A___construct(ptr %v0, i64 1, i64 8)"));
        assert!(register_between(&constructor, "call void @qiq_A_go(ptr %v", ", i64 3, i64 6)"), "{constructor}");
        let parent = compile_source("class A { function __construct(int n = 1, int m = 2) {} } class B extends A { function __construct() { parent::__construct(m: 9); } }").unwrap();
        assert!(parent.contains("call void @qiq_A___construct(ptr %this, i64 1, i64 9)"));
        let stdlib = compile_source("print(substr(\"abc\", length: 1, start: 1));").unwrap();
        assert!(stdlib.contains("call ptr @qiq_stdlib_substr(ptr getelementptr inbounds ({ i64, i32, i32, [4 x i8] }, ptr @.str0, i32 0, i32 3), i64 1, i64 1)"), "{stdlib}");
    }

    #[test]
    fn named_arguments_are_evaluated_in_written_order() {
        let ir = compile_source("function one(): int { return 1; } function two(): int { return 2; } function f(int a, int b): void {} f(b: two(), a: one());").unwrap();
        let main = &ir[ir.find("define i32 @main").unwrap()..];
        let two = main.find("%v0 = call i64 @two()").expect("two");
        let one = main.find("%v1 = call i64 @one()").expect("one");
        assert!(two < one, "{main}");
        assert!(main.contains("call void @f(i64 %v1, i64 %v0)"), "{main}");
    }

    #[test]
    fn dispatched_calls_can_skip_middle_parameters() {
        let ir = compile_source("interface T { function r(int a, int b = 1, int c = 2): int; } class A implements T { function r(int a, int b = 10, int c = 20): int { return a; } } function show(T t): void { print(t->r(0, c: 5)); } show(new A());").unwrap();
        assert_eq!(table_entries(&ir, "qiq_dispatch_T_r.s0_2"), ["@qiq_A_r.s0_2"]);
        assert!(ir.contains("define i64 @qiq_A_r.s0_2(ptr %this, i64 %arg0, i64 %arg2) {\nentry:\n  %result = call i64 @qiq_A_r(ptr %this, i64 %arg0, i64 10, i64 %arg2)"));
        // Naming only a leading prefix reuses the positional adapter.
        let prefix = compile_source("interface T { function r(int a, int b = 1): int; } class A implements T { function r(int a, int b = 10): int { return a; } } function show(T t): void { print(t->r(a: 0)); } show(new A());").unwrap();
        assert!(prefix.contains("define i64 @qiq_A_r.1(ptr %this, i64 %arg0)"));
    }

    #[test]
    fn rejects_invalid_named_arguments() {
        let g = "function g(string name, string greeting = \"Hi\", int times = 1): void {} ";
        let cases = [
            ("g(times: 2, \"Ada\");", "positional arguments cannot follow named arguments"),
            ("g(\"Ada\", nme: \"x\");", "function 'g' has no parameter named 'nme'"),
            ("g(\"Ada\", name: \"Lin\");", "function 'g': parameter 'name' is already given by position"),
            ("g(times: 2);", "function 'g': missing argument for required parameter 'name'"),
            ("g(\"Ada\", times: 1, times: 2);", "argument 'times' is named more than once"),
            ("g(\"Ada\", times: \"x\");", "argument type mismatch calling 'g': expected int, found string"),
            ("g(\"a\", \"b\", 1, 2);", "function 'g' expects 1 to 3 argument(s), found 4"),
        ];
        for (call, message) in cases {
            let source = format!("{g}{call}");
            let error = compile_source(&source).unwrap_err();
            assert!(error.contains(message), "{call}: expected '{message}', got '{error}'");
        }
        assert!(compile_source("class A { function m(int x = 1): void {} } A a = new A(); a->m(y: 2);")
            .unwrap_err()
            .contains("method 'm' has no parameter named 'y'"));
    }

    #[test]
    fn functions_are_declared_with_the_function_keyword() {
        assert!(compile_source("function add(int a, int b): int { return a + b; } print(add(1, 2));").is_ok());
        assert!(compile_source("fn add(int a, int b): int { return a + b; }")
            .unwrap_err()
            .contains("1:1: declare functions with 'function', not 'fn'"));
        assert!(compile_source("class A { fn m(): void {} }")
            .unwrap_err()
            .contains("1:11: declare functions with 'function', not 'fn'"));
        assert!(compile_source("interface I { fn m(); }")
            .unwrap_err()
            .contains("1:15: declare functions with 'function', not 'fn'"));
    }

    #[test]
    fn calls_are_followed_by_exception_checks() {
        let ir = compile_source("function f(): int { return 1; } print(f());").unwrap();
        let main = main_ir(&ir);
        assert!(main.contains("%v0 = call i64 @f()\n  %pending0 = load ptr, ptr @qiq_pending_exception, align 8\n  %thrown0 = icmp ne ptr %pending0, null\n  br i1 %thrown0, label %unwind, label %ok0\nok0:\n"), "{main}");
        // main reports uncaught exceptions; other functions return to their caller.
        assert!(main.contains("unwind:\n  %uncaught = load ptr, ptr @qiq_pending_exception"));
        assert!(main.contains("call void @qiq_uncaught(ptr %uncaught.name, ptr %uncaught.message)\n  unreachable"));
        assert!(ir.contains("define i64 @f() {\nentry:\n  ret i64 1\nunwind:\n  ret i64 0\n}"), "{ir}");
        // Runtime calls that can fail are checked too; plain arithmetic is not.
        let runtime = compile_source("int a = 1; int b = 2; print(a / b); print(a + b);").unwrap();
        assert_eq!(main_ir(&runtime).matches("= icmp ne ptr %pending").count(), 1);
    }

    #[test]
    fn runtime_exceptions_are_created_by_qiq_raise() {
        let ir = compile_source("print(1);").unwrap();
        assert!(ir.contains("define void @qiq_raise(i32 %kind, ptr %message) {\nentry:\n  switch i32 %kind, label %kind0 [ i32 1, label %kind1"));
        assert!(ir.contains("call void @qiq_stdlib.exceptions.Exception___construct(ptr %object2, ptr %message, i64 0)"));
        assert!(ir.contains("c\"DivisionByZeroException\\00\""));
        assert!(ir.contains("@qiq_class_names = unnamed_addr constant ["));
    }

    #[test]
    fn try_dispatches_on_class_ids_and_runs_finally() {
        let ir = compile_source("class MyException extends Exception {} try { throw new MyException(\"x\"); } catch (MyException e) { print(1); } finally { print(2); }").unwrap();
        let main = main_ir(&ir);
        assert!(main.contains("store ptr %v0, ptr @qiq_pending_exception, align 8\n  br label %try.dispatch0"), "{main}");
        assert!(main.contains("switch i64 %v2, label %try.rethrow0 ["), "{main}");
        assert!(main.contains("try.catch0.0:\n  store ptr null, ptr @qiq_pending_exception, align 8"));
        // finally is emitted on the catch path and on the rethrow path.
        assert_eq!(main.matches("i64 2)").count(), 2, "{main}");
        assert!(main.contains("try.rethrow0:\n  %v5 = load ptr, ptr @qiq_pending_exception"), "{main}");
    }

    #[test]
    fn finally_runs_before_return_break_and_continue() {
        let ir = compile_source("function f(): int { try { return 1; } finally { print(7); } }").unwrap();
        assert!(ir.contains("call i32 (ptr, ...) @printf(ptr @.fmt.int, i64 7)\n  ret i64 1"), "{ir}");
        let lp = compile_source("int[] xs = [1]; foreach (xs as x) { try { break; } finally { print(8); } }").unwrap();
        assert!(main_ir(&lp).contains("i64 8)\n  br label %foreach.end"), "{lp}");
        // A try around the loop doesn't run its finally on break.
        let outer = compile_source("int[] xs = [1]; try { foreach (xs as x) { break; } } finally { print(9); }").unwrap();
        assert!(!main_ir(&outer).contains("i64 9)\n  br label %foreach.end"), "{outer}");
    }

    #[test]
    fn exception_subclasses_inherit_constructors() {
        let ir = compile_source("class MyException extends Exception {} MyException e = new MyException(\"boom\", 3);").unwrap();
        assert!(ir.contains("call void @qiq_stdlib.exceptions.Exception___construct(ptr %v0, ptr getelementptr"), "{ir}");
        let user = compile_source("class A { function __construct(int n) {} } class B extends A {} class C extends B { function __construct() { parent::__construct(4); } } B b = new B(5);").unwrap();
        assert!(user.contains("call void @qiq_A___construct(ptr %v0, i64 5)"));
        assert!(user.contains("call void @qiq_A___construct(ptr %this, i64 4)"));
    }

    #[test]
    fn rejects_invalid_exception_code() {
        let cases = [
            ("throw 5;", "throw requires an Exception, found int"),
            ("class A {} throw new A();", "throw requires an Exception, found 'A'"),
            ("try { print(1); }", "try needs at least one catch or a finally block"),
            ("class A {} try { print(1); } catch (A e) { }", "catch requires Exception or a subclass, found 'A'"),
            ("try { print(1); } catch (Missing e) { }", "class 'Missing' is not declared or imported"),
            ("int e = 1; try { print(1); } catch (Exception e) { }", "variable 'e' is already declared"),
            ("class Exception {}", "class 'Exception' is already imported or built in"),
            ("function f(): int { throw new Exception(\"x\"); print(1); }", "unreachable statement after return, throw, break, or continue"),
            ("try { print(1); } catch (Exception e) { } print(e->getMessage());", "variable 'e' is not declared"),
        ];
        for (source, message) in cases {
            let error = compile_source(source).unwrap_err();
            assert!(error.contains(message), "{source}: expected '{message}', got '{error}'");
        }
        // A function may end by throwing instead of returning.
        assert!(compile_source("function f(int n): int { if (n > 0) { return n; } throw new Exception(\"no\"); }").is_ok());
    }

    #[test]
    fn file_functions_are_built_in() {
        let ir = compile_source("import file_put_contents, file_get_contents, file_exists from \"std/file\"; int n = file_put_contents(\"out.txt\", \"x\", append: true); string text = file_get_contents(\"out.txt\"); bool exists = file_exists(\"out.txt\");").unwrap();
        // Bool parameters carry zeroext after the type; bool returns before it.
        assert!(ir.contains("declare i64 @qiq_stdlib_file_put_contents(ptr, ptr, i1 zeroext)"), "{ir}");
        assert!(ir.contains("declare zeroext i1 @qiq_stdlib_file_exists(ptr)"));
        assert!(ir.contains("declare ptr @qiq_stdlib_file_get_contents(ptr)"));
        assert!(ir.contains(", i1 true)"));
        assert!(compile_source("function file_exists(string path): bool { return true; }").is_ok());
        // Prelude functions still can't be redeclared.
        assert!(compile_source("function trim(string text): string { return text; }")
            .unwrap_err()
            .contains("cannot redeclare built-in function 'trim'"));
    }

    #[test]
    fn while_loops_test_before_each_iteration() {
        let ir = compile_source("int i = 0; while (i < 3) { i = i + 1; }").unwrap();
        let main = main_ir(&ir);
        assert!(main.contains("  br label %while.test0\nwhile.test0:\n"), "{main}");
        assert!(main.contains("br i1 %v2, label %while.body0, label %while.end0\nwhile.body0:\n"), "{main}");
        assert!(main.contains("  br label %while.test0\nwhile.end0:\n"), "{main}");
        // continue re-tests the condition; break leaves the loop.
        let control = compile_source("int i = 0; while (i < 3) { i = i + 1; if (i == 1) { continue; } break; }").unwrap();
        assert!(main_ir(&control).contains("  br label %while.test0\nif.else"), "{control}");
        assert!(main_ir(&control).contains("  br label %while.end0\n"), "{control}");
    }

    #[test]
    fn while_true_without_break_ends_a_code_path() {
        assert!(compile_source("function f(int n): int { while (true) { if (n > 0) { return n; } n = n + 1; } }").is_ok());
        assert!(compile_source("function f(): int { while (true) { break; } }")
            .unwrap_err()
            .contains("non-void function must return a value on every path"));
        // A break in a nested loop leaves only that loop.
        assert!(compile_source("function f(): int { int[] xs = [1]; while (true) { foreach (xs as x) { break; } return 1; } }").is_ok());
    }

    #[test]
    fn logical_not_negates_bools() {
        let ir = compile_source("bool a = true; print(!a); print(!(1 > 2) && !false);").unwrap();
        assert!(ir.contains("= xor i1 %v1, true"), "{ir}");
        assert!(ir.contains("= xor i1 false, true"));
        let cases = [
            ("print(!5);", "'!' requires a bool; found int"),
            ("int n = 1; while (n) { n = 0; }", "while condition must be a bool, found int"),
            ("while true { }", "expected '(' after 'while'"),
            ("break;", "break is only valid inside a loop"),
        ];
        for (source, message) in cases {
            let error = compile_source(source).unwrap_err();
            assert!(error.contains(message), "{source}: expected '{message}', got '{error}'");
        }
    }

    #[test]
    fn loop_counters_are_allocated_in_the_entry_block() {
        let ir = compile_source("int i = 0; while (i < 3) { int[] xs = [1, 2]; foreach (xs as x) { print(x); } i = i + 1; }").unwrap();
        let main = main_ir(&ir);
        let entry_end = main.find("  br label %while.test").unwrap();
        let entry = &main[..entry_end];
        // Array literals live on the heap; only the foreach counter is a stack slot.
        assert!(!main.contains("alloca ["), "{main}");
        assert!(main.contains("call ptr @qiq_array_new(i64 2, i64 8, i32 "));
        assert!(entry.contains("%foreach.slot"), "{main}");
        assert_eq!(main.matches("alloca").count(), entry.matches("alloca").count(), "{main}");
    }

    #[test]
    fn functions_can_return_arrays_and_store_them() {
        let ir = compile_source("function make(): int[] { int[] xs = [1, 2]; return xs; } int[] got = make(); foreach (got as x) { print(x); } map[string, int[]] m = {\"a\" => make()}; int[] back = m[\"a\"]; print(back[0]);").unwrap();
        assert!(ir.contains("define ptr @make()"));
        assert!(ir.contains("%v6 = load ptr, ptr %v4, align 8\n  call void @qiq_retain(ptr %v6)\n  %release0 = load ptr, ptr %v4, align 8\n  call void @qiq_release(ptr %release0)\n  ret ptr %v6"), "{ir}");
        assert!(ir.contains("call i64 @qiq_array_length(ptr %v"), "{ir}");
        let errors = [
            ("int[] xs = 5;", "cannot initialize int[] variable 'xs' with int"),
            ("function f(): string[] { string[] s = [\"a\"]; return s; } int[] xs = f();", "cannot initialize int[] variable 'xs' with string[]"),
        ];
        for (source, message) in errors {
            let error = compile_source(source).unwrap_err();
            assert!(error.contains(message), "{source}: expected '{message}', got '{error}'");
        }
    }

    #[test]
    fn array_stdlib_functions_take_and_return_arrays() {
        let ir = compile_source("import file, scandir from \"std/file\"; string[] parts = explode(\",\", \"a,b\"); string joined = implode(\"-\", parts); string[] lines = file(\"x\"); string[] names = scandir(\".\");").unwrap();
        assert!(ir.contains("declare ptr @qiq_stdlib_explode(ptr, ptr, i64)"));
        assert!(ir.contains("declare ptr @qiq_stdlib_implode(ptr, ptr)"));
        assert!(ir.contains("declare ptr @qiq_stdlib_file(ptr)"));
        assert!(ir.contains("declare ptr @qiq_stdlib_scandir(ptr)"));
        assert!(ir.contains("i64 9223372036854775807)"), "{ir}");
        assert!(ir.contains("@qiq_raise"));
        assert!(ir.contains("c\"OutOfRangeException\\00\""));
    }

    #[test]
    fn array_variables_can_be_reassigned() {
        let ir = compile_source("int[] xs = [1]; int[] ys = [2, 3]; xs = ys; print(xs[1]); function f(string[] values): void { string[] other = [\"x\"]; values = other; }").unwrap();
        // xs lives in a stack slot; the assignment stores the other array's pointer there.
        assert!(main_ir(&ir).contains("store ptr %v"), "{ir}");
        assert!(ir.contains("define void @f(ptr %arg0)"));
        assert!(compile_source("string[] later; later = explode(\",\", \"a,b\");").is_ok());
        let cases = [
            ("int[] xs = [1]; string[] ys = [\"a\"]; xs = ys;", "cannot assign string[] to int[] variable 'xs'"),
            ("int[] xs = [1]; xs = 5;", "cannot assign int to int[] variable 'xs'"),
        ];
        // null is a valid array value.
        assert!(compile_source("int[] xs = [1]; xs = null;").is_ok());
        for (source, message) in cases {
            let error = compile_source(source).unwrap_err();
            assert!(error.contains(message), "{source}: expected '{message}', got '{error}'");
        }
    }

    #[test]
    fn array_and_map_literals_are_expressions() {
        let ir = compile_source("function total(int[] values): int { return 0; } int n = total([1, 2]); int[] xs = [3]; xs = [4, 5]; map[string, int[]] m = {\"a\" => [1]}; words = [\"x\"];").unwrap();
        assert!(ir.contains("call ptr @qiq_array_new(i64 2, i64 8, i32 "), "{ir}");
        assert!(ir.contains("call ptr @qiq_array_new(i64 1, i64 8, i32 "), "{ir}");
        // The map's values are arrays, which the map holds references to.
        assert!(ir.contains("call ptr @qiq_map_new(i32 3, i32 1)"), "{ir}");
        // A map literal's types come from its first entry when no type is expected.
        assert!(compile_source("scores = {\"ada\" => 1.5}; double d = scores[\"ada\"];").is_ok());
        assert!(compile_source("map[string, map[string, int]] nested = {\"a\" => {\"b\" => 1}};").is_ok());
    }

    #[test]
    fn empty_literals_take_their_type_from_context() {
        let ir = compile_source("int[] xs = [1]; xs = []; function f(string[] s): void {} f([]); function g(): map[string, int] { return {}; } map[string, bool] flags = {};").unwrap();
        assert!(ir.contains("call ptr @qiq_array_new(i64 0, i64 8, i32 "), "{ir}");
        assert!(ir.contains("call ptr @qiq_array_new(i64 0, i64 8, i32 "), "{ir}");
        assert!(ir.contains("call ptr @qiq_map_new(i32 3, i32 0)"));
        let cases = [
            ("x = [];", "cannot infer the type of an empty array literal []"),
            ("x = {};", "cannot infer the type of an empty map literal {}"),
            ("int n = [];", "an array literal cannot be used as int"),
            ("string s = {};", "a map literal cannot be used as string"),
            ("int[] xs = {1, 2};", "array literals use [...], e.g. int[] values = [1, 2];"),
            ("int[] xs = [1, true];", "array elements must all be int, found bool"),
            ("map[string, int] m = {\"a\" => \"b\"};", "map value requires int, found string"),
            ("map[string, int] m = {1 => 2};", "map key requires string, found int"),
            ("m = {\"a\" => 1, 2};", "expected '=>' between map key and value (array literals use [...])"),
        ];
        for (source, message) in cases {
            let error = compile_source(source).unwrap_err();
            assert!(error.contains(message), "{source}: expected '{message}', got '{error}'");
        }
    }

    #[test]
    fn map_variables_can_be_reassigned() {
        let ir = compile_source("map[string, int] ages = {\"ada\" => 36}; ages = {}; map[string, int] other = {\"lin\" => 29}; ages = other; print(ages[\"lin\"]); function f(map[string, int] m): void { m = {\"x\" => 1}; }").unwrap();
        // ages lives in a stack slot that each assignment stores into.
        assert!(main_ir(&ir).contains("alloca ptr"), "{ir}");
        assert!(ir.contains("define void @f(ptr %arg0)"));
        let cases = [
            ("map[string, int] m = {}; map[string, bool] k = {}; m = k;", "cannot assign map[string, bool] to map[string, int] variable 'm'"),
            ("map[string, int] m = {}; m = [1];", "cannot assign int[] to map[string, int] variable 'm'"),
            ("map[string, int] m = {}; m = {\"a\" => true};", "map value requires int, found bool"),
        ];
        for (source, message) in cases {
            let error = compile_source(source).unwrap_err();
            assert!(error.contains(message), "{source}: expected '{message}', got '{error}'");
        }
    }

    #[test]
    fn error_messages_show_types_as_written_in_qiq() {
        let cases = [
            ("map[string, int[]] m = {}; map[string, bool] k = {}; m = k;", "cannot assign map[string, bool] to map[string, int[]] variable 'm'"),
            ("map[string, map[int, char]] m = {}; m = 5;", "cannot assign int to map[string, map[int, char]] variable 'm'"),
            ("class Page {} Page p = new Page(); int n = p;", "cannot initialize int variable 'n' with Page"),
            ("function f(map[string, string] o): void {} f([1]);", "expected map[string, string], found int[]"),
            ("function g(): string[] { return 5; }", "return type mismatch: expected string[], found int"),
            ("double[] d = [1.5]; bool[] b = [true]; char[] c = ['x']; d = b;", "cannot assign bool[] to double[] variable 'd'"),
            ("function act(): void {} int n = act();", "cannot initialize int variable 'n' with null"),
            ("function f(int count = \"ten\"): void {}", "default for 'count' must be int, found string"),
        ];
        for (source, message) in cases {
            let error = compile_source(source).unwrap_err();
            assert!(error.contains(message), "{source}: expected '{message}', got '{error}'");
        }
    }

    #[test]
    fn arrays_can_hold_arrays_maps_and_objects() {
        let ir = compile_source("int[][] grid = [[1, 2], [3]]; int[] row = grid[1]; print(row[0]); grid[0] = [9]; int[][] empty = new int[][2]; map[string, int][] maps = [{\"a\" => 1}]; class Page {} Page[] pages = new Page[3]; pages[0] = new Page(); string[][][] cube = [[[\"x\"]]];").unwrap();
        // Arrays of pointers (arrays, maps, objects) use 8-byte elements.
        assert!(ir.contains("call ptr @qiq_array_new(i64 2, i64 8, i32 "), "{ir}");
        assert!(ir.contains("call ptr @qiq_array_new(i64 3, i64 8, i32 "), "{ir}");
        assert!(ir.contains("store ptr %v"), "{ir}");
        // Types are inferred through nesting, and empty inner literals take the element type.
        assert!(compile_source("xs = [[1], [2]]; int[] first = xs[0];").is_ok());
        assert!(compile_source("int[][] xs = [[], [1]];").is_ok());
        // Element types follow class assignability.
        assert!(compile_source("class Shape {} class Square extends Shape {} Shape[] shapes = [new Square(), new Shape()]; shapes[0] = new Square();").is_ok());
        let cases = [
            ("int[][] grid = [[1], [\"a\"]];", "array elements must all be int, found string"),
            ("int[][] grid = [[1]]; int[] row = grid;", "cannot initialize int[] variable 'row' with int[][]"),
            ("int[][] grid = [[1]]; grid[0] = 5;", "array element assignment requires int[], found int"),
            ("xs = [[], [1]];", "cannot infer the type of an empty array literal []"),
            ("int[][] grid = new int[2];", "cannot initialize int[][] variable 'grid' with int[]"),
            ("int n = 1; int[] xs = new int[\"a\"];", "array size must be an int, found string"),
        ];
        for (source, message) in cases {
            let error = compile_source(source).unwrap_err();
            assert!(error.contains(message), "{source}: expected '{message}', got '{error}'");
        }
    }

    #[test]
    fn print_writes_containers_and_several_values() {
        let ir = compile_source("int[][] g = [[1]]; map[string, bool[]] m = {}; class P {} print(g); print(m); print(new P()); print(\"a\", 1);").unwrap();
        let main = main_ir(&ir);
        // Descriptors: int[][] is "aai", map[string, bool[]] is "msab", objects are "o".
        assert!(ir.contains("c\"aai\\00\""), "{ir}");
        assert!(ir.contains("c\"msab\\00\""));
        assert!(ir.contains("c\"o\\00\""));
        assert!(main.contains("call void @qiq_print_value(ptr getelementptr"), "{main}");
        // Several values: separated by spaces, one newline at the end.
        assert!(main.contains("call void @qiq_print_space()"));
        assert_eq!(main.matches("call void @qiq_print_newline()").count(), 4, "{main}");
        // The class name table is visible to the runtime.
        assert!(ir.contains("@qiq_class_names = unnamed_addr constant ["));
    }

    #[test]
    fn printing_one_scalar_still_uses_printf() {
        let ir = compile_source("print(7); print(\"x\"); print(true);").unwrap();
        let main = main_ir(&ir);
        assert!(main.contains("call i32 (ptr, ...) @printf(ptr @.fmt.int, i64 7)"));
        assert!(!main.contains("@qiq_print_value"), "{main}");
        assert!(compile_source("print();").unwrap().contains("call void @qiq_print_newline()"));
        assert!(compile_source("function f(): void {} print(f());").is_ok());
    }

    #[test]
    fn bytes_are_unsigned_8_bit_values() {
        let ir = compile_source("byte b = 200; byte[] xs = [1, 255]; print(b); if (xs[1] > b) { print(1); } map[string, byte] m = {\"k\" => 7}; byte back = m[\"k\"];").unwrap();
        assert!(ir.contains("alloca i8"), "{ir}");
        assert!(ir.contains("store i8 200, ptr"));
        assert!(ir.contains("call ptr @qiq_array_new(i64 2, i64 1, i32 "));
        assert!(ir.contains("zext i8 %v"), "{ir}");
        assert!(ir.contains("icmp ugt i8"), "{ir}");
        // A literal compared with a byte is a byte.
        assert!(compile_source("byte[] d = [137]; if (d[0] == 137) { print(1); } if (80 != d[0]) { print(2); }").unwrap().contains("icmp eq i8 %v"));
        let cases = [
            ("byte b = 256;", "256 doesn't fit in a byte (0 to 255)"),
            ("byte b = -1;", "-1 doesn't fit in a byte (0 to 255)"),
            ("byte b = 1; byte c = b + b;", "found byte and byte (convert bytes with to_int)"),
            ("byte b = 1; int n = b;", "cannot initialize int variable 'n' with byte"),
            ("map[byte, int] m = {};", "map keys must use a scalar type"),
            ("function f(byte b = 300): void {}", "default for 'b' must be a byte (0 to 255), found 300"),
        ];
        for (source, message) in cases {
            let error = compile_source(source).unwrap_err();
            assert!(error.contains(message), "{source}: expected '{message}', got '{error}'");
        }
    }

    #[test]
    fn conversion_functions_cover_each_type() {
        let ir = compile_source("byte b = 9; double d = 2.5; print(to_int(b), to_int(d), to_int(true), to_int(\"7\"), to_double(3), to_double(b), to_double(\"1.5\"), to_byte(65), to_string(1), to_string(d), to_string(b), to_string(false), to_string('c'));").unwrap();
        for instruction in [
            "zext i8 %v",
            "call i64 @qiq_double_to_int(double",
            "zext i1 true to i64",
            "call i64 @qiq_string_to_int(ptr",
            "sitofp i64 3 to double",
            "uitofp i8 %v",
            "call double @qiq_string_to_double(ptr",
            "call zeroext i8 @qiq_int_to_byte(i64 65)",
            "call ptr @qiq_int_to_string(i64 1)",
            "call ptr @qiq_double_to_string(double",
            "select i1 false, ptr",
        ] {
            assert!(ir.contains(instruction), "missing {instruction}");
        }
        let cases = [
            ("print(to_int());", "to_int expects 1 argument, found 0"),
            ("print(to_int([1]));", "to_int cannot convert int[]"),
            ("print(to_byte(1.5));", "to_byte cannot convert double"),
            ("function to_int(int x): int { return x; }", "cannot redeclare built-in function 'to_int'"),
        ];
        for (source, message) in cases {
            let error = compile_source(source).unwrap_err();
            assert!(error.contains(message), "{source}: expected '{message}', got '{error}'");
        }
    }

    #[test]
    fn binary_files_and_byte_strings_are_built_in() {
        let ir = compile_source("import file_get_bytes, file_put_bytes from \"std/file\"; byte[] data = file_get_bytes(\"x\"); int n = file_put_bytes(\"y\", data, append: true); byte[] u = string_to_bytes(\"é\"); string s = bytes_to_string(u);").unwrap();
        assert!(ir.contains("declare ptr @qiq_stdlib_file_get_bytes(ptr)"));
        assert!(ir.contains("declare i64 @qiq_stdlib_file_put_bytes(ptr, ptr, i1 zeroext)"));
        assert!(ir.contains("declare ptr @qiq_stdlib_string_to_bytes(ptr)"));
        assert!(ir.contains("declare ptr @qiq_stdlib_bytes_to_string(ptr)"));
    }

    #[test]
    fn double_literals_accept_exponents() {
        let ir = compile_source("double a = 1e5; double b = 2.5E-3; double c = 6.02e+23; double d = -1.5e2; print(a, b, c, d);").unwrap();
        assert!(ir.contains("store double 1.00000000000000000e5"), "{ir}");
        // 2.5e-3 isn't exact in binary; LLVM shows the nearest double.
        assert!(ir.contains("store double 2.50000000000000005e-3"), "{ir}");
        assert!(ir.contains("store double 6.01999999999999996e23"), "{ir}");
        // A negative literal negates the positive one, as for other doubles.
        assert!(ir.contains("fneg double 1.50000000000000000e2"), "{ir}");
        // 1e5 is a double even without a decimal point.
        assert!(compile_source("int n = 1e5;").unwrap_err().contains("cannot initialize int variable 'n' with double"));
        assert!(compile_source("double big = 1e400;").unwrap_err().contains("1:14: double literal is out of range"));
        // An `e` without digits after it isn't an exponent.
        assert!(compile_source("double x = 2e;").is_err());
    }

    #[test]
    fn switch_tests_cases_in_order_without_fallthrough() {
        let ir = compile_source("int n = 2; switch (n) { case 1, 2: print(1); case 3: print(3); default: print(0); }").unwrap();
        let main = main_ir(&ir);
        assert!(main.contains("br i1 %v2, label %switch.case0.0, label %switch.test0.0.1"), "{main}");
        assert!(main.contains("label %switch.case0.1, label %switch.default0"), "{main}");
        // Each case body leaves the switch; none falls into the next.
        assert_eq!(main.matches("  br label %switch.end0\n").count(), 3, "{main}");
        // Strings compare by content.
        assert!(compile_source("switch (\"a\") { case \"a\": print(1); }").unwrap().contains("call i32 @qiq_string_equality"));
    }

    #[test]
    fn break_and_continue_inside_switch() {
        // break leaves the switch; continue goes to the enclosing loop.
        let ir = compile_source("foreach ([1] as n) { switch (n) { case 1: break; default: continue; } }").unwrap();
        let main = main_ir(&ir);
        assert!(main.contains("switch.case2.0:\n  br label %switch.end2"), "{main}");
        assert!(main.contains("switch.default2:\n  br label %foreach.increment1"), "{main}");
        // A switch where every branch returns ends the function's code path.
        assert!(compile_source("function f(int n): string { switch (n) { case 1: return \"a\"; default: return \"b\"; } }").is_ok());
        // break in a switch inside while(true) doesn't leave the loop.
        assert!(compile_source("function f(): int { int i = 0; while (true) { switch (i) { case 0: break; } return i; } }").is_ok());
    }

    #[test]
    fn rejects_invalid_switches() {
        let cases = [
            ("switch (1) { case 1: case 2: print(1); }", "1:14: an empty case does nothing because cases don't fall through"),
            ("switch (1) { case 1: print(1); case 1: print(2); }", "duplicate case value 1"),
            ("switch (\"a\") { case 1: print(1); }", "case value must be string, found int"),
            ("switch (1) { default: print(1); default: print(2); }", "a switch can have only one default"),
            ("switch (1) { case 1: continue; }", "continue is only valid inside a loop"),
            ("function f(int n): int { switch (n) { case 1: return 1; } }", "non-void function must return a value on every path"),
            ("int[] xs = [1]; switch (xs) { case xs: print(1); }", "switch requires an int, double, bool, byte, string, or char, found int[]"),
            ("switch (1) { }", "a switch needs at least one case"),
            ("switch (1) { print(1); }", "expected 'case' or 'default' in switch"),
            ("byte b = 1; switch (b) { case 300: print(1); }", "300 doesn't fit in a byte (0 to 255)"),
            ("break;", "break is only valid inside a loop or switch"),
        ];
        for (source, message) in cases {
            let error = compile_source(source).unwrap_err();
            assert!(error.contains(message), "{source}: expected '{message}', got '{error}'");
        }
    }

    #[test]
    fn final_classes_and_methods_cannot_be_extended_or_overridden() {
        // Final classes can be instantiated; final methods can be inherited and called.
        assert!(compile_source("final class A { function m(): int { return 1; } } A a = new A(); print(a->m());").is_ok());
        assert!(compile_source("class A { public final function m(): int { return 1; } } class B extends A {} print(new B()->m());").is_ok());
        assert!(compile_source("abstract class A { final public function run(): void { this->step(); } abstract protected function step(); } class B extends A { protected function step(): void {} }").is_ok());
        let cases = [
            ("final class A {} class B extends A {}", "1:36: class 'B' cannot extend final class 'A'"),
            ("class A { final function m(): void {} } class B extends A { function m(): void {} }", "method 'B::m' cannot override final method 'A::m'"),
            ("class A { final function m(): void {} } class B extends A {} class C extends B { function m(): void {} }", "method 'C::m' cannot override final method 'A::m'"),
            ("class A { final function __construct() {} } class B extends A { function __construct() {} }", "method 'B::__construct' cannot override final method 'A::__construct'"),
            ("abstract final class A {}", "a class can't be both abstract and final"),
            ("final abstract class A {}", "a class can't be both abstract and final"),
            ("final final class A {}", "'final' is repeated"),
            ("abstract class A { abstract final function m(); }", "method 'm' can't be both abstract and final"),
            ("class A { final private function m(): void {} }", "private method 'm' can't be final"),
            ("class A { final int x; }", "final can only modify a class or a method"),
            ("interface I { final function m(); }", "interface methods can't be final"),
            ("class A { public private function m(): void {} }", "a member modifier is repeated (or given two visibilities)"),
        ];
        for (source, message) in cases {
            let error = compile_source(source).unwrap_err();
            assert!(error.contains(message), "{source}: expected '{message}', got '{error}'");
        }
    }

    #[test]
    fn static_methods_are_called_without_an_object() {
        let ir = compile_source("class M { static function twice(int n): int { return n * 2; } static function four(): int { return self::twice(2); } } print(M::twice(3), M::four());").unwrap();
        // No `this` parameter, a hidden called-class id (for static::), and a
        // direct call: M::twice passes M's id, self::twice forwards its own.
        assert!(ir.contains("define i64 @qiq_M_twice(i64 %called, i64 %arg0)"), "{ir}");
        assert!(ir.contains("call i64 @qiq_M_twice(i64 %called, i64 2)"), "{ir}");
        let main = main_ir(&ir);
        assert!(main.contains("call i64 @qiq_M_twice(i64 ") && main.contains(", i64 3)"), "{main}");
        // Inherited, with named arguments, and as a statement (the result is
        // discarded, as with other calls).
        assert!(compile_source("class A { static function f(int a = 1, int b = 2): int { return a + b; } } class B extends A {} print(B::f(b: 5)); A::f();").is_ok());
        assert!(compile_source("class A { static function f(int a = 1, int b = 2): int { return a + b; } static function g(): void {} } class B extends A {} print(B::f(b: 5)); A::g();").is_ok());
        // Imported classes work too.
        let ir = compile_files(&[
            ("main.qiq", "import MathUtil from \"util\"; print(MathUtil::clamp(5));"),
            ("util.qiq", "class MathUtil { static function clamp(int n): int { return n; } }"),
        ])
        .unwrap();
        assert!(ir.contains("call i64 @qiq_util.1.MathUtil_clamp(i64 ") && ir.contains(", i64 5)"), "{ir}");
    }

    #[test]
    fn rejects_invalid_static_methods() {
        let cases = [
            ("class A { private int x; static function f(): int { return this->x; } }", "'this' is not available in static method 'f'"),
            ("class A { function m(): void {} } A::m();", "method 'A::m' is not static; call it on an object with ->"),
            ("class A { static function s(): void {} } A a = new A(); a->s();", "method 'A::s' is static; call it with A::s(...)"),
            ("print(self::f());", "self:: can only be used inside a class"),
            ("class A { function m(): void {} } class B extends A { static function m(): void {} }", "static method 'B::m' cannot override instance method 'A::m'"),
            ("class A { static function m(): void {} } class B extends A { function m(): void {} }", "instance method 'B::m' cannot override static method 'A::m'"),
            ("interface I { function m(); } class A implements I { static function m(): void {} }", "static method 'A::m' can't implement interface method 'I::m'"),
            ("abstract class A { abstract static function m(); }", "method 'm' can't be both abstract and static"),
            ("class A { static function __construct() {} }", "a constructor can't be static"),
            ("interface I { static function m(); }", "interface methods can't be static"),
            ("class A {} A::missing();", "class 'A' has no method 'missing'"),
            ("class A { private static function p(): void {} } A::p();", "cannot access non-public member 'p'"),
            ("Missing::f();", "class 'Missing' is not declared or imported"),
        ];
        for (source, message) in cases {
            let error = compile_source(source).unwrap_err();
            assert!(error.contains(message), "{source}: expected '{message}', got '{error}'");
        }
    }

    #[test]
    fn static_properties_store_one_value_per_class() {
        let ir = compile_source("class A { public static int n = 3; public static string s = \"x\"; public static int[] xs; static function bump(): void { self::n = self::n + 1; } } class B extends A { public static int n = 7; } A::bump(); print(A::n, B::n, A::s);").unwrap();
        assert!(ir.contains("@qiq_static.A.n = internal global i64 3, align 8"), "{ir}");
        assert!(ir.contains("@qiq_static.A.s = internal global ptr getelementptr"), "{ir}");
        assert!(ir.contains("@qiq_static.A.xs = internal global ptr null, align 8"));
        assert!(ir.contains("@qiq_static.B.n = internal global i64 7, align 8"));
        assert!(ir.contains("load i64, ptr @qiq_static.A.n"));
        assert!(ir.contains("store i64 %v"));
        let cases = [
            ("class A { public static int n = 0; } A::n = \"x\";", "static property 'n' requires int, found string"),
            ("class A { private static int n = 0; } print(A::n);", "cannot access non-public member 'n'"),
            ("class A { public static int n; } class B extends A { public static string n; }", "static property 'B::n' must keep the type int of 'A::n'"),
            ("class A {} print(A::missing);", "class 'A' has no constant or static property 'missing'"),
            ("class A { public static int n = 1 + 1; }", "default for 'n' must be a literal"),
            ("class A { public static int n; public int n; }", "property 'n' is already declared"),
            ("class A { public function f(): void {} public function f(): void {} }", "1:56: class 'A' declares method 'f' more than once"),
            ("class A { public function f(): void {} public static function f(int x): int { return x; } }", "class 'A' declares method 'f' more than once"),
            ("class A { public function __construct() {} public function __construct(int x) {} }", "class 'A' declares method '__construct' more than once"),
        ];
        for (source, message) in cases {
            let error = compile_source(source).unwrap_err();
            assert!(error.contains(message), "{source}: expected '{message}', got '{error}'");
        }
    }

    #[test]
    fn late_static_binding_uses_the_called_class() {
        let ir = compile_source("class A { protected static string t = \"a\"; static function name(): string { return static::t; } static function make(): static { return new static(); } } class B extends A { protected static string t = \"b\"; function hi(): string { return \"hi\"; } } B b = B::make(); print(b->hi(), A::name());").unwrap();
        assert!(ir.contains("@qiq_static_slots.A.t = private unnamed_addr constant ["), "{ir}");
        assert!(ir.contains("ptr @qiq_static.A.t") && ir.contains("ptr @qiq_static.B.t"));
        assert!(ir.contains("@qiq_new_static.A = private unnamed_addr constant ["));
        assert!(ir.contains("define ptr @qiq_new.B()"));
        // static:: in an instance method uses the object's class.
        assert!(compile_source("class A { protected static int n = 1; function get(): int { return static::n; } }").unwrap().contains("load i64, ptr %this"));
        let cases = [
            ("print(static::x);", "static:: can only be used inside a class"),
            ("class A { function m(): void {} static function s(): void { static::m(); } }", "static:: can only call static methods; 'A::m' is an instance method"),
            ("function f(): void { print(new static()); }", "new static can only be used inside a class"),
            ("class A { static function make(): static { return new static(); } } class B extends A { function __construct(int x) {} }", "new static in 'A' needs every subclass to take the same constructor arguments, but 'B' changes them"),
            ("function f(): static { return 1; }", "the return type static is only valid in a class"),
            ("class A { function m(): static { return this; } } class B extends A { function m(): B { return this; } }", "method 'B::m' does not match the inherited signature"),
        ];
        for (source, message) in cases {
            let error = compile_source(source).unwrap_err();
            assert!(error.contains(message), "{source}: expected '{message}', got '{error}'");
        }
    }

    #[test]
    fn parent_calls_the_parent_version() {
        let ir = compile_source("class A { function __construct(int n) {} function d(): string { return \"a\"; } } class B extends A { function __construct() { parent::__construct(1); } function d(): string { return parent::d() + \"b\"; } }").unwrap();
        assert!(ir.contains("call void @qiq_A___construct(ptr %this, i64 1)"));
        assert!(ir.contains("call ptr @qiq_A_d(ptr %this)"), "{ir}");
        // An empty parent::__construct() is allowed when the parent has no constructor.
        assert!(compile_source("class A {} class B extends A { function __construct() { parent::__construct(); } }").is_ok());
        assert!(compile_source("abstract class A { abstract function m(); } class B extends A { function m(): void { parent::m(); } }")
            .unwrap_err()
            .contains("cannot call abstract method 'A::m'"));
    }
}
