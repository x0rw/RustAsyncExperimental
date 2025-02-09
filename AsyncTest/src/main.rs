use std::{
    fmt::Error,
    io::{self, Read},
    net::TcpListener,
};

enum ConnnectionState {
    Read {
        request: [u8; 1024],
        read: usize,
    },
    Write {
        response: &'static [u8],
        written: usize,
    },
    Flush,
}
fn main() {
    let mut connections = Vec::new();
    let cnx = TcpListener::bind("127.0.0.1").unwrap();
    cnx.set_nonblocking(true).unwrap();
    loop {
        match cnx.accept() {
            Ok((stream, sockaddr)) => {
                stream.set_nonblocking(true).unwrap();
                let state = ConnnectionState::Read {
                    request: [0u8; 1024],
                    read: 0,
                };
                connections.push((stream, state));
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(e) => panic!("panic {e}"),
        }
        'next: for (stream, state) in connections.iter_mut() {
            if let ConnnectionState::Read { request, read } = state {
                loop {
                    match stream.read(&mut request[*read..]) {
                        Ok(n) => *read += n,
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue 'next,
                        Err(e) => panic!("{e}"),
                    }
                    if request.get(*read - 4..*read) == Some(b"\r\n\r\n") {
                        break;
                    }
                }
                let request = String::from_utf8_lossy(&request[..*read]);
                println!("{request}");

                if let ConnnectionState::Write { response, written } = state {}

                if let ConnnectionState::Flush = state {}
            }
        }
    }
}
