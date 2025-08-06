use socket2::Domain;
use socket2::Protocol;
use socket2::Socket;
use socket2::Type;
use std::io;
use std::net::TcpStream as StdTcpStream;
use std::net::ToSocketAddrs;
use std::os::fd::{FromRawFd, RawFd};

pub struct TcpStream(Socket);

// impl TcpStream {
//     pub async fn connect<A: ToSocketAddrs>(addr: A) -> io::Result<TcpStream> {
//         let a = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;
//     }
// }
