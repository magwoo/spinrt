use std::time::{Duration, Instant};

use spinrt::Runtime;

fn main() {
    let rt = Runtime::new(8);

    let a = rt.spawn(my_async_fn());

    rt.block_on(async move {
        let now = Instant::now();
        spinrt::time::sleep(Duration::from_secs(1)).await;
        println!("Hello World!, elapsed: {:?}", now.elapsed());
        println!("my async fn result: {}", a.await);
    });
}

async fn my_async_fn() -> i32 {
    spinrt::time::sleep(Duration::from_secs(3)).await;

    7
}
