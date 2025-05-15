use std::time::{Duration, Instant};

use spinrt::Runtime;

fn main() {
    let rt = Runtime::new(8);

    rt.block_on(async {
        let now = Instant::now();
        spinrt::time::sleep(Duration::from_secs(1)).await;
        println!("Hello World!, elapsed: {:?}", now.elapsed())
    });
}
