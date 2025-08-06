use spinrt::net::UdpSocket;
use std::{
    mem::MaybeUninit,
    time::{Duration, Instant},
};

fn main() {
    spinrt::create(8);

    let a = spinrt::spawn(my_async_fn());

    for i in 0..1000 {
        spinrt::spawn(async move {
            for j in 0..3 {
                println!("spawn {i} ticked {j}");
                spinrt::time::sleep(Duration::from_secs(1)).await;
            }
        });
    }

    spinrt::block_on(async move {
        let handle = spinrt::spawn(listen_udp_socket());

        let now = Instant::now();

        spinrt::time::sleep(Duration::from_secs(1)).await;
        println!("Hello World!, elapsed: {:?}", now.elapsed());

        println!("my async fn result: {}", a.await);
        println!("Hello World!, elapsed: {:?}", now.elapsed());

        handle.await;
    });
}

async fn my_async_fn() -> i32 {
    spinrt::time::sleep(Duration::from_secs(3)).await;

    7
}

async fn listen_udp_socket() {
    let socket = UdpSocket::bind("0.0.0.0:7500").await.unwrap();

    let mut buf = [MaybeUninit::uninit(); 4096];

    loop {
        let readed = socket.recv(&mut buf).await.unwrap();

        println!("udp socket readed {} bytes: {:?}", readed, &buf[..readed])
    }
}
