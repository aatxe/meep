//!     dMMMMMMMMb dMMMMMP dMMMMMP dMMMMb
//!    dMP"dMP"dMPdMP     dMP     dMP.dMP
//!   dMP dMP dMPdMMMP   dMMMP   dMMMMP"
//!  dMP dMP dMPdMP     dMP     dMP
//! dMP dMP dMPdMMMMMP dMMMMMP dMP
//!
//! meep - a simple pasting service
//! Copyright (C) 2018-2019 Aaron Weiss
//!
//! This program is free software: you can redistribute it and/or modify
//! it under the terms of the GNU Affero General Public License as published by
//! the Free Software Foundation, either version 3 of the License, or
//! (at your option) any later version.
//!
//! This program is distributed in the hope that it will be useful,
//! but WITHOUT ANY WARRANTY; without even the implied warranty of
//! MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
//! GNU Affero General Public License for more details.
//!
//! You should have received a copy of the GNU Affero General Public License
//! along with this program.  If not, see <https://www.gnu.org/licenses/>.

#![feature(proc_macro_hygiene, decl_macro, integer_atomics)]

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

use std::borrow::Cow;
use std::convert::From;
use std::env;
use std::fs::File;
use std::iter;
use std::io::prelude::*;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, AtomicU8, Ordering};

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel::result::{Error as QueryError, DatabaseErrorKind};
use dotenv_codegen::dotenv;
use fehler::{error, throw, throws};
use r2d2_diesel::ConnectionManager;
use rand::Rng;
use rocket::{Data, Outcome, Request, State};
use rocket::http::{RawStr, Status};
use rocket::request::{self, FromParam, FromRequest};
use rocket_codegen::{catch, catchers, get, post, routes};
use rocket_contrib::templates::Template;
use serde_derive::{Deserialize, Serialize};
use syntect::parsing::SyntaxSet;
use syntect::highlighting::{Color, ThemeSet};
use syntect::html;

//    __             ___                 __
//   /\ \          /'___\ __          __/\ \__  __
//   \_\ \     __ /\ \__//\_\    ___ /\_\ \ ,_\/\_\    ___     ___     ____
//   /'_` \  /'__`\ \ ,__\/\ \ /' _ `\/\ \ \ \/\/\ \  / __`\ /' _ `\  /',__\
//  /\ \L\ \/\  __/\ \ \_/\ \ \/\ \/\ \ \ \ \ \_\ \ \/\ \L\ \/\ \/\ \/\__, `\
//  \ \___,_\ \____\\ \_\  \ \_\ \_\ \_\ \_\ \__\\ \_\ \____/\ \_\ \_\/\____/
//   \/__,_ /\/____/ \/_/   \/_/\/_/\/_/\/_/\/__/ \/_/\/___/  \/_/\/_/\/___/

embed_migrations!();
pub static MAN_WIDTH: usize = 78;
fn mk_unknown() -> String { "unknown".to_owned() }
fn default_meep_root() -> String { "http://localhost:8000".to_owned() }

pub type Pool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(default = "mk_unknown")]
    maintainer: String,
    #[serde(default = "mk_unknown")]
    maintainer_email: String,

    #[serde(default = "default_meep_root")]
    meep_root: String,
    database_url: String,
    default_theme: String,
    extra_syntaxes_path: String,
}

impl Config {
    #[throws]
    pub fn load<P: AsRef<Path>>(path: P) -> Config {
        let contents = File::open(&path).and_then(|mut file| {
            let mut buf = String::new();
            file.read_to_string(&mut buf).map(|_| buf)
        })?;

        toml::from_str(&contents[..])?
    }

    #[throws]
    pub fn save<P: AsRef<Path>>(&self, path: P) -> () {
        let mut file = File::create(&path)?;
        let contents = toml::to_string(self)?;
        file.write_all(contents.as_bytes())?;
    }
}

#[derive(Clone, Debug)]
pub struct Conf(Arc<Config>);

impl<'a, 'r> FromRequest<'a, 'r> for Conf {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Conf, ()> {
        let conf = request.guard::<State<Conf>>()?;
        Outcome::Success(conf.clone())
    }
}

impl Deref for Conf {
    type Target = Config;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct DbConn(pub r2d2::PooledConnection<ConnectionManager<SqliteConnection>>);

impl<'a, 'r> FromRequest<'a, 'r> for DbConn {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<DbConn, ()> {
        let pool = request.guard::<State<Pool>>()?;
        match pool.get() {
            Ok(conn) => Outcome::Success(DbConn(conn)),
            Err(_) => Outcome::Failure((Status::ServiceUnavailable, ()))
        }
    }
}

impl Deref for DbConn {
    type Target = SqliteConnection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct PasteId<'a>(Cow<'a, str>);

impl<'a> FromParam<'a> for PasteId<'a> {
    type Error = &'a RawStr;

    #[throws(&'a RawStr)]
    fn from_param(param: &'a RawStr) -> PasteId<'a> {
        let valid = param.chars().all(|c| {
            (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9')
        });

        if !valid {
            throw!(param)
        }

        PasteId(Cow::Borrowed(param))
    }
}

pub struct Extension<'a>(Cow<'a, str>);

impl<'a> FromParam<'a> for Extension<'a> {
    type Error = &'a RawStr;

    #[throws(&'a RawStr)]
    fn from_param(param: &'a RawStr) -> Extension<'a> {
        let valid = param.chars().all(|c| {
            (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') || c == '.'
        });

        if !valid {
            throw!(param)
        }

        Extension(Cow::Borrowed(param))
    }
}

pub struct SyntectPaths {
    ss_path: Option<PathBuf>,
    ts_path: Option<PathBuf>,
}

impl SyntectPaths {
    pub fn new() -> SyntectPaths {
        SyntectPaths {
            ss_path: None,
            ts_path: None,
        }
    }

    pub fn syntaxes<P: AsRef<Path>>(mut self, path: P) -> SyntectPaths {
        self.ss_path = Some(path.as_ref().to_path_buf());
        self
    }

    pub fn themes<P: AsRef<Path>>(mut self, path: P) -> SyntectPaths {
        self.ts_path = Some(path.as_ref().to_path_buf());
        self
    }
}

pub struct Highlighting {
    paths: Arc<SyntectPaths>,
}

impl From<SyntectPaths> for Highlighting {
    fn from(paths: SyntectPaths) -> Self {
        Highlighting {
            paths: Arc::new(paths),
        }
    }
}

impl Highlighting {
    pub fn syntaxes(&self) -> SyntaxSet {
        self.paths.ss_path.as_ref().ok_or_else(|| ()).and_then(|path| {
            SyntaxSet::load_from_folder(path).map_err(|_| ())
        }).unwrap_or_else(|()| SyntaxSet::load_defaults_nonewlines())
    }

    pub fn themes(&self) -> ThemeSet {
        self.paths.ts_path.as_ref().ok_or_else(|| ()).and_then(|path| {
            ThemeSet::load_from_folder(path).map_err(|_| ())
        }).unwrap_or_else(|()| ThemeSet::load_defaults())
    }
}

pub struct Syntaxes(SyntaxSet);

impl<'a, 'r> FromRequest<'a, 'r> for Syntaxes {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Syntaxes, ()> {
        let highlighting = request.guard::<State<Highlighting>>()?;
        Outcome::Success(Syntaxes(highlighting.syntaxes()))
    }
}

impl Deref for Syntaxes {
    type Target = SyntaxSet;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct Themes(ThemeSet);

impl<'a, 'r> FromRequest<'a, 'r> for Themes {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Themes, ()> {
        let highlighting = request.guard::<State<Highlighting>>()?;
        Outcome::Success(Themes(highlighting.themes()))
    }
}

impl Deref for Themes {
    type Target = ThemeSet;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Copy, Clone)]
pub enum Theme {
    Default,
    InspiredGitHub,
    SolarizedLight,
    SolarizedDark,
}

impl Theme {
    fn str(&self) -> Option<&'static str> {
        match *self {
            Theme::Default => None,
            Theme::InspiredGitHub => Some("InspiredGitHub"),
            Theme::SolarizedLight => Some("Solarized (light)"),
            Theme::SolarizedDark  => Some("Solarized (dark)"),
        }
    }
}

impl<'a> FromParam<'a> for Theme {
    type Error = &'a RawStr;

    #[throws(&'a RawStr)]
    fn from_param(param: &'a RawStr) -> Theme {
        match &param.url_decode().map_err(|_| param)?[..] {
            "InspiredGitHub" | "gh"    => Theme::InspiredGitHub,
            "SolarizedLight" | "light" => Theme::SolarizedLight,
            "SolarizedDark"  | "dark"  => Theme::SolarizedDark,

            _ => throw!(param),
        }
    }
}

pub struct Load { occupied: AtomicI64 }
pub struct KeyLength { length: AtomicU8 }

impl Load {
    pub fn new() -> Load {
        Load { occupied: AtomicI64::new(0) }
    }
}

impl KeyLength {
    pub fn new() -> KeyLength {
        KeyLength { length: AtomicU8::new(4) }
    }
}

//                 ___
//   __          /'___\
//  /\_\    ___ /\ \__/  _ __    __
//  \/\ \ /' _ `\ \ ,__\/\`'__\/'__`\
//   \ \ \/\ \/\ \ \ \_/\ \ \//\ \L\.\_
//    \ \_\ \_\ \_\ \_\  \ \_\\ \__/.\_\
//     \/_/\/_/\/_/\/_/   \/_/ \/__/\/_/

fn main() {
    let path = env::var("MEEP_CONFIG").unwrap_or_else(|_| "meep.toml".to_owned());
    let config = match Config::load(&path) {
        Ok(cfg) => cfg,
        Err(_) => {
            let cfg = Config {
                maintainer: "unknown".to_owned(),
                maintainer_email: "unknown".to_owned(),
                meep_root: dotenv!("MEEP_ROOT").to_owned(),
                database_url: dotenv!("DATABASE_URL").to_owned(),
                default_theme: dotenv!("SYNTECT_THEME").to_owned(),
                extra_syntaxes_path: dotenv!("SYNTAX_PATH").to_owned(),
            };
            cfg.save(&path).unwrap_or_else(|_| {
                eprintln!("failed to save default config to {}", path);
                process::exit(1);
            });
            println!("wrote out a default configuration to {}", path);
            println!("please edit the configuration and rerun meep");
            process::exit(0);
        }
    };

    let connection = SqliteConnection::establish(&config.database_url).unwrap_or_else(|e| {
        eprintln!("failed to connect to database at {}", &config.database_url);
        eprintln!("error: {}", e);
        process::exit(1);
    });
    embedded_migrations::run(&connection).unwrap_or_else(|e| {
        eprintln!("failed to configure database at {}", &config.database_url);
        eprintln!("error: {}", e);
        process::exit(1);
    });

    rocket::ignite()
        .manage(Load::new())
        .manage(KeyLength::new())
        .manage(Highlighting::from(SyntectPaths::new().syntaxes(&config.extra_syntaxes_path)))
        .manage(init_pool(&config))
        .manage(Conf(Arc::new(config)))
        .mount("/", routes![index, paste, view, view_highlighted, view_highlighted_themed])
        .register(catchers![not_found, internal_server_error])
        .attach(Template::fairing())
        .launch();
}

pub fn init_pool(config: &Config) -> Pool {
    let manager = ConnectionManager::<SqliteConnection>::new(config.database_url.clone());
    r2d2::Pool::new(manager).expect("failed to create db pool")
}

//                       __
//                      /\ \__
//   _ __   ___   __  __\ \ ,_\    __    ____
//  /\`'__\/ __`\/\ \/\ \\ \ \/  /'__`\ /',__\
//  \ \ \//\ \L\ \ \ \_\ \\ \ \_/\  __//\__, `\
//   \ \_\\ \____/\ \____/ \ \__\ \____\/\____/
//    \/_/ \/___/  \/___/   \/__/\/____/\/___/

#[get("/")]
pub fn index(conf: Conf) -> String {
    let header = {
        let mut str = String::with_capacity(MAN_WIDTH);
        let base = "meep(1)";
        let centered = "MEEP";
        str.push_str(base);
        let spacing = MAN_WIDTH - (base.len() * 2) - centered.len();
        let spaces: String = iter::repeat(' ').take(spacing / 2).collect();
        str.push_str(&spaces);
        str.push_str(centered);
        str.push_str(&spaces);
        str.push_str(base);
        str
    };

    let footer = {
        let mut str = String::with_capacity(MAN_WIDTH);
        str.push_str("meep ");
        str.push_str(env!("CARGO_PKG_VERSION"));
        let end = "meep(1)";
        let spacing = MAN_WIDTH - str.len() - end.len();
        str.push_str(
            &iter::repeat(' ').take(spacing).collect::<String>()
        );
        str.push_str(end);
        str
    };

    format!(r#"{header}

                       dMMMMMMMMb dMMMMMP dMMMMMP dMMMMb
                      dMP"dMP"dMPdMP     dMP     dMP.dMP
                     dMP dMP dMPdMMMP   dMMMP   dMMMMP"
                    dMP dMP dMPdMP     dMP     dMP
                   dMP dMP dMPdMMMMMP dMMMMMP dMP

SYNOPSIS
    <command> | curl --data-binary "@-" {root}/

DESCRIPTION
    Simply POST data to {root}/ to paste

OPTIONS
    add /<ext> to resulting url for syntax highlighting
    add /<ext>/<theme> for syntax highlighting with a specific theme

THEMES
    default    {theme}
    gh         InspiredGitHub
    light      Solarized (light)
    dark       Solarized (dark)

EXAMPLES
    (meep) cat src/main.rs | curl --data-binary "@-" {root}/
           {root}/iVse
    (meep) firefox {root}/iVse/rs

MAINTAINER
    Instance maintained by {maintainer} <{email}>

SEE ALSO
    https://github.com/aatxe/meep

{footer}"#,
            root=&conf.meep_root, theme=&conf.default_theme, header=header, footer=footer,
            maintainer=&conf.maintainer, email=&conf.maintainer_email,
    )
}

#[throws]
#[post("/", data = "<in_data>")]
pub fn paste(
    conf: Conf, conn: DbConn, load: State<Load>, key_len: State<KeyLength>, in_data: Data
) -> String {
    use models::*;
    use schema::pastes::dsl::*;

    let total_entries = {
        let tmp = load.occupied.load(Ordering::Relaxed);
        if tmp == 0 { // i.e. we haven't loaded the count yet
            let val = pastes.count().get_result(&*conn)?;
            load.occupied.store(val, Ordering::Relaxed);
            val
        } else {
            tmp
        }
    };
    let max_entries = i64::pow(62, u32::from(key_len.length.load(Ordering::Relaxed)));

    // total_entries >= 0.75 * max_entries, but without the floating point
    // i.e. if our load factor is more than 0.75, increase the key size to prevent high collisions.
    // at load 0.75, ~1% chance of taking more than 16 attempts to find an unused key.
    let key_length = if total_entries / 3 >= max_entries / 4 {
        key_len.length.fetch_add(1, Ordering::Relaxed) + 1
    } else {
        key_len.length.load(Ordering::Relaxed)
    };

    let mut buf = Vec::new();
    let _ = in_data.stream_to(&mut buf)?;
    let str_data = String::from_utf8(buf)?;

    let mut rng = rand::thread_rng();
    let mut url;
    while {
        let ident = iter::repeat(())
            .map(|()| rng.sample(rand::distributions::Alphanumeric))
            .take(usize::from(key_length))
            .collect();
        url = format!("{}/{}", &conf.meep_root, &ident);

        let paste = Paste {
            id: ident,
            data: str_data.clone(),
        };

        let res = diesel::insert_into(pastes)
            .values(&paste)
            .execute(&*conn);

        match res {
            // insertion was successful, don't retry.
            Ok(_) => false,
            // insertion failed due to primary key constraints, retry.
            Err(QueryError::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => true,
            // insertion failed for some other reason, bail.
            Err(e) => throw!(e),
        }
    } { continue }

    // we've successfully inserted a new row, so increase the occupied entries count.
    load.occupied.fetch_add(1, Ordering::Relaxed);
    url
}

#[throws]
#[get("/<pid>")]
pub fn view(conn: DbConn, pid: PasteId) -> Option<String> {
    use models::*;
    use schema::pastes::dsl::*;

    let paste = match pastes.find(pid.0).first::<Paste>(&*conn) {
        Ok(res) => res,
        Err(QueryError::NotFound) => return None,
        Err(e) => throw!(e),
    };

    Some(paste.data)
}

#[throws]
#[get("/<pid>/<ext>")]
pub fn view_highlighted(
    conf: Conf, conn: DbConn, syntaxes: Syntaxes, themes: Themes, pid: PasteId, ext: Extension,
) -> Option<Template> {
    impl_view_highlighted(conf, conn, syntaxes, themes, pid, ext, None)?
}

#[throws]
#[get("/<pid>/<ext>/<theme>")]
pub fn view_highlighted_themed(
    conf: Conf, conn: DbConn, syntaxes: Syntaxes, themes: Themes, pid: PasteId, ext: Extension,
    theme: Theme,
) -> Option<Template> {
    impl_view_highlighted(conf, conn, syntaxes, themes, pid, ext, Some(theme))?
}

// Shared implementation for highlighted view routes.
#[throws]
fn impl_view_highlighted(
    conf: Conf, conn: DbConn, syntaxes: Syntaxes, themes: Themes, pid: PasteId, ext: Extension,
    theme: Option<Theme>,
) -> Option<Template> {
    use models::*;
    use schema::pastes::dsl::*;

    let paste = match pastes.find(pid.0).first::<Paste>(&*conn) {
        Ok(res) => res,
        Err(QueryError::NotFound) => return None,
        Err(e) => throw!(e),
    };

    let theme_name = theme.and_then(|t| t.str()).unwrap_or(&conf.default_theme);
    let theme = themes.themes.get(theme_name).ok_or_else(|| {
        error!(format!("could not find theme: {}", &conf.default_theme))
    })?;
    let syntax = syntaxes.find_syntax_by_extension(&ext.0).or_else(|| {
        syntaxes.find_syntax_by_first_line(&paste.data)
    });

    let content = match syntax {
        Some(syntax) => html::highlighted_html_for_string(&paste.data, &syntaxes.0, syntax, theme),
        None => format!("<pre>{}</pre>", paste.data),
    };

    let bg = theme.settings.background.unwrap_or(Color::WHITE);
    let bg_color = format!("#{:02x}{:02x}{:02x}", bg.r, bg.g, bg.b);

    let mut cxt = std::collections::HashMap::new();
    cxt.insert("contents", content);
    cxt.insert("background", bg_color);

    Some(Template::render("hl_view", cxt))
}

#[catch(404)]
pub fn not_found(req: &Request) -> Template {
    let mut map = std::collections::HashMap::new();
    map.insert("path", req.uri().to_string());
    Template::render("error/404", &map)
}

#[catch(500)]
pub fn internal_server_error(req: &Request) -> Template {
    let mut map = std::collections::HashMap::new();
    map.insert("path", req.uri().to_string());
    Template::render("error/500", &map)
}

//                        __          ___
//                       /\ \        /\_ \
//    ___ ___     ___    \_\ \     __\//\ \     ____
//  /' __` __`\  / __`\  /'_` \  /'__`\\ \ \   /',__\
//  /\ \/\ \/\ \/\ \L\ \/\ \L\ \/\  __/ \_\ \_/\__, `\
//  \ \_\ \_\ \_\ \____/\ \___,_\ \____\/\____\/\____/
//   \/_/\/_/\/_/\/___/  \/__,_ /\/____/\/____/\/___/

pub mod models {
    use super::schema::pastes;

    #[derive(Insertable, Queryable)]
    #[table_name="pastes"]
    pub struct Paste {
        pub id: String,
        pub data: String,
    }
}

//                __
//               /\ \
//    ____    ___\ \ \___      __    ___ ___      __
//   /',__\  /'___\ \  _ `\  /'__`\/' __` __`\  /'__`\
//  /\__, `\/\ \__/\ \ \ \ \/\  __//\ \/\ \/\ \/\ \L\.\
//  \/\____/\ \____\\ \_\ \_\ \____\ \_\ \_\ \_\ \__/.\_\
//   \/___/  \/____/ \/_/\/_/\/____/\/_/\/_/\/_/\/__/\/_/

pub mod schema {
    table! {
        pastes (id) {
            id -> Text,
            data -> Text,
        }
    }
}
