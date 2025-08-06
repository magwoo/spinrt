use spinrt::net::UdpSocket;
use std::mem::{MaybeUninit, transmute};
use std::time::Duration;

fn main() {
    spinrt::create(8);

    for port in 7000..8000 {
        spinrt::spawn(async move {
            let socket = UdpSocket::bind(format!("0.0.0.0:{port}").as_str())
                .await
                .unwrap();

            let mut buf = [MaybeUninit::uninit(); 4096];

            loop {
                let readed = socket.recv(&mut buf).await.unwrap();

                println!("port {port} handled: {}", unsafe {
                    String::from_utf8_lossy(transmute::<&[MaybeUninit<u8>], &[u8]>(&buf[..readed]))
                })
            }
        });
    }

    spinrt::block_on(spinrt::time::sleep(Duration::from_secs(u32::MAX as u64)));
}
