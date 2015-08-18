#[macro_use]
extern crate nom;
use nom::{IResult, space, Needed, Err};
use nom::IResult::*;
use std::str;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter, Result};

pub struct Record{
    pub headers: HashMap<String, String>,
    pub content: Vec<u8>
}

impl<'a> Debug for Record {
    fn fmt(&self, form:&mut Formatter) -> Result {
        write!(form, "\nHeaders:\n");
        for (name, value) in &self.headers {
            write!(form, "{}", name);
            write!(form, ": ");
            write!(form, "{}", value);
            write!(form, "\n");
        }
        write!(form, "Content Length:{}\n", self.content.len());
        let s = match String::from_utf8(self.content.clone()){
            Ok(s) => s,
            Err(_) => {"Could not convert".to_string()}
        };
        write!(form, "Content :{:?}\n", s);
        write!(form, "\n")
    }
}

fn version_number(input: &[u8]) -> IResult<&[u8], &[u8]> {
    for (idx, chr) in input.iter().enumerate() {
        match *chr {
            46 | 48...57  => continue,
            _ => return IResult::Done(&input[idx..], &input[..idx]),
        }
    }
    IResult::Incomplete(Needed::Size(1))
}

//should allow utf8 chars too
fn just_about_everything(input: &[u8]) -> IResult<&[u8], &[u8]> {
    for (idx, chr) in input.iter().enumerate() {
        match *chr {
            32...126 => continue,
            _ => return IResult::Done(&input[idx..], &input[..idx]),
        }
    }
    IResult::Incomplete(Needed::Size(1))
}

fn token(input: &[u8]) -> IResult<&[u8], &[u8]> {
    for (idx, chr) in input.iter().enumerate() {
        match *chr {
            33 | 35...39 | 42 | 43 | 45 | 48...57 | 65...90 | 94...122 | 124 => continue,
            _ => return IResult::Done(&input[idx..], &input[..idx]),
        }
    }
    IResult::Incomplete(Needed::Size(1))
}

named!(init_line <&[u8], (&str, &str)>,
    chain!(
        tag!("\r")?                 ~
        tag!("\n")?                 ~
        tag!("WARC")                ~
        tag!("/")                   ~
        space?                      ~
        version: map_res!(version_number, str::from_utf8)~
        tag!("\r")?                 ~
        tag!("\n")                  ,
        || {("WARCVERSION", version)}
    )
);
named!(header_match <&[u8], (&str, &str)>,
    chain!(
        name: map_res!(token, str::from_utf8)~
        space?                      ~
        tag!(":")                   ~
        space?                      ~
        value: map_res!(just_about_everything, str::from_utf8)~
        tag!("\r")?                 ~
        tag!("\n")                  ,
        || {(name, value)}
    )
);

named!(header_aggregator<&[u8], Vec<(&str,&str)> >, many1!(header_match));

named!(warc_header<&[u8], ((&str, &str), Vec<(&str,&str)>) >,
    chain!(
        version: init_line          ~
        headers: header_aggregator  ~
        tag!("\r")?                 ~
        tag!("\n")                  ,
        move ||{(version, headers)}
    )
);

pub fn record(input: &[u8]) -> IResult<&[u8], Record>{
    let mut h: HashMap<String,  String> = HashMap::new();
    match warc_header(input) {
        IResult::Done(mut i, tuple_vec) => {
            let (name, version) = tuple_vec.0;
            h.insert(name.to_string(), version.to_string());
            let headers =  tuple_vec.1; // not need figure it out
            for &(k,ref v) in headers.iter() {
                h.insert(k.to_string(), v.clone().to_string());
            }
            let mut content = None;
            match h.get("Content-Length"){
                Some(length) => {
                    let mut length_number = length.parse::<usize>().unwrap();
                    println!("{:?} :: {:?}", length_number, i.len());
                    match h.get("WARC-Truncated"){
                        Some(_) =>{
                            length_number = std::cmp::min(length_number, i.len());
                        }
                        _ => {}
                    }
                    println!("{:?} :: {:?}", length_number, i.len());
                    content = Some(&i[0..length_number as usize]);
                    i = &i[length_number as usize ..];
                }
                _ => { }
            }
            match content {
                Some(content) => {
                    let record = Record{headers: h, content: content.to_vec()};
                    IResult::Done(i, record)
                }
                None => {
                    IResult::Incomplete(Needed::Size(1))
                }
            }
        },
        IResult::Incomplete(a)     => IResult::Incomplete(a),
        IResult::Error(a)          => IResult::Error(a)
    }
}

named!(record_complete <&[u8], Record >,
    chain!(
        record: record              ~
        tag!("\r")?                 ~
        tag!("\n")                  ~
        tag!("\r")?                 ~
        tag!("\n")                  ,
        move ||{record}
    )
);
named!(pub records<&[u8], Vec<Record> >, many1!(record_complete));
