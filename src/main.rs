use std::collections::HashMap;

// TODO refactor to Result<JsonValue, JsonError>
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
    ParseError(String)
}

#[derive(Debug, Clone, PartialEq)]
pub enum JsonError {
    ParseError(String)
}

pub type JsonResult = Result<JsonValue, JsonError>;

pub struct JsonParser<'a> {
    input: &'a str,
    pos: usize
}

impl<'a> JsonParser<'a> {
    pub fn new(input: &'a str) -> JsonParser<'a> {
        JsonParser {
            input: input, 
            pos: 0,
         
        }
    }

    fn consume_char(&mut self) -> char {
        let mut iter = self.input[self.pos..].char_indices();
        let (_, cur_char) = iter.next().expect("Failed to get the next character");
        let (next_pos, _) = iter.next().unwrap_or((1, ' '));
        self.pos += next_pos;
        println!("{}", cur_char);
        return cur_char;
    }

    fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }
    
    fn peek_next(&self) -> char {
        self.input[self.pos..].chars().next().expect("Failed to peek next")
    }

    fn consume_text(&mut self, text: &str) -> Option<String> {
        let mut buf = String::new();
        for c in text.chars() {
            let d = self.peek_next();
            if c != d {
                return None;
            }
            self.consume_char();
            buf.push(d);
            
        }
        Some(buf)
    }

    fn consume_whitespace(&mut self) {
        self.consume_while(|c| c.is_whitespace() || c == '\n' || c == '\r');
    }
    
    fn consume_while<F: Fn(char) -> bool>(&mut self, test: F) -> String {
        let mut result = String::new();
        while !self.eof() && test(self.peek_next()) {
            let c = self.consume_char();
            result.push(c);
        }
        
        result
    }

    fn parse_null(&mut self) -> JsonResult {
        match self.consume_text("null") {
            Some(_) => Ok(JsonValue::Null),
            None => Err(JsonError::ParseError("Expected null".to_string()))
        }
    }

    fn parse_num(&mut self) -> JsonResult {
        let c = self.peek_next();
        
        if c.is_digit(10) || c == '-' {
            let num_str = self.consume_while(|c| c.is_digit(10) || c == '.' || c == 'e' || c == 'E' || c == '-' || c == '+');
           
            let n = num_str.parse::<f64>();
            match n {
                Ok(num) => return Ok(JsonValue::Num(num)),
                Err(why) => {
                    return Err(JsonError::ParseError(why.to_string()));
                }
            }
        
        } else {
            Err(JsonError::ParseError(format!("Expected number, got {}", c)))
        }
    }

    fn parse_string(&mut self) -> JsonResult {
        let c = self.peek_next();
        match c {
            '\"' => {
                self.consume_char();
                let mut found_end = false;
                let mut s = String::new();
                while !self.eof() {
                    let c = self.consume_char();
                    if c == '"' {
                        found_end = true;
                        break;
                    }
                    s.push(c);
                }
                if found_end {
                    Ok(JsonValue::Str(s))
                } else {
                   Err(JsonError::ParseError("Unclosed string literal".to_string()))
                }
            },
            _ => Err(JsonError::ParseError(String::new()))
        }
    }

    fn parse_bool(&mut self) -> JsonResult {
        let c = self.peek_next();
        match c {
            'f' => {
                self.consume_text("false");
                Ok(JsonValue::Bool(false))
            }
            't' => {
                self.consume_text("true");
                Ok(JsonValue::Bool(true))
            }
            _ => {
                let err = format!("Expected true or false, got {}", c);
                Ok(JsonValue::ParseError(err))
            }
        }
    }

    fn parse_value(&mut self) -> JsonResult {
        self.parse_bool()
            .or(self.parse_string())
            .or(self.parse_num())
            .or(self.parse_null())
    }

    fn parse_array(&mut self) -> JsonResult {
        let c = self.peek_next();
        match c {
            '[' => {
                let mut array = Vec::new();
                self.consume_char();
                
                while !self.eof() && self.peek_next() != ']' {
                    println!("self.pos before parse_value: {}", self.pos);
                    let value = try!(self.parse_value());
                    println!("Parsed value: {:?}", value);
                    println!("self.pos after parse_value: {}", self.pos);
                    array.push(value);
                    let comma = self.peek_next();
                    if comma == ']' {
                        return Ok(JsonValue::Array(array));
                    }
                    if comma != ',' {
                        return Err(JsonError::ParseError(format!("expected comma, got {}", comma)));
                    }

                    self.consume_char();
                }
                
                Ok(JsonValue::Array(array))
            }
            _ => Err(JsonError::ParseError(format!("Expected [, got {}", c)))
            
        }
    }
}

#[test]
fn parse_null() {
    let mut parser = JsonParser::new("null");
    let result = parser.parse_null();
    assert_eq!(result, Ok(JsonValue::Null));
}

#[test]
fn parse_number() {
    let mut parser = JsonParser::new("4.2342");

    let result = parser.parse_num();
    assert_eq!(result, Ok(JsonValue::Num(4.2342)));
}

#[test]
fn parse_number_2() {
    let mut parser = JsonParser::new("16237");
    let result = parser.parse_num();
    assert_eq!(result, Ok(JsonValue::Num(16237.0)));
}

#[test]
fn parse_number_error() {
    let mut parser = JsonParser::new("abcdef");
    let result = parser.parse_num();
    match result {
        Ok(_) => { assert!(false); }
        Err(_) => { return; }
    }
}

#[test]
fn parse_string() {
    let mut parser = JsonParser::new("\"String\"");
    let result = parser.parse_string();
    assert_eq!(result, Ok(JsonValue::Str("String".to_string())));
}

#[test]
fn parse_string_error() {
    let mut parser = JsonParser::new("\"String");
    let result = parser.parse_string();
    match result {
        Ok(_) => { assert!(false); }
        Err(_) => { return; }
    }
    
}

#[test]
fn parse_bool() {
    let mut parser = JsonParser::new("false");
    let result = parser.parse_bool();
    assert_eq!(result, Ok(JsonValue::Bool(false)));

    parser = JsonParser::new("true");
    let result = parser.parse_bool();
    assert_eq!(result, Ok(JsonValue::Bool(true)));
}

#[test]
fn parse_array() {
    let mut parser = JsonParser::new("[true,true,true]");
    let result = parser.parse_array();
    match result {
        Ok(val) => {
            let expected = JsonValue::Array(vec![JsonValue::Bool(true), JsonValue::Bool(true), JsonValue::Bool(true)]);
            assert_eq!(val, expected);
        }
        Err(why) => {
            panic!("{:?}", why);
        }
    }
}

fn main() {

}
