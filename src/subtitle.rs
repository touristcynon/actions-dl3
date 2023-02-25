use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::str::FromStr;
use std::{
    fmt,
    io::{self, Result},
};

use nom::{
    branch::alt,
    bytes::complete::{tag, take_until1},
    character::{self, complete::multispace0},
    error::ErrorKind,
    error_position,
    multi::{many1, separated_list1},
    sequence::{delimited, preceded, separated_pair},
    IResult, Parser,
};

#[derive(Debug, PartialEq, Eq)]
pub struct Timestamp {
    hours: u8,
    minutes: u8,
    seconds: u8,
    milliseconds: u16,
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02}:{:02}:{:02},{:03}",
            self.hours, self.minutes, self.seconds, self.milliseconds
        )
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Subtitle {
    pub index: u16,
    timestamp_start: Timestamp,
    timestamp_end: Timestamp,
    pub text: String,
}

impl fmt::Display for Subtitle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}\n{} --> {}\n{}",
            self.index, self.timestamp_start, self.timestamp_end, self.text
        )
    }
}

fn parse_timestamp(input: &str) -> IResult<&str, Timestamp> {
    let (remain, result) =
        separated_list1(alt((tag(":"), tag(","))), character::complete::u16)(input)?;
    if result.len() != 4 {
        return Err(nom::Err::Failure(error_position!(
            "timestamp need 4 fields to parse, less or more",
            ErrorKind::Fail
        )));
    }
    Ok((
        remain,
        Timestamp {
            hours: result[0] as u8,
            minutes: result[1] as u8,
            seconds: result[2] as u8,
            milliseconds: result[3],
        },
    ))
}

fn parse_subtitle(input: &str) -> IResult<&str, Subtitle> {
    let number_fn = delimited(multispace0, character::complete::u16, multispace0);
    let timestamp = separated_pair(parse_timestamp, tag(" --> "), parse_timestamp);
    let timestamps_fn = delimited(multispace0, timestamp, multispace0);
    let text_fn = preceded(multispace0, take_until1("\n\n"));

    let (remain, ((index, (start, end)), text)) =
        number_fn.and(timestamps_fn).and(text_fn).parse(input)?;

    if text.contains(" --> ") {
        return Err(nom::Err::Error(error_position!(remain, ErrorKind::Fail)));
    }
    Ok((
        remain,
        Subtitle {
            index,
            timestamp_start: start,
            timestamp_end: end,
            text: text.trim().to_string(),
        },
    ))
}

fn parse_subtitle_stream(input: &str) -> IResult<&str, Vec<Subtitle>> {
    many1(parse_subtitle)(input)
}

#[derive(Debug)]
pub struct SubtitleStream(Vec<Subtitle>);

impl SubtitleStream {
    pub fn load_from_file(file: impl AsRef<Path>) -> Result<Self> {
        let mut input = String::from_utf8(std::fs::read(file)?)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        // remove utf8 BOM
        input = input.trim_start_matches('\u{feff}').to_string();
        if input.contains('\r') {
            input = input.replace('\r', "");
        }
        input.parse()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.len() == 0
    }
}

impl FromStr for SubtitleStream {
    type Err = io::Error;
    fn from_str(input: &str) -> Result<Self> {
        let (_, stream) = parse_subtitle_stream(input)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        Ok(Self(stream))
    }
}

impl IntoIterator for SubtitleStream {
    type Item = Subtitle;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a SubtitleStream {
    type Item = &'a Subtitle;
    type IntoIter = std::slice::Iter<'a, Subtitle>;
    fn into_iter(self) -> std::slice::Iter<'a, Subtitle> {
        self.0[..].iter()
    }
}

impl Deref for SubtitleStream {
    type Target = Vec<Subtitle>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> IntoIterator for &'a mut SubtitleStream {
    type Item = &'a mut Subtitle;
    type IntoIter = std::slice::IterMut<'a, Subtitle>;
    fn into_iter(self) -> Self::IntoIter {
        self.0[..].iter_mut()
    }
}

impl DerefMut for SubtitleStream {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl fmt::Display for SubtitleStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for line in self {
            write!(f, "{}\n\n\n", line)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_subtitle_should_work() {
        let input = "1
        00:00:01,140 --> 00:00:03,190
        I've been keeping a secret from you guys

        2
        00:00:03,190 --> 00:00:03,200
        I've been keeping a secret from you guys";
        let (remain, c) = parse_subtitle(input).unwrap();
        let expected = "

        2
        00:00:03,190 --> 00:00:03,200
        I've been keeping a secret from you guys";
        assert_eq!(remain, expected);
        assert_eq!(
            c,
            Subtitle {
                index: 1,
                timestamp_start: Timestamp {
                    hours: 0,
                    minutes: 0,
                    seconds: 1,
                    milliseconds: 140
                },
                timestamp_end: Timestamp {
                    hours: 0,
                    minutes: 0,
                    seconds: 3,
                    milliseconds: 190
                },
                text: "I've been keeping a secret from you guys".to_string()
            }
        );
    }

    #[test]
    fn subtitle_stream_parse_from_file_should_work() {
        let parsed_subs = SubtitleStream::load_from_file("../fixtures/we_have_baby.srt");
        assert!(parsed_subs.is_ok());
        assert_eq!(
            parsed_subs.unwrap().0[0],
            Subtitle {
                index: 1,
                timestamp_start: Timestamp {
                    hours: 0,
                    minutes: 0,
                    seconds: 1,
                    milliseconds: 140
                },
                timestamp_end: Timestamp {
                    hours: 0,
                    minutes: 0,
                    seconds: 3,
                    milliseconds: 190
                },
                text: "I've been keeping a secret from you guys".to_string()
            }
        );
    }

    #[test]
    #[should_panic]
    fn subtitle_parse_without_blank_line_should_panic() {
        let c = "1
        00:00:01,140 --> 00:00:03,190

        I've been keeping a secret from you guys
        2
        00:00:03,190 --> 00:00:03,200
        I've been keeping a secret from you guys
         ";
        parse_subtitle(c).unwrap();
    }
}
