//! Request/response schema.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Target {
    Launcher,
    Settings,
    Privacy,
    Notifications,
    Power,
}

impl std::str::FromStr for Target {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "launcher" => Ok(Target::Launcher),
            "settings" => Ok(Target::Settings),
            "privacy" => Ok(Target::Privacy),
            "notifications" => Ok(Target::Notifications),
            "power" => Ok(Target::Power),
            other => Err(format!(
                "unknown target `{other}` (launcher|settings|privacy|notifications|power)"
            )),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "kebab-case")]
pub enum Request {
    Ping,
    Toggle {
        target: Target,
    },
    Show {
        target: Target,
    },
    Hide {
        target: Target,
    },
    /// Focus the terminal or filesystem panel of the shell.
    Focus {
        target: String,
    },
    /// `op`: "volume" with `delta` or `set`, "mute", "mic-mute".
    Audio {
        op: String,
        #[serde(default)]
        delta: Option<i32>,
        #[serde(default)]
        set: Option<u32>,
    },
    Brightness {
        delta: i32,
    },
    Theme {
        name: String,
    },
    Reload,
    State,
    ScreenshotScene,
    /// Show an OSD/toast from scripts.
    Notify {
        summary: String,
        #[serde(default)]
        body: String,
    },
    /// Open ranger in a new terminal tab, in `path` or the file panel's directory.
    Files {
        #[serde(default)]
        path: Option<String>,
    },
    /// Run the eDEX-OS installer or another shell-integrated action.
    Action {
        name: String,
    },
    Quit,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Response {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl Response {
    pub fn ok() -> Self {
        Self {
            ok: true,
            error: None,
            data: None,
        }
    }

    pub fn with_data(data: serde_json::Value) -> Self {
        Self {
            ok: true,
            error: None,
            data: Some(data),
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            error: Some(msg.into()),
            data: None,
        }
    }
}

/// Parse a command line such as `toggle launcher` or `audio volume +5` into a request.
pub fn parse_args(args: &[String]) -> Result<Request, String> {
    let mut it = args.iter().map(String::as_str);
    let cmd = it.next().ok_or("missing command")?;
    let arg = |it: &mut dyn Iterator<Item = &str>, name: &str| -> Result<String, String> {
        it.next()
            .map(|s| s.to_string())
            .ok_or_else(|| format!("missing {name}"))
    };
    Ok(match cmd {
        "ping" => Request::Ping,
        "toggle" => Request::Toggle {
            target: arg(&mut it, "target")?.parse()?,
        },
        "show" => Request::Show {
            target: arg(&mut it, "target")?.parse()?,
        },
        "hide" => Request::Hide {
            target: arg(&mut it, "target")?.parse()?,
        },
        "focus" => Request::Focus {
            target: arg(&mut it, "target")?,
        },
        "audio" => {
            let op = arg(&mut it, "op")?;
            let value = it.next();
            match op.as_str() {
                "volume" => {
                    let v = value.ok_or("missing volume delta (+5/-5) or value")?;
                    if let Some(rest) = v.strip_prefix('+') {
                        Request::Audio {
                            op,
                            delta: Some(rest.parse().map_err(|_| "bad delta")?),
                            set: None,
                        }
                    } else if v.starts_with('-') {
                        Request::Audio {
                            op,
                            delta: Some(v.parse().map_err(|_| "bad delta")?),
                            set: None,
                        }
                    } else {
                        Request::Audio {
                            op,
                            delta: None,
                            set: Some(v.parse().map_err(|_| "bad value")?),
                        }
                    }
                }
                "mute" | "mic-mute" => Request::Audio {
                    op,
                    delta: None,
                    set: None,
                },
                other => return Err(format!("unknown audio op `{other}`")),
            }
        }
        "brightness" => {
            let v = arg(&mut it, "delta")?;
            Request::Brightness {
                delta: v.trim_start_matches('+').parse().map_err(|_| "bad delta")?,
            }
        }
        "theme" => Request::Theme {
            name: arg(&mut it, "name")?,
        },
        "reload" => Request::Reload,
        "state" => Request::State,
        "screenshot-scene" => Request::ScreenshotScene,
        "notify" => {
            let summary = arg(&mut it, "summary")?;
            let body = it.collect::<Vec<_>>().join(" ");
            Request::Notify { summary, body }
        }
        "files" => Request::Files {
            path: {
                let p = it.collect::<Vec<_>>().join(" ");
                (!p.is_empty()).then_some(p)
            },
        },
        "action" => Request::Action {
            name: arg(&mut it, "name")?,
        },
        "quit" => Request::Quit,
        other => return Err(format!("unknown command `{other}`")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cli_and_roundtrips_json() {
        let r = parse_args(&["audio".into(), "volume".into(), "+5".into()]).unwrap();
        assert_eq!(
            r,
            Request::Audio {
                op: "volume".into(),
                delta: Some(5),
                set: None
            }
        );
        let r = parse_args(&["toggle".into(), "launcher".into()]).unwrap();
        let json = serde_json::to_string(&r).unwrap();
        assert_eq!(json, r#"{"cmd":"toggle","target":"launcher"}"#);
        let back: Request = serde_json::from_str(&json).unwrap();
        assert_eq!(back, r);
        assert!(parse_args(&["toggle".into(), "nope".into()]).is_err());
        assert_eq!(
            parse_args(&["brightness".into(), "-10".into()]).unwrap(),
            Request::Brightness { delta: -10 }
        );
    }
}
