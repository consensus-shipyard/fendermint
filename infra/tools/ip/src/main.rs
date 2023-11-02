// Copyright 2022-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT
use reqwest::Client;
use futures::{stream, StreamExt};
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Duration;

const CONCURRENT_REQUESTS: usize = 2;

#[tokio::main]
async fn main() -> Result<(), &'static str> {
    let client = Client::builder()
        .user_agent("curl/7.79.1")
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap();

    let urls = vec![
        "https://ident.me",
        "https://checkip.amazonaws.com",
        "https://ifconfig.me",
        "https://api.ipify.org",
        "https://ifconfig.co",
        "https://ipinfo.io/ip",
        "https://icanhazip.com",
        "https://api.ipify.org",
    ];

    let ips: HashMap<String, i32> = HashMap::new();
    let ips_cell = RefCell::new(ips);

    let bodies = stream::iter(urls)
        .map(|url| {
            let client = &client;
            async move {
                let resp = client.get(url).send().await?;
                resp.text().await
            }
        })
        .buffer_unordered(CONCURRENT_REQUESTS);

    bodies
        .for_each(|b| async {
            let mut borrowed = ips_cell.borrow_mut();
            match b {
                Ok(b) => {
                    if let Some(count) = borrowed.get_mut(&b) {
                        *count += 1;
                    } else {
                        borrowed.insert(b.clone(), 1);
                    }
                }
                Err(e) => eprintln!("Failed to get a response: {}", e),
            }
        })
        .await;

    let mut external_ip: Option<String> = None;
    let threshold = 3;

    for (ip, &n) in ips_cell.borrow().iter() {
        if n >= threshold {
            external_ip = Some(ip.to_string());
            break;
        }
    }

    match external_ip {
        Some(ip) => {
            let ip = format_ip_addr(ip);
            print!("{}", ip);
            Ok(())
        }
        None => Err("failed to resolve external IP"),
    }
}

fn format_ip_addr(addr: String) -> String {
    if addr.contains(':') {
        return format!("[{}]", addr);
    }
    addr
}
