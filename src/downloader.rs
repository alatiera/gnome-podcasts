use reqwest;
use hyper::header::*;

use std::fs::File;
use std::io::{BufWriter, Read, Write};
use errors::*;

// Adapted from https://github.com/mattgathu/rget .
pub fn download_to(target: &str, url: &str) -> Result<()> {
    let mut resp = reqwest::get(url)?;
    info!("GET request to: {}", url);

    if resp.status().is_success() {
        let headers = resp.headers().clone();

        let ct_len = headers.get::<ContentLength>().map(|ct_len| **ct_len);
        let ct_type = headers.get::<ContentType>().unwrap();
        ct_len.map(|x| info!("File Lenght: {}", x));
        info!("Content Type: {:?}", ct_type);

        // FIXME
        let out_file = target.to_owned() + "/bar.mp3";
        info!("Save destination: {}", out_file);

        let chunk_size = match ct_len {
            Some(x) => x as usize / 99,
            None => 1024usize, // default chunk size
        };

        // let foo_file =

        let mut writer = BufWriter::new(File::create(out_file)?);

        loop {
            let mut buffer = vec![0; chunk_size];
            let bcount = resp.read(&mut buffer[..]).unwrap();
            buffer.truncate(bcount);
            if !buffer.is_empty() {
                writer.write(buffer.as_slice()).unwrap();
            } else {
                break;
            }
        }
    }
    Ok(())
}
