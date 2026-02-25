use tokio::time::{sleep, Duration};

pub async fn run_nemesis_schedule() {
    sleep(Duration::from_secs(5)).await;
    println!("NEMESIS: partitioning leader");

    sleep(Duration::from_secs(10)).await;
    println!("NEMESIS: healing partition");

    sleep(Duration::from_secs(5)).await;
    println!("NEMESIS: killing node");

    sleep(Duration::from_secs(5)).await;
    println!("NEMESIS: restarting node");

    sleep(Duration::from_secs(5)).await;
    println!("NEMESIS: done");
}
