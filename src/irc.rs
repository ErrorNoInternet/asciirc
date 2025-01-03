use std::str::FromStr;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::{Duration, timeout},
};

#[derive(Debug)]
pub struct Client {
    tcp_stream: TcpStream,
    channel: Option<String>,
    logged_in: bool,
    pub privmsgs: Vec<PrivMsg>,
}

#[derive(Debug)]
pub struct PrivMsg {
    pub source: String,
    pub content: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    Notice,
    LoggedIn,
    JoinChannel,
    PrivMsg,
}

impl FromStr for Event {
    type Err = crate::irc::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "NOTICE" => Ok(Self::Notice),
            "JOIN" => Ok(Self::JoinChannel),
            "PRIVMSG" => Ok(Self::PrivMsg),
            "001" => Ok(Self::LoggedIn),
            _ => Err(Self::Err::InvalidEvent),
        }
    }
}

impl Client {
    pub async fn new(server: &str) -> Result<Self, Error> {
        let tcp_stream = tokio::net::TcpStream::connect(server)
            .await
            .map_err(Error::Send)?;
        let mut client = Self {
            tcp_stream,
            logged_in: false,
            channel: None,
            privmsgs: Vec::new(),
        };
        client.sync(Some(Event::Notice)).await?;
        Ok(client)
    }

    pub async fn sync(&mut self, target_event: Option<Event>) -> Result<(), Error> {
        self.sync_with_timeout(target_event, Duration::from_secs(120))
            .await
    }

    pub async fn sync_with_timeout(
        &mut self,
        target_event: Option<Event>,
        read_timeout: Duration,
    ) -> Result<(), Error> {
        let mut buf = [0u8; 1024];
        while let Ok(Ok(len)) = timeout(read_timeout, self.tcp_stream.read(&mut buf)).await {
            for line in std::str::from_utf8(&buf[..len])
                .map_err(Error::InvalidString)?
                .lines()
            {
                let Some(server_segments) = line.split_once(' ') else {
                    continue;
                };

                if server_segments.0 == "PING" {
                    self.tcp_stream
                        .write_all(format!("PONG {}\r\n", server_segments.1).as_bytes())
                        .await
                        .map_err(Error::Send)?;
                } else if let Some(event_segments) = server_segments.1.split_once(' ') {
                    let Ok(event) = Event::from_str(event_segments.0) else {
                        continue;
                    };

                    match event {
                        Event::Notice => println!("notice: {}", event_segments.1),
                        Event::LoggedIn => self.logged_in = true,
                        Event::JoinChannel => {
                            self.channel = Some(
                                event_segments
                                    .1
                                    .trim_start_matches(':')
                                    .trim_start_matches('#')
                                    .to_owned(),
                            );
                        }
                        Event::PrivMsg => {
                            let Some(privmsg_segments) = event_segments.1.split_once(' ') else {
                                continue;
                            };
                            let Some(source_nick) = server_segments.0.split_once('!') else {
                                continue;
                            };

                            self.privmsgs.push(PrivMsg {
                                source: source_nick.0[1..].to_owned(),
                                content: privmsg_segments.1[1..].to_owned(),
                            });
                        }
                    }
                    if Some(event) == target_event {
                        return Ok(());
                    }
                }
            }
        }

        if target_event.is_some() {
            Err(Error::Timeout)
        } else {
            Ok(())
        }
    }

    pub async fn login(&mut self, nickname: &str) -> Result<(), Error> {
        self.tcp_stream
            .write_all(
                format!("USER {nickname} {nickname} {nickname} :asciirc\r\nNICK {nickname}\r\n")
                    .as_bytes(),
            )
            .await
            .map_err(Error::Send)?;
        self.sync(Some(Event::LoggedIn)).await
    }

    pub async fn join_channel(&mut self, channel: &str) -> Result<(), Error> {
        if !self.logged_in {
            return Err(Error::NotLoggedIn);
        }

        self.tcp_stream
            .write_all(format!("JOIN #{channel}\r\n").as_bytes())
            .await
            .map_err(Error::Send)?;
        self.sync(Some(Event::JoinChannel)).await
    }

    pub async fn send_message(&mut self, message: &str) -> Result<(), Error> {
        if !self.logged_in {
            return Err(Error::NotLoggedIn);
        }

        if let Some(channel) = &self.channel {
            self.tcp_stream
                .write_all(format!("PRIVMSG #{channel} :{message}\r\n").as_bytes())
                .await
                .map_err(Error::Send)?;
            self.tcp_stream.flush().await.map_err(Error::Flush)?;
            Ok(())
        } else {
            Err(Error::NoChannelJoined)
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Flush(std::io::Error),
    InvalidEvent,
    InvalidString(std::str::Utf8Error),
    NoChannelJoined,
    NotLoggedIn,
    Send(std::io::Error),
    Timeout,
}
