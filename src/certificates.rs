extern crate openssl;
use openssl::ssl::{SslStream};

use std::net::TcpStream;
use std::path::Path;
use std::fs::File;

use rusqlite;

pub enum ServerCertError {
    CertNotPresent,
    CertChanged
}

pub fn create_db() {
    let conn = open_db().unwrap();
    conn.execute(
        "CREATE TABLE certificate (
                  host            TEXT PRIMARY KEY,
                  digest          BLOB
                  )",
        rusqlite::params![],
    ).unwrap();
}

fn open_db() -> Option<rusqlite::Connection> {
    match rusqlite::Connection::open("/tmp/ruostepurkki.db") {
        Ok(c) => Some(c),
        Err(_) => None
    }
}

fn insert_into_db(host: &str, digest: &[u8]) -> Result<(), ()> {
    let conn: rusqlite::Connection = match open_db() {
        Some(c) => c,
        None => { return Err(()); }
    };

    match conn.execute("INSERT INTO certificate (host, digest) VALUES (?, ?)", rusqlite::params![host, digest]) {
        Ok(_) => Ok(()),
        Err(_) => Err(())
    }
}

fn cert_in_db(host: &str) -> Option<Vec<u8>> {
    let conn: rusqlite::Connection = match open_db() {
        Some(c) => c,
        None => { return None; }
    };

    let stmt = "SELECT digest FROM certificate WHERE host=(?)";
    match conn.query_row(stmt, &[&host], |r| r.get(0)) {
        Ok(r) => r,
        Err(_) => None
    }
}

pub fn check_cert(stream: &SslStream<TcpStream>, host: &str) -> Result<(), (ServerCertError, Option<String>)> {
    let cert = match stream.ssl().peer_certificate() {
        Some(c) => c,
        None => { 
            return Err( (ServerCertError::CertNotPresent, None) );
        }
    };
    let digest: Vec<u8> = cert.digest(openssl::hash::MessageDigest::sha256()).unwrap().as_ref().iter().cloned().collect();
    
    let known_digest: Vec<u8> = match cert_in_db(host) {
        Some(v) => {
            println!("Known cert is {} bytes long", v.len());
            v.iter().cloned().collect()
        }
        None => {
            println!("Certificate not found in db");
            let d = cert.digest(openssl::hash::MessageDigest::sha256()).unwrap();
            insert_into_db(host, &d).unwrap();
            // Would be better to just use digest directly
            //cert_in_db(host).unwrap()
            d.as_ref().iter().cloned().collect()
        }
    };

    if digest == known_digest {
        return Ok(());
    }
    else {
        return Err((ServerCertError::CertChanged, None));
    }
}