//#![feature(test)]

extern crate time;
//extern crate test;

use std::collections::HashMap;
use std::fmt;
use std::env::args;
use std::fs::File;
use std::io::prelude::*;
use std::ops::Index;
use JsonValue::*;
use JsonError::*;


/// Representation of a JSON value. An array is
/// represented as a Vec of JSON values, an
/// object is a map from string keys to JSON values
/// and numbers are stored as f64 for simplicity.
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
    pub fn find(&self, idx: &str) -> Option<&JsonValue> {
	match self {
	    &Object(ref map) => map.get(idx),
	    _ => None
	}
    }
    
    pub fn get_string(self) -> Option<String> {
        match self {
            JsonValue::Str(s) => Some(s),
            _ => None
        }
    }

    pub fn get_bool(self) -> Option<bool> {
        match self {
            Bool(b) => Some(b),
            _ => None
        }
    }

    pub fn get_num(self) -> Option<f64> {
        match self {
            Num(n) => Some(n),
            _ => None
        }
    }

    pub fn get_array(self) -> Option<Vec<JsonValue>> {
        match self {
            Array(vec) => Some(vec),
            _ => None
        }
    }
    pub fn get_object(self) -> Option<HashMap<String, JsonValue>> {
        match self {
            Object(map) => Some(map),
            _ => None
        }
    }
       
}

/// Indexing a JSON array
impl Index<usize> for JsonValue {
    type Output = JsonValue;
    fn index(&self, index: usize) -> &JsonValue {
	match self {
	    &Array(ref vec) => &vec[index],
	    _ => panic!("Can only index arrays with usize!")
	}
    }
}

/// Indexing a JSON object
impl<'a> Index<&'a str> for JsonValue {
    type Output = JsonValue;
    fn index(&self, idx: &str) -> &JsonValue {
	self.find(idx).expect("Can only index objects with &str!")
    }
}

/// Only simple error codes for now, I should probably
/// wrap this in an actual Error struct that also stores
/// line/column information about where the error occurred.
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

/// Result of most parsing functions. Either we succeed in parsing
/// and a value is returned or ther was an error and we return
/// an error code.
pub type JsonResult = Result<JsonValue, JsonError>;

/// The parser stores an iterator over characters,
/// information about the current position (line/col)
/// and the current character.
pub struct JsonParser<T> {
    iter: T,
    line: usize,
    col: usize,
    ch: Option<char>
}

impl<T: Iterator<Item = char>> JsonParser<T> {
    pub fn new(input: T) -> JsonParser<T> {
        let mut parser = JsonParser {
            iter: input,
            line: 1,
            col: 0,
            ch: Some('\x00')
        };
        parser.consume_char();
        parser
    }

    // Advances the character iterator by one and returns the new character
    #[inline]
    fn consume_char(&mut self) -> char {
        self.ch = self.iter.next();
        if self.ch_is('\n') {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        self.ch.unwrap_or('\x00')
    }

    // Is the current character equal to c?
    #[inline]
    fn ch_is(&self, c: char) -> bool {
        self.ch == Some(c)
    }

    // Are we at the end of the file?
    #[inline]
    fn eof(&self) -> bool {
        self.ch.is_none()
    }

    // Advances the input by the length of the passed text.
    // If one of the characters in the input is not equal
    // to the corresponding character in the text, returns None.
    fn consume_text(&mut self, text: &str) -> Option<String> {
        let mut buf = String::new();
        self.consume_whitespace();

        for c in text.chars() {
            if !self.ch_is(c) {
                return None;
            }
            let d = self.consume_char();
            buf.push(d);
            
        }
        self.consume_whitespace();

        Some(buf)
    }

    #[inline]
    fn ch_is_digit(&self) -> bool {
        match self.ch.unwrap_or('\x00') {
            '0'...'9' => true,
            _ => false
        }
    }

    #[inline]
    fn ch_is_whitespace(&self) -> bool {
        self.ch_is(' ') || self.ch_is('\n') ||
            self.ch_is('\t') || self.ch_is('\r')
    }

    // Consumes whitespace until the next non-whitespace character is reached
    #[inline]
    fn consume_whitespace(&mut self) {
        while self.ch_is_whitespace() {
            self.consume_char();
        }
    }

    // Consumes a numerical literal and returns its value as a string.
    #[inline]
    fn consume_num(&mut self) -> String {
        let mut result = String::new();
        self.consume_whitespace();

        while self.ch_is_digit() || self.ch_is('.') || self.ch_is('e') || self.ch_is('E')
            || self.ch_is('E') || self.ch_is('-') || self.ch_is('+') {
                result.push(self.ch.unwrap());
                self.consume_char();
            }
        result
    }
    // Parses the JSON null value.
    fn parse_null(&mut self) -> JsonResult {
        match self.consume_text("null") {
            Some(_) => Ok(Null),
            None => Err(ExpectedNull)
        }
    }

    // Parses a JSON number.
    fn parse_num(&mut self) -> JsonResult {
        self.consume_whitespace();
        
        if self.ch_is_digit() || self.ch_is('-') {
            let num_str = self.consume_num();
            
            let n = num_str.parse::<f64>();
            match n {
                Ok(num) => return Ok(Num(num)),
                Err(_) => {
                    return Err(NumberParsing);
                }
            }
            
        } else {
            Err(NumberParsing)
        }
    }
    
    // Parses a JSON string value.
    fn parse_string(&mut self) -> JsonResult {
        self.consume_whitespace();
        
        if self.ch_is('"') {
            self.consume_char();
            let mut found_end = false;
            let mut s = String::new();
            while !self.eof() {
                if self.ch_is('"') {
                    found_end = true;
                    self.consume_char();
                    break;
                }
                s.push(self.ch.unwrap());
                self.consume_char();
            }
            if found_end {
                Ok(Str(s))
            } else {
                Err(UnclosedStringLiteral)
            }
        }
        else {
            Err(UnclosedStringLiteral)
        }
    }

    // Parses a JSON boolean.
    fn parse_bool(&mut self) -> JsonResult {
        self.consume_whitespace();
        
        if self.ch_is('f') {
            self.consume_text("false");
            return Ok(Bool(false));
        }
        if self.ch_is('t')  {
            self.consume_text("true");
            return Ok(Bool(true));
        }
        else {
            Err(ExpectedBool)
        }   
    }
    // Parses any JSON value, this is the entry point
    // for the parser. Tries each possible parse until
    // one fits. If there are no suitable parses,
    // returns the most recent error. Error handling
    // this way isn't exacly ideal because the most recent
    // error is not always the most fitting one.
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
    
    // Parses a JSON array of values. Example: [true, false, 1, "hello"]
    fn parse_array(&mut self) -> JsonResult {
        if self.ch_is('[') {
            // Consume the opening bracket
            self.consume_char();
            let mut array = Vec::new();
            
            loop {
                let value = self.parse_value();
                match value {
                    Ok(v) => array.push(v),
                    e @ Err(_) => return e
                }
                // Parse the next value in the array
                if self.ch_is(',') {
                    self.consume_char();
                    continue;
                }
                // Reached the end of the array, return it
                if self.ch_is(']') {
                    self.consume_char();
                    return Ok(Array(array));
                }
            }
        }
        else {
            Err(UnclosedArray)
        }
    }
    // Parses a JSON object. Example: {"key": [1, 2, 3]}
    fn parse_object(&mut self) -> JsonResult {
        if self.eof() {
            return Err(EndOfFile);
        }
        self.consume_whitespace();
        if self.ch_is('{') {
            let mut object = HashMap::new();
            self.consume_char();
            loop {
                self.consume_whitespace();
                // The key is always a string value.
                let key = self.parse_string();
                let key_string = match key {
                    Ok(s) => s.get_string().unwrap(),
                    Err(why) => return Err(why)
                };

                self.consume_whitespace();

                // The separating colon between key and value
                if !self.ch_is(':') {
                    return Err(ExpectedColon);
                }
                self.consume_whitespace();
                self.consume_char();

                // Parse any value
                let value = self.parse_value();
                match value {
                    Ok(v) => object.insert(key_string, v),
                    e @ Err(_) => return e
                };
                self.consume_whitespace();

                // Continue with the next value
                if self.ch_is(',') {
                    self.consume_char();
                    continue;
                }
                // End of the current object
                if self.ch_is('}') {
                    self.consume_char();
                    return Ok(Object(object));
                }
            }
        }
        
        else {
            Err(UnclosedObject)
        }
        
    }

    pub fn parse(&mut self) -> JsonResult {
        self.parse_value()
    }
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
    //use test::*;

    #[test]
    fn parse_null() {
        let mut parser = JsonParser::new("   null  ".chars());
        let result = parser.parse_null();
        assert_eq!(result, Ok(Null));
    }

    #[test]
    fn parse_number() {
        let mut parser = JsonParser::new("  4.2342 ".chars());

        let result = parser.parse_num();
        assert_eq!(result, Ok(Num(4.2342)));
    }

    #[test]
    fn parse_number_2() {
        let mut parser = JsonParser::new("  16237  ".chars());
        let result = parser.parse_num();
        assert_eq!(result, Ok(Num(16237.0)));
    }

    #[test]
    fn parse_number_error() {
        let mut parser = JsonParser::new("  abcdef  ".chars());
        let result = parser.parse_num();
        match result {
            Ok(_) => assert!(false),
            Err(e) => assert_eq!(e, NumberParsing)
        }
    }

    #[test]
    fn parse_string() {
        let mut parser = JsonParser::new("  \"String\" ".chars());
        let result = parser.parse_string();
        assert_eq!(result, Ok(Str("String".to_string())));
    }

    #[test]
    fn parse_string_error() {
        let mut parser = JsonParser::new("\"String".chars());
        let result = parser.parse_string();
        match result {
            Ok(_) => assert!(false),
            Err(err) => assert_eq!(err, UnclosedStringLiteral)
        }
        
    }

    #[test]
    fn parse_bool() {
        let mut parser = JsonParser::new("false".chars());
        let result = parser.parse_bool();
        assert_eq!(result, Ok(Bool(false)));

        parser = JsonParser::new("true".chars());
        let result = parser.parse_bool();
        assert_eq!(result, Ok(Bool(true)));
    }

    #[test]
    fn parse_bool_array() {
        let mut parser = JsonParser::new("[ true , true , true ]".chars());
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
        let mut parser = JsonParser::new("[1.2, 4.2, 1.2, 4.5]".chars());
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
        let mut parser = JsonParser::new("[[true, true], [true, false]]".chars());
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
        let mut parser = JsonParser::new("{\"label\" : 1.5}".chars());
        let result = parser.parse_object();

        let mut obj = HashMap::new();
        obj.insert("label".to_string(), Num(1.5));

        assert_eq!(Object(obj), result.unwrap());
    }

    #[test]
    fn parse_object_array() {
        let mut parser = JsonParser::new("{\"label\" : [true, true, true]}".chars());
        let result = parser.parse_object();

        let mut obj = HashMap::new();
        obj.insert("label".to_string(), Array(vec![Bool(true), Bool(true), Bool(true)]));

        assert_eq!(Object(obj), result.unwrap());

    }
    
    #[test]
    fn index_array() {
    	let mut parser = JsonParser::new("[1, 2, 3, 4, 5]".chars());
    	let result = parser.parse().unwrap();
    	for i in 1..6 {
    		assert_eq!(result[i-1], Num(i as f64));
    	}
    }
    
    #[test]
    fn index_object() {
    	let mut parser = JsonParser::new("{\"label\" : 1.5}".chars());
        let result = parser.parse_object().unwrap();
        let indexed = result["label"].clone();
        let expected = Num(1.5);
        assert_eq!(indexed, expected);
    }
    
    // fn big_json(count: usize) -> String {
    //     let mut src = "[\n".to_string();
    //     for _ in 0..count {
    //         src.push_str(r#"{ "a": true, "b": null, "c":3.1415, "d": "Hello world", "e": \
    //                         [1,2,3]},"#);
    //     }
    //     src.push_str("{}]");
    //     return src;
    // }

    // #[bench]
    // fn parse_small(b: &mut Bencher) {
    //     let data = big_json(500);
        
    //     b.iter(|| {
    //         let mut parser = JsonParser::new(data.chars());
    //         black_box(parser.parse());
    //     });
    // }

    // #[bench]
    // fn parse_big(b: &mut Bencher) {
    //     let data = big_json(5000);
        
    //     b.iter(|| {
    //         let mut parser = JsonParser::new(data.chars());
    //         black_box(parser.parse());
    //     });
    // }
}

#[cfg(not(test))]
fn main() {
    let args: Vec<String> = args().skip(1).collect();
    let path = args[0].clone();
    let mut file = File::open(path).unwrap();
    let mut data = String::new();
    file.read_to_string(&mut data).unwrap();
    let start = time::precise_time_ns();

    let duration_s = 5.0;
    
    let duration_ns = (duration_s * 1e9) as u64;
    let mut iters = 0;
    let file_size = 136306;
    
    loop {
        let elapsed = time::precise_time_ns() - start;
        if elapsed >= duration_ns {
            break;
        }
        let mut parser = JsonParser::new(data.chars());
        let result = parser.parse().unwrap();

        iters += 1;
    }
    let mbs_read = file_size as f64 * iters as f64 / (1000.0 * 1000.0);
    println!("{} MB/s", mbs_read / duration_s);
}
