use std::env;
use std::process;
use std::thread;
use std::net::{TcpStream, TcpListener};
use std::io;
use std::io::{Write};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use tprox::utils::{Counter, SerializableBe, fill_buf_from_stream};
use tprox::{ProxyHeader, ProxyConn};

// TODO use BufReader to optimize number of syscalls

fn handle_proxy_client(mut src_stream: TcpStream, dst_stream: TcpStream, conn_id: u16) -> io::Result<()> {
    let mut conn = ProxyConn {
        dst_conn: dst_stream,
        conn_id: conn_id,
        // TODO handle error
        port: src_stream.local_addr().unwrap().port(),
    };

    // Read from stream, write to ProxyConn
    io::copy(&mut src_stream, &mut conn)?;

    Ok(())
}

/// Read from client and proxy the contents read to the right client based on conn_id
fn proxy_client_to_srcs(mut client_conn: TcpStream, src_conn_map: &RwLock<HashMap<u16, TcpStream>>) -> io::Result<()> {
    // Note on why we don't expect a Start header here (like we do on the client side):
    // Currently, the sole purpose of the start header is to tell the client what port the remote
    // proxy server is forwarding to it. Since there is no such requirement on the server side, we
    // sending a Start header is a waste

    const HLEN: usize = ProxyHeader::HEADER_LEN;
    let mut header_buf: [u8; HLEN] = [0; HLEN];

    loop {
        fill_buf_from_stream(&mut client_conn, &mut header_buf)?;

        // TODO handle error
        let header = ProxyHeader::from_be_bytes(&header_buf).unwrap();
        {
            // TODO handle error
            let mut src_conn_map_rw = src_conn_map.write().unwrap();
            let conn_id = header.conn_id;

            let maybe_conn = src_conn_map_rw.get_mut(&conn_id);
            match maybe_conn {
                Some(conn) => {
                    let mut data_buf = vec![0 as u8; header.data_len as usize];
                    // TODO handle error
                    fill_buf_from_stream(&mut client_conn, data_buf.as_mut_slice()).unwrap();
                    // TODO do I need to use io::copy here to make sure all of the data is written?
                    match conn.write(data_buf.as_slice()) {
                        Ok(size) => { println!("done writing {} bytes", size); },
                        Err(e) => { eprintln!("uh oh: {}", e); },
                    };
                },
                None => {
                    // TODO send a Fin packet here
                },
            }
        }
    }
}

fn run_proxy_server(mut client_conn: TcpStream) -> io::Result<()> {
    let mut conn_id_counter = Counter::new();

    // A port of zero will make the kernel assign us a port
    let addr = "0.0.0.0:0";

    let listener = TcpListener::bind(&addr)?;

    // TODO remove this, handle error
    println!("New proxy server for client {} listening on {}", client_conn.peer_addr().unwrap(), listener.local_addr().unwrap());

    let start_header = ProxyHeader::make_start_header(listener.local_addr().unwrap().port());
    client_conn.write(&start_header.serialize_be())?;

    let conn_map = Arc::new(RwLock::new(HashMap::new()));
    
    let conn_map_clone = Arc::clone(&conn_map);
    let c_conn_clone = client_conn.try_clone()?;
    thread::spawn(move || {
        proxy_client_to_srcs(c_conn_clone, &conn_map_clone).unwrap();
    });

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                // TODO handle error
                println!("New connection on proxy server from {}", stream.peer_addr().unwrap());

                let conn_id = conn_id_counter.increment();

                // TODO handle error
                let stream_clone = stream.try_clone().unwrap();
                {
                    // TODO handle error
                    let mut conn_map_rw = conn_map.write().unwrap();
                    conn_map_rw.insert(conn_id, stream_clone);
                }

                // TODO handle error
                let client_conn_clone = client_conn.try_clone().unwrap();

                thread::spawn(move || {
                    handle_proxy_client(stream, client_conn_clone, conn_id).unwrap();
                });
            },
            Err(e) => {
                eprintln!("Error in proxy server: {}", e);
            }
        }
    }

    Ok(())
}

fn handle_control_client(stream: TcpStream) {
    run_proxy_server(stream).unwrap();
}

fn run_control_server(control_port: &str) -> io::Result<()>{
    let addr = format!("0.0.0.0:{}", control_port);

    let listener = TcpListener::bind(&addr)?;

    println!("Server listening on {}", addr);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let client_addr = stream.peer_addr().unwrap();
                println!("New connection on control port: {}", client_addr);

                thread::spawn(move || {
                    handle_control_client(stream);
                });
            },
            Err(e) => {
                println!("Error on control port: {}", e);
            },
        }
    }

    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} port", args[0]);
        process::exit(1);
    }

    let control_port = &args[1];
    run_control_server(control_port).unwrap();
}
