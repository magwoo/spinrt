use std::time::{Duration, Instant};

fn main() {
    spinrt::create(8);

    let a = spinrt::spawn(my_async_fn());

    spinrt::block_on(async move {
        let now = Instant::now();
        spinrt::time::sleep(Duration::from_secs(1)).await;
        println!("Hello World!, elapsed: {:?}", now.elapsed());
        println!("my async fn result: {}", a.await);
        println!("Hello World!, elapsed: {:?}", now.elapsed());
    });
}

async fn my_async_fn() -> i32 {
    spinrt::time::sleep(Duration::from_secs(3)).await;

    7
}
