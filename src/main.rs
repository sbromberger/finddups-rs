use std::collections::HashMap;
use std::{env, fs, io, error, path::PathBuf};
use jwalk::WalkDir;
use blake2::{Blake2s256, Digest};
use base64ct::{Base64, Encoding};
use std::time::Instant;
use rayon::prelude::*;

// we'll make this a command line argument at some point.
const MIN_FILE_SIZE: u64 = 1024;

// given a path buf, get its size. If the stat fails,
// log an error and return None.
fn get_size(e: &PathBuf) -> Option<u64> {
    match fs::metadata(e) {
        Ok(md) => Some(md.len()),
        Err(err) => {
            eprintln!("Cannot access {}: {:?}", e.to_str()?, err);
            None
        },
    }
}

fn main() -> Result<(),Box<dyn error::Error>> {
    let t0 = Instant::now();
    // this should be a command line argument.
    let curr_dir = env::current_dir()?;
    let mut size_map: HashMap<u64, Vec<String>> = HashMap::new();
    for (entry, size) in WalkDir::new(curr_dir)
        .into_iter()
            .map(|e| e.unwrap().path().to_owned())
            .filter(|e| e.is_file())  // only deal with regular files
            .filter_map(|e| {
                match get_size(&e) {
                    Some(sz) => Some((e.to_str()?.to_string(), sz)),
                    None => None
                }
            })
    .filter(|(_, size)| *size >= MIN_FILE_SIZE) {
        size_map.entry(size).or_default().push(entry);
    }

    eprintln!("Walked {}ms", Instant::elapsed(&t0).as_millis());
    // second pass: for the same-sized files, calc their hashes.
    let all_possible_dups: Vec<&String> = size_map.values().flatten().collect();
    let hash_pairs: Vec<(String, String)> = all_possible_dups.par_iter().map(|entry| {
            let mut hasher = Blake2s256::new();
            let mut f = fs::File::open(&entry).unwrap();
            io::copy(&mut f, &mut hasher).unwrap();
            let res = hasher.finalize();
            (Base64::encode_string(&res), (*entry).to_owned())
    }).collect();

    let mut hash_map: HashMap<&String, Vec<&String>> = HashMap::new();
    for (res, entry) in hash_pairs.iter() {
            hash_map.entry(res).or_default().push(entry);
    }
    let mut dup_files: Vec<Vec<&String>> = Vec::new();
    for v in hash_map.values() {
        if v.len() > 1 {    // there is more than one entry in the hashmap - therefore, duplicate
            dup_files.push(v.to_owned());
        }
    }
    for df in dup_files {
        println!("{:?}", df)
    }
    eprintln!("Took {}ms", Instant::elapsed(&t0).as_millis());
    Ok(())
}
