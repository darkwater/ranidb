use std::{io, string::FromUtf8Error, time::Duration};

use crate::command_builder::CommandBuilder;

use tokio::{net::UdpSocket, time::Instant};

mod command_builder;
#[macro_use]
pub mod responses;

pub use responses::*;

type RadResult<T> = Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Encoding(FromUtf8Error),
    AniDb(responses::Error),
    NoSession,
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(e: FromUtf8Error) -> Self {
        Error::Encoding(e)
    }
}

impl From<responses::Error> for Error {
    fn from(e: responses::Error) -> Self {
        Error::AniDb(e)
    }
}

pub struct AniDb {
    api_endpoint: String,
    client: &'static str,
    clientver: i32,
    session_key: Option<String>,
    connection: Option<UdpSocket>,
    next_call: Instant,
}

impl AniDb {
    pub fn new(client: &'static str, clientver: i32) -> Self {
        Self {
            api_endpoint: "api.anidb.net:9000".to_owned(),
            client,
            clientver,
            session_key: None,
            connection: None,
            next_call: Instant::now(),
        }
    }

    pub fn resume_session(client: &'static str, clientver: i32, session_key: String) -> Self {
        Self {
            session_key: Some(session_key),
            ..Self::new(client, clientver)
        }
    }

    pub fn client(&self) -> &str {
        self.client
    }

    pub fn client_version(&self) -> i32 {
        self.clientver
    }

    pub fn session_key(&self) -> Option<&str> {
        self.session_key.as_deref()
    }

    pub fn session_key_or_err(&self) -> RadResult<&str> {
        self.session_key.as_deref().ok_or(Error::NoSession)
    }

    async fn connect(&mut self) -> Result<&UdpSocket, io::Error> {
        if self.connection.is_none() {
            let conn = UdpSocket::bind("0.0.0.0:12345").await?;
            conn.connect(&self.api_endpoint).await?;
            self.connection = Some(conn);
        }

        Ok(self.connection.as_ref().unwrap())
    }

    pub async fn request(&mut self, cmd: &str) -> RadResult<String> {
        tokio::time::sleep_until(self.next_call).await;

        self.next_call = Instant::now() + Duration::from_secs(2);

        for line in cmd.lines() {
            log::trace!("-> {}", line);
        }

        let conn = self.connect().await?;
        conn.send(cmd.as_bytes()).await?;

        let mut buf = [0; 1400]; // 1400 is AniDB's default and maximum MTU
        let read = conn.recv(&mut buf).await?;
        let bytes = buf[..read].to_owned();
        let s = String::from_utf8(bytes)?;

        for line in s.lines() {
            log::trace!("<- {}", line);
        }

        Ok(s)
    }
}

// Authing Commands
impl AniDb {
    /// WARNING: UNENCRYPTED PASSWORD
    pub async fn auth(&mut self, username: &str, password: &str) -> RadResult<()> {
        let cmd = CommandBuilder::new("AUTH")
            .arg("user", username)
            .arg("pass", password)
            .arg("client", self.client)
            .arg("clientver", self.clientver)
            .arg("protover", 3)
            .arg("enc", "UTF8")
            .build();

        match_res! {
            match self.request(&cmd).await?;
            LoginAccepted { session_key } => {
                self.session_key = Some(session_key);
            },
            LoginAcceptedNewVersion { session_key } => {
                self.session_key = Some(session_key);
            },
        }

        Ok(())
    }

    pub async fn logout(&mut self) -> RadResult<()> {
        let cmd = CommandBuilder::new("LOGOUT")
            .arg("s", self.session_key_or_err()?)
            .build();

        match_res! {
            match self.request(&cmd).await?;
            LoggedOut {} => {},
        }

        Ok(())
    }
}

parser!(Anime {
    "230 ANIME\n"
    {aid: u32} "|"
    {dateflags: i32} "|"
    {year: String} "|"
    {atype: String} "|"
    {related_aid_list: String} "|"
    {related_aid_type: String} "|"
    {romaji_name: String} "|"
    {kanji_name: String} "|"
    {english_name: String} "|"
    {short_name_list: String} "|"
    {episodes: i32} "|"
    {special_ep_count: i32} "|"
    {air_date: i32} "|"
    {end_date: i32} "|"
    {picname: String} "|"
    {nsfw: bool} "|"
    {characterid_list: String} "|"
    {specials_count: i32} "|"
    {credits_count: i32} "|"
    {other_count: i32} "|"
    {trailer_count: i32} "|"
    {parody_count: i32} "\n"
});

parser!(Episode {
    "240 EPISODE\n"
    {eid: u32} "|"
    {aid: u32} "|"
    {length: i32} "|"
    {rating: i32} "|"
    {votes: i32} "|"
    {epno: String} "|"
    {eng: String} "|"
    {romaji: String} "|"
    {kanji: String} "|"
    {aired: i32} "|"
    {etype: i32} "\n"
});

parser!(File {
    "220 FILE\n"
    {fid: u32} "|"
    {aid: u32} "|"
    {eid: u32} "|"
    {gid: u32} "|"
    {state: i16} "|"
    {size: i64} "|"
    {ed2k: String} "|"
    {colour_depth: String} "|"
    {quality: String} "|"
    {source: String} "|"
    {audio_codec_list: String} "|"
    {audio_bitrate_list: i32} "|"
    {video_codec: String} "|"
    {video_bitrate: i32} "|"
    {video_resolution: String} "|"
    {dub_language: String} "|"
    {sub_language: String} "|"
    {length_in_seconds: i32} "|"
    {description: String} "|"
    {aired_date: i32} "\n"
});

parser!(Group {
    "250 GROUP\n"
    {gid: u32} "|"
    {rating: i32} "|"
    {votes: i32} "|"
    {acount: i32} "|"
    {fcount: i32} "|"
    {name: String} "|"
    {short: String} "|"
    {irc_channel: String} "|"
    {irc_server: String} "|"
    {url: String} "|"
    {picname: String} "|"
    {foundeddate: i32} "|"
    {disbandeddate: i32} "|"
    {dateflags: i16} "|"
    {lastreleasedate: i32} "|"
    {lastactivitydate: i32} "|"
    {grouprelations: String} "\n"
});

// Data Commands
impl AniDb {
    pub async fn anime_by_id(&mut self, id: u32) -> RadResult<Anime> {
        let cmd = CommandBuilder::new("ANIME")
            .arg("s", self.session_key_or_err()?)
            .arg("aid", id)
            .arg("amask", "fce8ba010080f8")
            .build();

        match_res! {
            match self.request(&cmd).await?;
            return Anime,
        }
    }

    pub async fn episode_by_id(&mut self, id: u32) -> RadResult<Episode> {
        let cmd = CommandBuilder::new("EPISODE")
            .arg("s", self.session_key_or_err()?)
            .arg("eid", id)
            .build();

        match_res! {
            match self.request(&cmd).await?;
            return Episode,
        }
    }

    pub async fn file_by_ed2k(&mut self, size: u64, hash: &str) -> RadResult<File> {
        let cmd = CommandBuilder::new("FILE")
            .arg("s", self.session_key_or_err()?)
            .arg("size", size)
            .arg("ed2k", hash)
            .arg("fmask", "71c2fef800")
            .arg("amask", "00000000")
            .build();

        match_res! {
            match self.request(&cmd).await?;
            return File,
        }
    }

    pub async fn group_by_id(&mut self, id: u32) -> RadResult<Group> {
        let cmd = CommandBuilder::new("GROUP")
            .arg("s", self.session_key_or_err()?)
            .arg("gid", id)
            .build();

        match_res! {
            match self.request(&cmd).await?;
            return Group,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
