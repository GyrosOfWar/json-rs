#![feature(test, str_char)]

extern crate time;
extern crate test;

use std::collections::HashMap;
use std::fmt;
use std::env::args;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;

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
    fn get_string(self) -> Option<String> {
        match self {
            JsonValue::Str(s) => Some(s),
            _ => None
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum JsonError {
    UnclosedStringLiteral,
    UnclosedArray,
    UnclosedObject,
    MissingColon,
    ExpectedBool,
    NumberParsing,
    ExpectedColon,
    EndOfFile,
    ExpectedNull,
    Other
}

impl JsonError {
    pub fn description(&self) -> &str {
        match *self {           
            JsonError::UnclosedStringLiteral => "Unclosed string literal",
            JsonError::UnclosedArray => "Unclosed array bracket",
            JsonError::UnclosedObject => "Unclosed object bracket",
            JsonError::MissingColon => "Missing colon",
            JsonError::ExpectedBool => "Expected true or false",
            JsonError::NumberParsing => "Error parsing number",
            JsonError::ExpectedColon => "Expected colon",
            JsonError::EndOfFile => "End of file reached",
            JsonError::ExpectedNull => "Expected null",
            JsonError::Other => "Unknown error"
        }
    }
}


impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

pub type JsonResult = Result<JsonValue, JsonError>;


// TODO add line and col
// TODO replace input string/pos by <T: Iterator<Item = char>>
pub struct JsonParser<'a> {
    input: &'a str,
    pos: usize
}

use JsonValue::*;
use JsonError::*;

impl<'a> JsonParser<'a> {
    pub fn new(input: &'a str) -> JsonParser<'a> {
        JsonParser {
            input: input, 
            pos: 0
        }
    }

    #[inline]
    fn consume_char(&mut self) -> char {
        let mut iter = self.input[self.pos..].char_indices();
        let (_, cur_char) = iter.next().expect("Failed to get the next character");
        let (next_pos, _) = iter.next().unwrap_or((1, ' '));
        self.pos += next_pos;
        return cur_char;
    }

    #[inline]
    fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    #[inline]
    fn peek_next(&self) -> char {
        self.input[self.pos..].chars().next().expect("Failed to peek next")
    } 

    fn consume_text(&mut self, text: &str) -> Option<String> {
        let mut buf = String::new();
        self.consume_whitespace();

        for c in text.chars() {
            let d = self.peek_next();
            if c != d {
                return None;
            }
            self.consume_char();
            buf.push(d);
            
        }
        self.consume_whitespace();

        Some(buf)
    }

    #[inline]
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
            Some(_) => Ok(Null),
            None => Err(ExpectedNull)
        }
    }

    fn parse_num(&mut self) -> JsonResult {
        self.consume_whitespace();
        let c = self.peek_next();
        
        if c.is_digit(10) || c == '-' {
            let num_str = self.consume_while(|c| c.is_digit(10) || c == '.' || c == 'e' || c == 'E' || c == '-' || c == '+');
           
            let n = num_str.parse::<f64>();
            match n {
                Ok(num) => return Ok(Num(num)),
                Err(why) => {
                    return Err(NumberParsing);
                }
            }
        
        } else {
            Err(NumberParsing)
        }
    }

    fn parse_string(&mut self) -> JsonResult {
        self.consume_whitespace();
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
                    Ok(Str(s))
                } else {
                    Err(UnclosedStringLiteral)
                }
            },
            _ => Err(UnclosedStringLiteral)
        }
    }

    fn parse_bool(&mut self) -> JsonResult {
        self.consume_whitespace();

        let c = self.peek_next();
        match c {
            'f' => {
                self.consume_text("false");
                Ok(Bool(false))
            }
            't' => {
                self.consume_text("true");
                Ok(Bool(true))
            }
            _ => {
                Err(ExpectedBool)
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
        let mut most_recent_error: Option<JsonError> = None;
        for result in p {
            match result {
                r @ Ok(_) => return r,
                Err(e) => most_recent_error = Some(e)
            }
        }
        
        Err(most_recent_error.expect("Bug!"))
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
                        e @ Err(_) => return e
                    }

                    let next = self.peek_next();
                    if next == ',' {
                        self.consume_char();
                        continue;
                    }
                    if next == ']' {
                        self.consume_char();
                        return Ok(Array(array));
                    }
                }
            }
            _ => Err(UnclosedArray)
        }
    }

    fn parse_object(&mut self) -> JsonResult {
        if self.eof() {
            return Err(EndOfFile);
        }
        self.consume_whitespace();
        let c = self.peek_next();
        match c {
            '{' => {
                let mut object = HashMap::new();
                self.consume_char();
                loop {
                    self.consume_whitespace();

                    let key = self.parse_string();
                    let key_string = match key {
                        Ok(s) => s.get_string().unwrap(),
                        Err(why) => return Err(why)
                    };

                    self.consume_whitespace();

                    let next = self.peek_next();
                    if next != ':' {
                        return Err(ExpectedColon);
                    }
                    self.consume_whitespace();

                    // Consume the colon
                    self.consume_char();
                    let value = self.parse_value();
                    match value {
                        Ok(v) => object.insert(key_string, v),
                        e @ Err(_) => return e
                    };
                    self.consume_whitespace();
                    
                    let next = self.peek_next();
                    if next == ',' {
                        self.consume_char();
                        continue;
                    }
                    if next == '}' {
                        self.consume_char();
                        return Ok(Object(object));
                    }
                }
            },
            _ => Err(UnclosedObject)
        }
    }

    pub fn parse(&mut self) -> JsonResult {
        self.parse_value()
    }

}

fn first_ok(results: Vec<JsonResult>) -> JsonResult {
    for r in results.iter() {
        if r.is_ok() {
            return r.clone();
        }
    }
    results[0].clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::JsonValue::*;
    use super::JsonError::*;
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::prelude::*;
    use std::io::BufReader;
    use std::path::Path;
    use test::*;

    #[test]
    fn parse_null() {
        let mut parser = JsonParser::new("   null  ");
        let result = parser.parse_null();
        assert_eq!(result, Ok(Null));
    }

    #[test]
    fn parse_number() {
        let mut parser = JsonParser::new("  4.2342 ");

        let result = parser.parse_num();
        assert_eq!(result, Ok(Num(4.2342)));
    }

    #[test]
    fn parse_number_2() {
        let mut parser = JsonParser::new("  16237  ");
        let result = parser.parse_num();
        assert_eq!(result, Ok(Num(16237.0)));
    }

    #[test]
    fn parse_number_error() {
        let mut parser = JsonParser::new("  abcdef  ");
        let result = parser.parse_num();
        match result {
            Ok(_) => assert!(false),
            Err(e) => assert_eq!(e, NumberParsing)
        }
    }

    #[test]
    fn parse_string() {
        let mut parser = JsonParser::new("  \"String\" ");
        let result = parser.parse_string();
        assert_eq!(result, Ok(Str("String".to_string())));
    }

    #[test]
    fn parse_string_error() {
        let mut parser = JsonParser::new("\"String");
        let result = parser.parse_string();
        match result {
            Ok(_) => assert!(false),
            Err(err) => assert_eq!(err, UnclosedStringLiteral)
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
        let mut parser = JsonParser::new("[ true , true , true ]");
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
        let mut parser = JsonParser::new("[1.2, 4.2, 1.2, 4.5]");
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
        let mut parser = JsonParser::new("[[true, true], [true, false]]");
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
        let mut parser = JsonParser::new("{\"label\" : 1.5}");
        let result = parser.parse_object();

        let mut obj = HashMap::new();
        obj.insert("label".to_string(), Num(1.5));

        assert_eq!(Object(obj), result.unwrap());
    }

    #[test]
    fn parse_object_array() {
        let mut parser = JsonParser::new("{\"label\" : [true, true, true]}");
        let result = parser.parse_object();

        let mut obj = HashMap::new();
        obj.insert("label".to_string(), Array(vec![Bool(true), Bool(true), Bool(true)]));

        assert_eq!(Object(obj), result.unwrap());

    }
    
    fn big_json(count: usize) -> String {
        let mut src = "[\n".to_string();
        for _ in 0..count {
            src.push_str(r#"{ "a": true, "b": null, "c":3.1415, "d": "Hello world", "e": \
                            [1,2,3]},"#);
        }
        src.push_str("{}]");
        return src;
    }


    #[bench]
    fn parse_small(b: &mut Bencher) {
        let data = big_json(500);
        
        b.iter(|| {
            let mut parser = JsonParser::new(&data);
            black_box(parser.parse());
        });
    }

    #[bench]
    fn parse_big(b: &mut Bencher) {
        let data = big_json(5000);
        
        b.iter(|| {
            let mut parser = JsonParser::new(&data);
            black_box(parser.parse());
        });
    }
}

#[cfg(not(test))]
fn main() {
    let args: Vec<String> = args().skip(1).collect();
    let mut file = BufReader::new(File::open(args[0].clone()).unwrap());
    let mut data = String::new();
    file.read_to_string(&mut data);

    let mut parser = JsonParser::new(&data);
    let start = time::precise_time_ns();
    let parsed = parser.parse();
    let end = time::precise_time_ns();
    let duration = ((end as f64) - (start as f64)) / 1e9;

    println!("{:?}", parsed);
}
