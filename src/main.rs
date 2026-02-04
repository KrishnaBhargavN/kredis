use std::io::Cursor;

use bytes::{Buf, BytesMut, buf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::frame::Frame;
mod frame;

mod db;
use db::Db;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:6379").await?;
    println!("Server running on 127.0.0.1:6379");
    let db = Db::new();
    loop {
        let (socket, _) = listener.accept().await?;
        let db_handle = db.clone();
        tokio::spawn(async move {
            process(socket, db_handle).await;
        });
    }
}

async fn process(mut socket: TcpStream, db: Db) {
    let mut buffer = BytesMut::with_capacity(4096);

    loop {
        if socket.read_buf(&mut buffer).await.unwrap() == 0 {
            return;
        }

        let mut cursor = Cursor::new(&buffer[..]);

        match Frame::parse(&mut cursor) {
            Ok(frame) => {
                println!("Received command : {:?}", frame);

                let len = cursor.position() as usize;
                buffer.advance(len);

                let response = match frame {
                    Frame::Array(ref array) => {
                        if let Some(Frame::Bulk(cmd_bytes)) = array.get(0) {
                            let cmd_str = std::str::from_utf8(cmd_bytes).unwrap().to_uppercase();

                            match cmd_str.as_str() {
                                "SET" => handle_set(&db, array),
                                "GET" => handle_get(&db, array),
                                "LPUSH" => handle_lpush(&db, array),
                                "RPOP" => handle_rpop(&db, array),
                                _ => Frame::Error("Unknown command".to_string()),
                            }
                        } else {
                            Frame::Error("Invalid command format".to_string())
                        }
                    }
                    _ => Frame::Error("Command must be an array".to_string()),
                };

                write_frame(&mut socket, response).await;
            }
            Err(frame::Error::Incomplete) => {
                continue;
            }
            Err(e) => {
                eprintln!("Error parsing : {:?}", e);
                return;
            }
        }
    }
}

fn handle_set(db: &Db, args: &[Frame]) -> Frame {
    if args.len() != 3 {
        return Frame::Error("ERR wrong number of arguments for 'set' command".to_string());
    }

    let key = match &args[1] {
        Frame::Bulk(b) => String::from_utf8(b.to_vec()).unwrap(),
        _ => return Frame::Error("Key must be string".to_string()),
    };

    let value = match &args[2] {
        Frame::Bulk(b) => b.clone(),
        _ => return Frame::Error("Value must be string".to_string()),
    };

    db.set(key, value);
    Frame::Simple("Ok".to_string())
}

fn handle_get(db: &Db, args: &[Frame]) -> Frame {
    if args.len() != 2 {
        return Frame::Error("ERR wrong number of arguments for 'get' command".to_string());
    }
    let key = match &args[1] {
        Frame::Bulk(b) => String::from_utf8(b.to_vec()).unwrap(),
        _ => return Frame::Error("Key must be string".to_string()),
    };

    match db.get(&key) {
        Some(value) => Frame::Bulk(value),
        None => Frame::Null,
    }
}

fn handle_lpush(db: &Db, args: &[Frame]) -> Frame {
    if args.len() != 3 {
        return Frame::Error("ERR wrong number of arguments for 'lpush' command.".to_string());
    }

    let key = match &args[1] {
        Frame::Bulk(b) => String::from_utf8(b.to_vec()).unwrap(),
        _ => return Frame::Error("Key must be string".to_string()),
    };

    let value = match &args[2] {
        Frame::Bulk(b) => b.clone(),
        _ => return Frame::Error("Value must be string".to_string()),
    };
    match db.lpush(key, value) {
        Ok(len) => Frame::Integer(len as u64),
        Err(msg) => Frame::Error(msg.to_string()),
    }
}

fn handle_rpop(db: &Db, args: &[Frame]) -> Frame {
    if args.len() != 2 {
        return Frame::Error("ERR wrong number of arguments for the 'rpop' command.".to_string());
    }

    let key = match &args[1] {
        Frame::Bulk(b) => String::from_utf8(b.to_vec()).unwrap(),
        _ => return Frame::Error("Key must be string".to_string()),
    };

    match db.rpop(&key) {
        Some(value) => Frame::Bulk(value),
        None => Frame::Null,
    }
}

async fn write_frame(socket: &mut TcpStream, frame: Frame) {
    match frame {
        Frame::Simple(s) => socket
            .write_all(format!("+{}\r\n", s).as_bytes())
            .await
            .unwrap(),
        Frame::Integer(i) => socket
            .write_all(format!("+{}\r\n", i).as_bytes())
            .await
            .unwrap(),
        Frame::Error(s) => socket
            .write_all(format!("-{}\r\n", s).as_bytes())
            .await
            .unwrap(),
        Frame::Bulk(b) => {
            socket
                .write_all(format!("${}\r\n", b.len()).as_bytes())
                .await
                .unwrap();
            socket.write_all(&b).await.unwrap();
            socket.write_all(b"\r\n").await.unwrap();
        }
        Frame::Null => socket.write_all(b"$-1\r\n").await.unwrap(),
        _ => {}
    }
}
