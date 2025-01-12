#![feature(let_chains)]

mod arguments;
mod irc;

use std::time::Duration;

use clap::Parser;

#[tokio::main]
async fn main() {
    let args = arguments::Arguments::parse();

    let mut clients = Vec::with_capacity(args.clients);
    for i in 0..args.clients {
        let nickname = &(args.nickname.clone() + &i.to_string());
        println!("joining with {nickname}...");
        let mut client = irc::Client::new(&args.server).await.unwrap();
        client.login(nickname).await.unwrap();
        client.join_channel(&args.channel).await.unwrap();
        clients.push(client);
    }
    let client_count = clients.len();
    println!("ready with {client_count} clients! listening for messages...");

    let mut last_file = None;
    for i in 0.. {
        let privmsg = if (clients[i % client_count]
            .sync_with_timeout(Some(irc::Event::PrivMsg), Duration::from_secs(10))
            .await)
            .is_ok()
        {
            clients[i % client_count].privmsgs.pop().unwrap()
        } else {
            continue;
        };
        if !args.owners.contains(&privmsg.source) {
            continue;
        }
        if let Some(ref last) = last_file
            && privmsg.content == *last
        {
            continue;
        }

        let Ok(art) = std::fs::read_to_string(&privmsg.content) else {
            continue;
        };
        println!("sending {}...", &privmsg.content);
        last_file = Some(privmsg.content);

        let mut last_line = None;
        for line_chunk in art.lines().collect::<Vec<_>>().chunks(args.clients) {
            for (line, client) in line_chunk.iter().zip(&mut clients) {
                if client_count > 1
                    && let Some(&last_line) = last_line
                {
                    'outer: loop {
                        while let Some(privmsg) = client.privmsgs.pop() {
                            if privmsg.content == last_line {
                                break 'outer;
                            }
                        }

                        if let Err(irc::Error::Timeout) = client
                            .sync_with_timeout(
                                Some(irc::Event::PrivMsg),
                                Duration::from_millis(args.line_timeout),
                            )
                            .await
                        {
                            eprintln!("timed out waiting for previous line to be received");
                            break 'outer;
                        }
                    }
                }

                client
                    .send_message(if line.is_empty() { " " } else { line })
                    .await
                    .unwrap();
                last_line = if line.is_empty() { None } else { Some(line) };
            }
        }
    }
}
