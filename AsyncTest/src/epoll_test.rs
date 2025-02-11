use epoll::{ControlOptions::*, Event, Events};
use std::collections::HashMap;
use std::io::ErrorKind;
use std::os::fd::AsRawFd;
use std::{
    fmt::Error,
    io::{self, Read, Write},
    net::TcpListener,
};
#[warn(dead_code)]

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
    let epoll = epoll::create(false).unwrap();
    let mut connections = Vec::new();

    let mut connections = HashMap::new();
    let cnx = TcpListener::bind("127.0.0.1:3000").unwrap();
    cnx.set_nonblocking(true).unwrap();
    let event = Event::new(Events::EPOLLIN, cnx.as_raw_fd() as u64);
    epoll::ctl(epoll, EPOLL_CTL_ADD, cnx.as_raw_fd(), event);
    loop {
        let mut events = [Event::new(Events::empty(), 0); 1024];
        let to = -1;

        //waiting for a syscall signal from clib epoll
        let num_events = epoll::wait(epoll, to, &mut events).unwrap();
        'next: for event in &events[..num_events] {
            let fd = event.data as i32;

            //listener ready
            if fd == cnx.as_raw_fd() {
                match cnx.accept() {
                    Ok((stream, _)) => {
                        stream.set_nonblocking(true).unwrap();

                        //register a new epoll
                        let fd = stream.as_raw_fd();
                        let event = Event::new(Events::EPOLLIN | Events::EPOLLOUT, fd as u64);
                        epoll::ctl(epoll, EPOLL_CTL_ADD, fd, event).unwrap();

                        let state = ConnnectionState::Read {
                            request: [0u8; 1024],
                            read: 0,
                        };
                        connections.insert(fd, (stream, state));
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                    Err(e) => panic!("{e}"),
                }
            }
            let (stream, state) = connections.get_mut(&fd).unwrap();
        }
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
        let mut completed = Vec::new();
        //iterate over the connections after every .accept and check their state
        // extract until there is no blocking if a blocking exists do the next iteration over the
        // connections
        // go back and accept new connections(if not blocked)
        //
        'next: for (i, (stream, state)) in connections.iter_mut().enumerate() {
            if let ConnnectionState::Read { request, read } = state {
                loop {
                    match stream.read(&mut request[*read..]) {
                        Ok(0) => {
                            println!("no size left to read");
                            completed.push(i);
                        }
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
                let res = concat!("HTTP/1.1 200 OK\r\n\r\n", "hello");
                *state = ConnnectionState::Write {
                    response: res.as_bytes(),
                    written: 0,
                };
            }
            if let ConnnectionState::Write { response, written } = state {
                loop {
                    match stream.write(&response[*written..]) {
                        Ok(0) => {
                            println!("ggeg");
                            completed.push(i);
                            continue 'next;
                        }
                        Ok(n) => *written += n,
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue 'next,
                        Err(e) => panic!("fff{e}"),
                    }
                    if response.len() == *written {
                        break;
                    }
                }
                *state = ConnnectionState::Flush;
            }

            if let ConnnectionState::Flush = state {
                match stream.flush() {
                    Ok(_) => completed.push(i),
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue 'next,
                    Err(e) => panic!("error {e}"),
                }
            }
        }
        for i in completed.into_iter().rev() {
            connections.remove(i);
        }
    }
}
