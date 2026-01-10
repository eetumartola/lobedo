use super::value::Value;

#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Assign { target: String, expr: Expr },
}

#[derive(Debug, Clone)]
pub enum Expr {
    Literal(Value),
    Attr(String),
    Ident(String),
    Swizzle {
        expr: Box<Expr>,
        mask: String,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    Pos,
    Neg,
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Ident(String),
    Number(f32),
    At,
    Dot,
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
    Comma,
    Equal,
    Semicolon,
}

pub fn parse_program(code: &str) -> Result<Program, String> {
    let tokens = tokenize(code)?;
    let mut parser = Parser::new(tokens);
    let mut statements = Vec::new();
    while !parser.is_end() {
        parser.consume_separators();
        if parser.is_end() {
            break;
        }
        let stmt = parser.parse_statement()?;
        statements.push(stmt);
        parser.consume_separators();
    }
    Ok(Program { statements })
}

fn tokenize(code: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = code.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' | '\t' | '\r' => {
                i += 1;
            }
            '\n' | ';' => {
                tokens.push(Token::Semicolon);
                i += 1;
            }
            '+' => {
                tokens.push(Token::Plus);
                i += 1;
            }
            '-' => {
                tokens.push(Token::Minus);
                i += 1;
            }
            '*' => {
                tokens.push(Token::Star);
                i += 1;
            }
            '/' => {
                if i + 1 < chars.len() && chars[i + 1] == '/' {
                    i += 2;
                    while i < chars.len() && chars[i] != '\n' {
                        i += 1;
                    }
                } else {
                    tokens.push(Token::Slash);
                    i += 1;
                }
            }
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            ',' => {
                tokens.push(Token::Comma);
                i += 1;
            }
            '=' => {
                tokens.push(Token::Equal);
                i += 1;
            }
            '@' => {
                tokens.push(Token::At);
                i += 1;
            }
            '.' => {
                if i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
                    let start = i;
                    i += 1;
                    while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                        i += 1;
                    }
                    let number: f32 = chars[start..i]
                        .iter()
                        .collect::<String>()
                        .parse()
                        .map_err(|_| "Invalid number literal".to_string())?;
                    tokens.push(Token::Number(number));
                } else {
                    tokens.push(Token::Dot);
                    i += 1;
                }
            }
            '0'..='9' => {
                let start = i;
                i += 1;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let number: f32 = chars[start..i]
                    .iter()
                    .collect::<String>()
                    .parse()
                    .map_err(|_| "Invalid number literal".to_string())?;
                tokens.push(Token::Number(number));
            }
            '_' | 'a'..='z' | 'A'..='Z' => {
                let start = i;
                i += 1;
                while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let ident = chars[start..i].iter().collect::<String>();
                tokens.push(Token::Ident(ident));
            }
            _ => {
                return Err(format!("Unexpected character '{}'", c));
            }
        }
    }
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn is_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn consume_separators(&mut self) {
        while matches!(self.peek(), Some(Token::Semicolon)) {
            self.pos += 1;
        }
    }

    fn parse_statement(&mut self) -> Result<Statement, String> {
        self.expect(Token::At)?;
        let target = match self.next() {
            Some(Token::Ident(name)) => name,
            _ => return Err("Expected attribute name after '@'".to_string()),
        };
        self.expect(Token::Equal)?;
        let expr = self.parse_expr()?;
        Ok(Statement::Assign { target, expr })
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_add_sub()
    }

    fn parse_add_sub(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_mul_div()?;
        loop {
            match self.peek() {
                Some(Token::Plus) => {
                    self.pos += 1;
                    let right = self.parse_mul_div()?;
                    expr = Expr::Binary {
                        op: BinaryOp::Add,
                        left: Box::new(expr),
                        right: Box::new(right),
                    };
                }
                Some(Token::Minus) => {
                    self.pos += 1;
                    let right = self.parse_mul_div()?;
                    expr = Expr::Binary {
                        op: BinaryOp::Sub,
                        left: Box::new(expr),
                        right: Box::new(right),
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_mul_div(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_unary()?;
        loop {
            match self.peek() {
                Some(Token::Star) => {
                    self.pos += 1;
                    let right = self.parse_unary()?;
                    expr = Expr::Binary {
                        op: BinaryOp::Mul,
                        left: Box::new(expr),
                        right: Box::new(right),
                    };
                }
                Some(Token::Slash) => {
                    self.pos += 1;
                    let right = self.parse_unary()?;
                    expr = Expr::Binary {
                        op: BinaryOp::Div,
                        left: Box::new(expr),
                        right: Box::new(right),
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Some(Token::Plus) => {
                self.pos += 1;
                let expr = self.parse_unary()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Pos,
                    expr: Box::new(expr),
                })
            }
            Some(Token::Minus) => {
                self.pos += 1;
                let expr = self.parse_unary()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: Box::new(expr),
                })
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;
        loop {
            if !matches!(self.peek(), Some(Token::Dot)) {
                break;
            }
            self.pos += 1;
            let mask = match self.next() {
                Some(Token::Ident(name)) => name,
                _ => return Err("Expected swizzle mask after '.'".to_string()),
            };
            expr = Expr::Swizzle {
                expr: Box::new(expr),
                mask,
            };
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.next() {
            Some(Token::Number(value)) => Ok(Expr::Literal(Value::Float(value))),
            Some(Token::At) => match self.next() {
                Some(Token::Ident(name)) => Ok(Expr::Attr(name)),
                _ => Err("Expected attribute name after '@'".to_string()),
            },
            Some(Token::Ident(name)) => {
                if matches!(self.peek(), Some(Token::LParen)) {
                    self.pos += 1;
                    let mut args = Vec::new();
                    if !matches!(self.peek(), Some(Token::RParen)) {
                        loop {
                            args.push(self.parse_expr()?);
                            match self.peek() {
                                Some(Token::Comma) => {
                                    self.pos += 1;
                                }
                                Some(Token::RParen) => break,
                                _ => {
                                    return Err("Expected ',' or ')' in function call".to_string())
                                }
                            }
                        }
                    }
                    self.expect(Token::RParen)?;
                    Ok(Expr::Call { name, args })
                } else if name == "PI" {
                    Ok(Expr::Literal(Value::Float(std::f32::consts::PI)))
                } else if name == "E" {
                    Ok(Expr::Literal(Value::Float(std::f32::consts::E)))
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            Some(Token::LParen) => {
                let expr = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            other => Err(format!("Unexpected token {:?}", other)),
        }
    }

    fn expect(&mut self, token: Token) -> Result<(), String> {
        match self.next() {
            Some(t) if t == token => Ok(()),
            other => Err(format!("Expected {:?}, got {:?}", token, other)),
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        if self.pos >= self.tokens.len() {
            return None;
        }
        let token = self.tokens[self.pos].clone();
        self.pos += 1;
        Some(token)
    }
}
