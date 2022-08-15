use std::env;
use std::process;
use std::thread;
use std::collections::HashMap;
use std::net::{Shutdown, TcpStream};
use std::io;

use tprox::utils::{fill_buf_from_stream};
use tprox::{ProxyHeader, ProxyHeaderType, ProxyConn};

fn proxy_backend_to_remote(mut backend_conn: TcpStream, remote_conn: TcpStream, conn_id: u16) {
    let mut conn = ProxyConn {
        dst_conn: remote_conn,
        conn_id: conn_id,
        port: 0,
    };

    // Read from backend, write to ProxyConn
    // TODO handle error
    io::copy(&mut backend_conn, &mut conn).unwrap();
}

fn start_proxy_conn(proxy_addr: &str, backend_addr: &str) -> io::Result<()> {
    let mut conn_map: HashMap<u16, TcpStream> = HashMap::new();

    let mut stream = TcpStream::connect(proxy_addr)?;

    const HLEN: usize = ProxyHeader::HEADER_LEN;
    let mut header_buf: [u8; HLEN] = [0; HLEN];

    // TODO handle error
    fill_buf_from_stream(&mut stream, &mut header_buf).unwrap();

    // TODO handle error
    let header = ProxyHeader::from_be_bytes(&header_buf).unwrap();

    match header.header_type {
        ProxyHeaderType::Start => {
            println!("Proxy running at {}:{}", &proxy_addr.split(":").collect::<Vec<&str>>()[0], header.port);
        },
        _ => {
            // TODO proper error
            panic!("Did not receive start header, aborting!");
        }
    };

    loop {
        // TODO handle error
        fill_buf_from_stream(&mut stream, &mut header_buf).unwrap();

        // TODO handle error
        let header = ProxyHeader::from_be_bytes(&header_buf).unwrap();

        let conn_exists = conn_map.contains_key(&header.conn_id);

        match header.header_type {
            ProxyHeaderType::Fin => {
                if conn_exists {
                    // TODO handle error
                    let conn = conn_map.get_mut(&header.conn_id).unwrap();
                    // TODO handle error (ignore it? who cares?)
                    conn.shutdown(Shutdown::Both).unwrap();
                }
                continue
            },
            ProxyHeaderType::Data => {
                if ! conn_exists {
                    let conn = TcpStream::connect(backend_addr)?;

                    let remote_conn_clone = stream.try_clone()?;
                    let backend_conn_clone = conn.try_clone()?;
                    let conn_id = header.conn_id;
                    thread::spawn(move || {
                        proxy_backend_to_remote(backend_conn_clone, remote_conn_clone, conn_id);
                    });

                    conn_map.insert(header.conn_id, conn);
                }

                let mut conn = conn_map.get_mut(&header.conn_id).unwrap();

                let mut data_buf = vec![0 as u8; header.data_len as usize];
                fill_buf_from_stream(&mut stream, &mut data_buf).unwrap();

                // TODO handle error
                io::copy(&mut data_buf.as_slice(), &mut conn).unwrap();
            },
            _ => {
                // TODO proper error
                panic!("Invalid header type received");
            }
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: {} server-hostname:server-port backend-port", args[0]);
        process::exit(1);
    }

    let proxy_addr = &args[1];
    let backend_addr = format!("127.0.0.1:{}", args[2]);

    start_proxy_conn(proxy_addr, &backend_addr).unwrap();
}
