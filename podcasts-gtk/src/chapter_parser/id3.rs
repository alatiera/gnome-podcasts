// id3.rs
//
// Copyright 2025 nee <nee-git@patchouli.garden>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::{Result, anyhow, bail};
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;

use crate::chapter_parser::Chapter;

struct Id3FrameHeader {
    header: [u8; 4],
    size: u32,
    /// Can be used for advanced Table Of Content (CTOC) parsing in a future MR.
    /// Or for handling separated chapters that should not be played continuously.
    /// "Set to 0 [..] This provides a hint as to whether the elements should be played as a continuous ordered sequence or played individually"
    #[allow(dead_code)]
    flags: u16,
}

impl Id3FrameHeader {
    fn read_from<T: Read>(buf: &mut BufReader<T>) -> Result<Id3FrameHeader> {
        let mut header: [u8; 4] = [0, 0, 0, 0];
        buf.read_exact(&mut header)?;

        let mut size_bytes: [u8; 4] = [0, 0, 0, 0];
        buf.read_exact(&mut size_bytes)?;

        let mut flags_bytes: [u8; 2] = [0, 0];
        buf.read_exact(&mut flags_bytes)?;

        Ok(Id3FrameHeader {
            header,
            size: u32::from_be_bytes(size_bytes),
            flags: u16::from_be_bytes(flags_bytes),
        })
    }
}

/// Implemented from doc:
/// https://web.archive.org/web/20120313123311/https://id3.org/id3v2-chapters-1.0
pub fn read_id3_chapter<T: Read>(buf: &mut BufReader<T>) -> Result<Chapter> {
    let frame = Id3FrameHeader::read_from(buf)?;
    if &frame.header == b"CHAP" {
        // CHAP frame
        let id = read_0_terminated_string_iso(buf, frame.size)?;

        let mut start_milisec: [u8; 4] = [0, 0, 0, 0];
        buf.read_exact(&mut start_milisec)?;

        let mut end_milisec: [u8; 4] = [0, 0, 0, 0];
        buf.read_exact(&mut end_milisec)?;

        // offset in the file in bytes, often just 0xffff
        let mut start_bytes: [u8; 4] = [0, 0, 0, 0];
        let _ = buf.read_exact(&mut start_bytes);

        // offset in the file in bytes, often just 0xffff
        let mut end_bytes: [u8; 4] = [0, 0, 0, 0];
        let _ = buf.read_exact(&mut end_bytes);

        // TIT2 frame (optional)
        let title = read_id3_tit(buf, b"TIT2").unwrap_or("".to_string());

        // TIT3 frame (optional)
        let description = read_id3_tit(buf, b"TIT3").unwrap_or("".to_string());

        Ok(Chapter {
            id,
            title,
            description,
            start: chrono::Duration::milliseconds(i32::from_be_bytes(start_milisec).into()),
            end: chrono::Duration::milliseconds(i32::from_be_bytes(end_milisec).into()),
        })
    } else {
        // CTOC "Table Of Contents" and APIC Images could be parsed here in the future.
        bail!("not a chapter");
    }
}

fn read_id3_tit<T: Read>(buf: &mut BufReader<T>, header: &[u8; 4]) -> Result<String> {
    let frame = Id3FrameHeader::read_from(buf)?;
    if &frame.header == header {
        // 00 – ISO-8859-1 (ASCII).
        // 01 – UCS-2 (UTF-16 encoded Unicode with BOM), in ID3v2.2 and ID3v2.3.
        // 02 – UTF-16BE encoded Unicode without BOM, in ID3v2.4.
        // 03 – UTF-8 encoded Unicode, in ID3v2.4.
        let mut encoding: [u8; 1] = [0];
        buf.read_exact(&mut encoding)?;

        match encoding {
            [0] => read_0_terminated_string_iso(buf, frame.size - 1), // -1 for encoding byte
            [1] => read_0_terminated_string_u16_ucs(buf, frame.size - 1),
            [2] => read_0_terminated_string_u16(buf, frame.size - 1),
            [3] => read_0_terminated_string_u8(buf, frame.size - 1),
            _ => Err(anyhow!("Invalid string encoding")),
        }
    } else {
        bail!("not a TIT frame");
    }
}

fn read_0_terminated_string_iso<T: Read>(buf: &mut BufReader<T>, max_size: u32) -> Result<String> {
    let mut bytes = Vec::new();
    let amount_read = buf.read_until(0, &mut bytes)?;
    bytes.pop(); // pop 0 terminator
    let mut too_much_read = (amount_read as i32) - max_size as i32;
    while too_much_read > 0 {
        bytes.pop();
        too_much_read += 1;
    }
    Ok(String::from_utf8(bytes)?)
}

fn read_0_terminated_string_u8<T: Read>(buf: &mut BufReader<T>, max_size: u32) -> Result<String> {
    let mut bytes = Vec::new();
    let mut character: [u8; 2] = [0, 0];
    let mut counter = 0;
    loop {
        let amount_read = buf.read(&mut character);
        if character == [0, 0] || amount_read.unwrap_or(0) == 0 || counter >= max_size {
            break;
        }
        bytes.push(character[1]);
        bytes.push(character[0]);
        counter += 2;
    }
    Ok(String::from_utf8(bytes)?)
}

fn read_0_terminated_string_u16<T: Read>(buf: &mut BufReader<T>, max_size: u32) -> Result<String> {
    let mut bytes = Vec::new();
    let mut character: [u8; 2] = [0, 0];
    let mut counter = 0;
    loop {
        let amount_read = buf.read(&mut character);
        if character == [0, 0] || amount_read.unwrap_or(0) == 0 || counter >= max_size {
            break;
        }
        bytes.push(u16::from_be_bytes([character[1], character[0]]));
        counter += 2;
    }
    Ok(String::from_utf16(&bytes)?)
}

fn read_0_terminated_string_u16_ucs<T: Read>(
    buf: &mut BufReader<T>,
    max_size: u32,
) -> Result<String> {
    let mut bytes = Vec::new();
    let mut character: [u8; 2] = [0, 0];
    let mut counter = 0;
    loop {
        let amount_read = buf.read(&mut character);
        if character == [0, 0] || amount_read.unwrap_or(0) == 0 || counter >= max_size {
            break;
        }
        bytes.push(u16::from_be_bytes([character[1], character[0]]));
        counter += 2;
    }
    // Due to the nature of UCS-2, the output buffer could end up with
    // three bytes for every character in the input buffer.
    let mut title_buf = vec![0; bytes.len() * 3];
    let res = ucs2::decode(&bytes, &mut title_buf);
    match res {
        Ok(_) => Ok(String::from_utf8(title_buf)?
            .trim_end_matches('\0')
            .to_string()),
        // fallback to basic UTF-16
        Err(e) => {
            error!("UCS ERROR {e:#?}");
            Ok(String::from_utf16(&bytes)?)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[rustfmt::skip]
    fn test_id3_newrustacean() -> Result<()> {
        // Examples taken from https://newrustacean.com/feed.xml Episode

        // USLT - Unsynced lyrics Tag
        let id3_bin: &[u8] = &[ 85, 83, 76, 84, 0, 0, 0, 132, 0, 0, 1, 101, 110, 103, 255, 254, 0, 0, 255, 254, 87, 0, 65, 0, 83, 0, 73, 0, 44, 0, 32, 0, 96, 0, 79, 0, 112, 0, 116, 0, 105, 0, 111, 0, 110, 0, 58, 0, 58, 0, 99, 0, 111, 0, 112, 0, 105, 0, 101, 0, 100, 0, 96, 0, 44, 0, 32, 0, 97, 0, 110, 0, 100, 0, 32, 0, 116, 0, 104, 0, 101, 0, 32, 0, 102, 0, 117, 0, 116, 0, 117, 0, 114, 0, 101, 0, 32, 0, 111, 0, 102, 0, 32, 0, 97, 0, 115, 0, 121, 0, 110, 0, 99, 0, 47, 0, 97, 0, 119, 0, 97, 0, 105, 0, 116, 0, 32, 0, 115, 0, 121, 0, 110, 0, 116, 0, 97, 0, 120, 0, 33, 0, ];
        let chapter = read_id3_chapter(&mut BufReader::new(id3_bin));
        assert!(chapter.is_err());

        // CTOC - Chapter Table of Contents
        let id3_bin: &[u8] = &[ 67, 84, 79, 67, 0, 0, 0, 41, 0, 0, 116, 111, 99, 0, 3, 7, 99, 104, 112, 48, 0, 99, 104, 112, 49, 0, 99, 104, 112, 50, 0, 99, 104, 112, 51, 0, 99, 104, 112, 52, 0, 99, 104, 112, 53, 0, 99, 104, 112, 54, 0, ];
        let chapter = read_id3_chapter(&mut BufReader::new(id3_bin));
        assert!(chapter.is_err());

        let id3_bin: &[u8] = &[ 67, 72, 65, 80, 0, 0, 0, 44, 0, 0, 99, 104, 112, 48, 0, 0, 0, 0, 0, 0, 0, 89, 206, 255, 255, 255, 255, 255, 255, 255, 255, 84, 73, 84, 50, 0, 0, 0, 13, 0, 0, 1, 255, 254, 73, 0, 110, 0, 116, 0, 114, 0, 111, 0, ];
        let chapter = read_id3_chapter(&mut BufReader::new(id3_bin))?;
        assert_eq!(
            Chapter {
                id: "chp0".to_string(),
                title: "\u{feff}Intro".to_string(),
                description: "".to_string(),
                start: chrono::Duration::new(0, 0).unwrap(),
                end: chrono::Duration::new(22, 990000000).unwrap(),
            },
            chapter
        );

        let id3_bin: &[u8] = &[ 67, 72, 65, 80, 0, 0, 0, 113, 0, 0, 99, 104, 112, 49, 0, 0, 0, 89, 206, 0, 1, 1, 208, 255, 255, 255, 255, 255, 255, 255, 255, 84, 73, 84, 50, 0, 0, 0, 33, 0, 0, 1, 255, 254, 83, 0, 112, 0, 111, 0, 110, 0, 115, 0, 111, 0, 114, 0, 58, 0, 32, 0, 80, 0, 97, 0, 114, 0, 105, 0, 116, 0, 121, 0, 87, 88, 88, 88, 0, 0, 0, 39, 0, 0, 0, 99, 104, 97, 112, 116, 101, 114, 32, 117, 114, 108, 0, 104, 116, 116, 112, 115, 58, 47, 47, 119, 119, 119, 46, 112, 97, 114, 105, 116, 121, 46, 105, 111, 47, 106, 111, 98, 115, ];
        let chapter = read_id3_chapter(&mut BufReader::new(id3_bin))?;
        assert_eq!(
            Chapter {
                id: "chp1".to_string(),
                title: "\u{feff}Sponsor: Parity".to_string(),
                description: "".to_string(),
                start: chrono::Duration::new(22, 990000000).unwrap(),
                end: chrono::Duration::new(66, 0).unwrap(),
            },
            chapter
        );

        let id3_bin: &[u8] = &[ 67, 72, 65, 80, 0, 0, 0, 68, 0, 0, 99, 104, 112, 50, 0, 0, 1, 1, 208, 0, 3, 59, 240, 255, 255, 255, 255, 255, 255, 255, 255, 84, 73, 84, 50, 0, 0, 0, 37, 0, 0, 1, 255, 254, 52, 0, 32, 0, 121, 0, 101, 0, 97, 0, 114, 0, 115, 0, 32, 0, 115, 0, 105, 0, 110, 0, 99, 0, 101, 0, 32, 0, 49, 0, 46, 0, 48, 0, ];
        let chapter = read_id3_chapter(&mut BufReader::new(id3_bin))?;
        assert_eq!(
            Chapter {
                id: "chp2".to_string(),
                title: "\u{feff}4 years since 1.0".to_string(),
                description: "".to_string(),
                start: chrono::Duration::new(66, 0).unwrap(),
                end: chrono::Duration::new(211, 952000000).unwrap(),
            },
            chapter
        );

        let id3_bin: &[u8] = &[ 67, 72, 65, 80, 0, 0, 0, 52, 0, 0, 99, 104, 112, 51, 0, 0, 3, 59, 240, 0, 7, 122, 16, 255, 255, 255, 255, 255, 255, 255, 255, 84, 73, 84, 50, 0, 0, 0, 21, 0, 0, 1, 255, 254, 82, 0, 117, 0, 115, 0, 116, 0, 32, 0, 49, 0, 46, 0, 51, 0, 53, 0, ];
        let chapter = read_id3_chapter(&mut BufReader::new(id3_bin))?;
        assert_eq!(
            Chapter {
                id: "chp3".to_string(),
                title: "\u{feff}Rust 1.35".to_string(),
                description: "".to_string(),
                start: chrono::Duration::new(211, 952000000).unwrap(),
                end: chrono::Duration::new(490, 0).unwrap(),
            },
            chapter
        );

        let id3_bin: &[u8] = &[ 67, 72, 65, 80, 0, 0, 0, 90, 0, 0, 99, 104, 112, 52, 0, 0, 7, 122, 16, 0, 15, 35, 0, 255, 255, 255, 255, 255, 255, 255, 255, 84, 73, 84, 50, 0, 0, 0, 59, 0, 0, 1, 255, 254, 70, 0, 105, 0, 110, 0, 97, 0, 108, 0, 32, 0, 96, 0, 97, 0, 115, 0, 121, 0, 110, 0, 99, 0, 96, 0, 47, 0, 96, 0, 97, 0, 119, 0, 97, 0, 105, 0, 116, 0, 96, 0, 32, 0, 115, 0, 121, 0, 110, 0, 116, 0, 97, 0, 120, 0, ];
        let chapter = read_id3_chapter(&mut BufReader::new(id3_bin))?;
        assert_eq!(
            Chapter {
                id: "chp4".to_string(),
                title: "\u{feff}Final `async`/`await` syntax".to_string(),
                description: "".to_string(),
                start: chrono::Duration::new(490, 0).unwrap(),
                end: chrono::Duration::new(992, 0).unwrap(),
            },
            chapter
        );

        let id3_bin: &[u8] = &[ 67, 72, 65, 80, 0, 0, 0, 121, 0, 0, 99, 104, 112, 53, 0, 0, 15, 35, 0, 0, 15, 235, 140, 255, 255, 255, 255, 255, 255, 255, 255, 84, 73, 84, 50, 0, 0, 0, 35, 0, 0, 1, 255, 254, 80, 0, 97, 0, 116, 0, 114, 0, 101, 0, 111, 0, 110, 0, 32, 0, 83, 0, 112, 0, 111, 0, 110, 0, 115, 0, 111, 0, 114, 0, 115, 0, 87, 88, 88, 88, 0, 0, 0, 45, 0, 0, 0, 99, 104, 97, 112, 116, 101, 114, 32, 117, 114, 108, 0, 104, 116, 116, 112, 115, 58, 47, 47, 112, 97, 116, 114, 101, 111, 110, 46, 99, 111, 109, 47, 110, 101, 119, 114, 117, 115, 116, 97, 99, 101, 97, 110, ];
        let chapter = read_id3_chapter(&mut BufReader::new(id3_bin))?;
        assert_eq!(
            Chapter {
                id: "chp5".to_string(),
                title: "\u{feff}Patreon Sponsors".to_string(),
                description: "".to_string(),
                start: chrono::Duration::new(992, 0).unwrap(),
                end: chrono::Duration::new(1043, 340000000).unwrap(),
            },
            chapter
        );

        let id3_bin: &[u8] = &[ 67, 72, 65, 80, 0, 0, 0, 52, 0, 0, 99, 104, 112, 54, 0, 0, 15, 235, 140, 0, 16, 193, 16, 255, 255, 255, 255, 255, 255, 255, 255, 84, 73, 84, 50, 0, 0, 0, 21, 0, 0, 1, 255, 254, 83, 0, 104, 0, 111, 0, 119, 0, 32, 0, 105, 0, 110, 0, 102, 0, 111, 0, ];
        let chapter = read_id3_chapter(&mut BufReader::new(id3_bin))?;
        assert_eq!(
            Chapter {
                id: "chp6".to_string(),
                title: "\u{feff}Show info".to_string(),
                description: "".to_string(),
                start: chrono::Duration::new(1043, 340000000).unwrap(),
                end: chrono::Duration::new(1098, 0).unwrap(),
            },
            chapter
        );

        Ok(())
    }
}
