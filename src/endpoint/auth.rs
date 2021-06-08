#![allow(dead_code)]

use chrono::{DateTime, Duration, Utc};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::PathBuf;

use super::{CLIENT, RATE_LIMIT};

fn auth_file_location() -> PathBuf {
    crate::data_dir().join("auth.json")
}

fn open_or_create_token() -> Token {
    let loc = auth_file_location();
    if loc.exists() {
        let f = File::open(loc).expect("Failed to open auth token file");
        serde_json::from_reader(f).expect("Failed to deserialize auth token")
    } else {
        todo!()
    }
}

static TOKEN: Lazy<Token> = Lazy::new(|| todo!());

#[derive(Debug, Deserialize, Serialize)]
struct Token {
    session_token: String,
    refresh_token: String,
    session_expiry: DateTime<Utc>,
    refresh_expiry: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    session: String,
    refresh: String,
}

impl Token {
    fn needs_refresh(&self) -> bool {
        self.session_expiry - Utc::now() < Duration::seconds(30)
    }

    fn needs_relogin(&self) -> bool {
        self.refresh_expiry - Utc::now() < Duration::zero()
    }

    async fn refresh(&mut self) -> reqwest::Result<()> {
        if !self.needs_refresh() {
            return Ok(());
        }
        if self.needs_relogin() {
            *self = Self::login().await?;
            return Ok(());
        }

        #[derive(Serialize)]
        struct Refresh<'a> {
            token: &'a str,
        }
        let refresh = Refresh {
            token: &self.refresh_token,
        };

        let permit = RATE_LIMIT.request().await;
        let now = Utc::now();
        let new_token = CLIENT
            .post("https://api.mangadex.org/auth/refresh")
            .json(&refresh)
            .send()
            .await?
            .error_for_status()?
            .json::<TokenResponse>()
            .await?;

        self.session_token = new_token.session;
        self.refresh_token = new_token.refresh;
        self.session_expiry = now + Duration::minutes(15);

        drop(permit);

        Ok(())
    }

    async fn login() -> reqwest::Result<Self> {
        fn read(prompt: &str) -> String {
            println!("{}", prompt);
            let mut s = String::new();
            std::io::stdin().read_line(&mut s).unwrap();
            s
        }
        let username = read("username:");
        let password = read("password:");

        #[derive(Serialize)]
        struct Login<'a> {
            username: &'a str,
            password: &'a str,
        }
        let login = Login {
            username: username.trim(),
            password: password.trim(),
        };

        let permit = RATE_LIMIT.request().await;
        let now = Utc::now();
        let token = CLIENT
            .post("https://api.mangadex.org/auth/login")
            .json(&login)
            .send()
            .await?
            .error_for_status()?
            .json::<TokenResponse>()
            .await?;

        drop(permit);

        Ok(Self {
            session_token: token.session,
            refresh_token: token.refresh,
            session_expiry: now + Duration::minutes(15),
            refresh_expiry: now + Duration::days(30),
        })
    }
}
