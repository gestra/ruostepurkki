extern crate openssl;
use openssl::ssl::{SslStream};

use std::net::TcpStream;

use rusqlite;

pub enum ServerCertError {
    CertNotPresent,
    CertChanged
}

fn open_db() -> rusqlite::Result<rusqlite::Connection> {
    let c = rusqlite::Connection::open("/tmp/ruostepurkki.db")?;
    c.execute("CREATE TABLE IF NOT EXISTS certificate (host TEXT PRIMARY KEY, digest BLOB);", rusqlite::NO_PARAMS)?;
    Ok(c)
}

fn insert_into_db(host: &str, digest: &[u8]) -> rusqlite::Result<()> {
    let conn = open_db()?;
    conn.execute("INSERT INTO certificate (host, digest) VALUES (?, ?)", rusqlite::params![host, digest])?;
    Ok(())
}

fn cert_in_db(host: &str) -> rusqlite::Result<Option<Vec<u8>>> {
    let conn = open_db()?;

    let stmt = "SELECT digest FROM certificate WHERE host=(?)";
    let result = conn.query_row(stmt, &[&host], |r| r.get(0))?;
    Ok(Some(result))
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
        Ok(Some(v)) => v,
        Ok(None) => {
            return Err((ServerCertError::CertChanged, Some("Something weird must have happened".to_string())));
        },
        Err(err) => {
            if err == rusqlite::Error::QueryReturnedNoRows {
                insert_into_db(host, &digest).unwrap();
                return Ok(());
            }
            else {
                println!("Error: {}", err);
                return Err((ServerCertError::CertChanged, Some(err.to_string())));
            }
        }
    };

    if digest == known_digest {
        return Ok(());
    }
    else {
        return Err((ServerCertError::CertChanged, None));
    }
}