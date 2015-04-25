use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>)
}

impl JsonValue {
    pub fn get_string(&self) -> Option<String> {
        match *self {
            JsonValue::Str(ref s) => Some(s.clone()),
            _ => None
        }
    }
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
        println!("c = {}", cur_char);
        return cur_char;
    }

    fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }
    
    fn peek_next(&self) -> char {
        self.input[self.pos..].chars().next().expect("Failed to peek next")
    }

    fn peek_current(&self) -> char {
        self.input[self.pos..].chars().nth(0).unwrap()
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
            None => parse_error("Expected null".to_string())
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
                    return parse_error(why.to_string());
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
                   parse_error("Unclosed string literal".to_string())
                }
            },
            _ => parse_error(format!("Expected double quote, got {}", c))
        }
    }

    fn parse_bool(&mut self) -> JsonResult {
        let c = self.peek_next();
        match c {
            'f' => {
                println!("parsing false");
                self.consume_text("false");
                Ok(JsonValue::Bool(false))
            }
            't' => {
                println!("parsing true");
                self.consume_text("true");
                Ok(JsonValue::Bool(true))
            }
            _ => {
                parse_error(format!("Expected true or false, got {}", c))
            }
        }
    }

    fn parse_value(&mut self) -> JsonResult {
        let p = vec![self.parse_bool(),
                     self.parse_string(),
                     self.parse_num(),
                     self.parse_null(),
                     self.parse_array(),
                     self.parse_object()];

        first_ok(p)
    }

    fn parse_array(&mut self) -> JsonResult {
        let c = self.peek_next();
        match c {
            '[' => {
                // Consume the opening bracket
                self.consume_char();
                let mut array = Vec::new();
                
                loop {
                    let value = self.parse_value();
                    match value {
                        Ok(v) => array.push(v),
                        Err(why) => { return parse_error(format!("{:?}", why)); }
                    }
                    let next = self.peek_next();
                    if next == ',' {
                        self.consume_char();
                        continue;
                    }
                    if next == ']' {
                        self.consume_char();
                        return Ok(JsonValue::Array(array));
                    }
                }
            }
            _ => parse_error(format!("Got {}, expected [", c))
        }
    }

    fn parse_object(&mut self) -> JsonResult {
        if self.eof() {
            return parse_error("EOF".to_string());
        }
        
        let c = self.peek_next();
        match c {
            '{' => {
                let mut object = HashMap::new();
                self.consume_char();
                loop {
                    println!("Trying to parse string..");
                    let key = self.parse_string();
                    let key_string = match key {
                        Ok(s) => s.get_string().unwrap(),
                        Err(why) => return parse_error(format!("{:?}", why))
                    };

                    println!("key: {}", key_string);
                    
                    let next = self.peek_next();
                    if next != ':' {
                        return parse_error(format!("Expected :, got {}", next));
                    }
                    // Consume the colon
                    self.consume_char();
                    let value = self.parse_value();
                    println!("value = {:?}", value);
                    match value {
                        Ok(v) => object.insert(key_string, v),
                        Err(why) => return parse_error(format!("{:?}", why))
                    };

                    println!("next character = {}", next);
                    let next = self.peek_next();
                    if next == ',' {
                        self.consume_char();
                        println!("Found ',', continuing");
                        continue;
                    }
                    if next == '}' {
                        self.consume_char();
                        println!("returning {:?}", object);
                        return Ok(JsonValue::Object(object));
                    }
                }
            },
            _ => parse_error(format!("Got {}, expected {{", c))
        }
    }

}

fn parse_error(s: String) -> JsonResult {
    Err(JsonError::ParseError(s))
}

fn first_ok(results: Vec<JsonResult>) -> JsonResult {
    for r in results.iter() {
        if r.is_ok() {
            return r.clone();
        }
    }
    results[0].clone()
}

mod tests {
    use super::*;
    use super::JsonValue::*;
    use std::collections::HashMap;

    #[test]
    fn parse_null() {
        let mut parser = JsonParser::new("null");
        let result = parser.parse_null();
        assert_eq!(result, Ok(Null));
    }

    #[test]
    fn parse_number() {
        let mut parser = JsonParser::new("4.2342");

        let result = parser.parse_num();
        assert_eq!(result, Ok(Num(4.2342)));
    }

    #[test]
    fn parse_number_2() {
        let mut parser = JsonParser::new("16237");
        let result = parser.parse_num();
        assert_eq!(result, Ok(Num(16237.0)));
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
        assert_eq!(result, Ok(Str("String".to_string())));
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
        assert_eq!(result, Ok(Bool(false)));

        parser = JsonParser::new("true");
        let result = parser.parse_bool();
        assert_eq!(result, Ok(Bool(true)));
    }

    #[test]
    fn parse_bool_array() {
        let mut parser = JsonParser::new("[true,true,true]");
        let result = parser.parse_array();
        match result {
            Ok(val) => {
                let expected = Array(vec![Bool(true), Bool(true), Bool(true)]);
                assert_eq!(val, expected);
            }
            Err(why) => {
                panic!("{:?}", why);
            }
        }
    }

    #[test]
    fn parse_num_array() {
        let mut parser = JsonParser::new("[1.2,4.2,1.2,4.5]");
        let result = parser.parse_array();
        match result {
            Ok(value) => {
                let expected = Array(vec![Num(1.2), Num(4.2), Num(1.2), Num(4.5)]);
                assert_eq!(expected, value);
            }
            Err(err) => {
                panic!("{:?}", err);
            }
        }
    }

    #[test]
    fn parse_nested_array() {
        let mut parser = JsonParser::new("[[true,true],[true,false]]");
        let result = parser.parse_value();
        match result {
            Ok(value) => {
                let expected = Array(vec![
                    Array(vec![Bool(true), Bool(true)]),
                    Array(vec![Bool(true), Bool(false)])]);
                assert_eq!(expected, value);
            }
            Err(err) => {
                panic!("{:?}", err);
            }
        }
    }

    #[test]
    fn parse_object_simple() {
        let mut parser = JsonParser::new("{\"label\":1.5}");
        let result = parser.parse_object();

        let mut obj = HashMap::new();
        obj.insert("label".to_string(), Num(1.5));

        assert_eq!(Object(obj), result.unwrap());
    }
}
fn main() {
}
