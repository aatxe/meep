//!     dMMMMMMMMb dMMMMMP dMMMMMP dMMMMb
//!    dMP"dMP"dMPdMP     dMP     dMP.dMP
//!   dMP dMP dMPdMMMP   dMMMP   dMMMMP"
//!  dMP dMP dMPdMP     dMP     dMP
//! dMP dMP dMPdMMMMMP dMMMMMP dMP
//!
//! meep - a simple pasting service
//! Copyright (C) 2018 Aaron Weiss
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

#![feature(custom_derive, plugin)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate diesel;
extern crate dotenv;
#[macro_use]
extern crate dotenv_codegen;
extern crate failure;
extern crate r2d2_diesel;
extern crate r2d2;
extern crate rand;
extern crate rocket;
extern crate rocket_contrib;
extern crate syntect;

use std::borrow::Cow;
use std::convert::From;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use r2d2_diesel::ConnectionManager;
use rand::Rng;
use rocket::{Data, Outcome, Request, State};
use rocket::http::{RawStr, Status};
use rocket::request::{self, FromParam, FromRequest};
use rocket_contrib::Template;
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

pub static MEEP_ROOT: &'static str = dotenv!("MEEP_ROOT");
pub static DATABASE_URL: &'static str = dotenv!("DATABASE_URL");
pub static SYNTECT_THEME: &'static str = dotenv!("SYNTECT_THEME");

pub type Result<T> = std::result::Result<T, failure::Error>;
pub type Pool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

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

    fn from_param(param: &'a RawStr) -> std::result::Result<PasteId<'a>, &'a RawStr> {
        let valid = param.chars().all(|c| {
            (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9')
        });

        if valid {
            Ok(PasteId(Cow::Borrowed(param)))
        } else {
            Err(param)
        }
    }
}

pub struct Extension<'a>(Cow<'a, str>);

impl<'a> FromParam<'a> for Extension<'a> {
    type Error = &'a RawStr;

    fn from_param(param: &'a RawStr) -> std::result::Result<Extension<'a>, &'a RawStr> {
        let valid = param.chars().all(|c| {
            (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9')
        });

        if valid {
            Ok(Extension(Cow::Borrowed(param)))
        } else {
            Err(param)
        }
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
            let mut ss = SyntaxSet::new();
            ss.load_syntaxes(path, true).map_err(|_| ())?;
            Ok(ss)
        }).unwrap_or_else(|()| SyntaxSet::load_defaults_newlines())
    }

    pub fn themes(&self) -> ThemeSet {
        self.paths.ss_path.as_ref().ok_or_else(|| ()).and_then(|path| {
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
    InspiredGitHub,
    SolarizedLight,
    SolarizedDark,
}

impl Theme {
    fn str(&self) -> &'static str {
        match *self {
            Theme::InspiredGitHub => "InspiredGitHub",
            Theme::SolarizedLight => "Solarized (light)",
            Theme::SolarizedDark  => "Solarized (dark)",
        }
    }
}

impl<'a> FromParam<'a> for Theme {
    type Error = &'a RawStr;

    fn from_param(param: &'a RawStr) -> std::result::Result<Theme, &'a RawStr> {
        let branch_str = if param == "default" {
            SYNTECT_THEME
        } else {
            param
        };

        Ok(match branch_str {
            "InspiredGitHub" | "gh"    => Theme::InspiredGitHub,
            "SolarizedLight" | "light" => Theme::SolarizedLight,
            "SolarizedDark"  | "dark"  => Theme::SolarizedDark,

            _ => return Err(param),
        })
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
    rocket::ignite()
        .manage(Highlighting::from(SyntectPaths::new()))
        .manage(init_pool())
        .mount("/", routes![index, paste, view, view_highlighted, view_highlighted_themed])
        .catch(errors![not_found])
        .attach(Template::fairing())
        .launch();
}

pub fn init_pool() -> Pool {
    let manager = ConnectionManager::<SqliteConnection>::new(DATABASE_URL);
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
pub fn index() -> String {
    format!(r#"meep(1)                              MEEP                               meep(1)

                       dMMMMMMMMb dMMMMMP dMMMMMP dMMMMb
                      dMP"dMP"dMPdMP     dMP     dMP.dMP
                     dMP dMP dMPdMMMP   dMMMP   dMMMMP"
                    dMP dMP dMPdMP     dMP     dMP
                   dMP dMP dMPdMMMMMP dMMMMMP dMP

SYNOPSIS
    <command> | curl --data-binary "@-" {root}

DESCRIPTION
    add ?<lang> to resulting url for line numbers and syntax highlighting

EXAMPLES
    (meep) cat src/main.rs | curl --data-binary "@-" {root}
           {root}/iVse
    (meep) firefox {root}/iVse/rs

SEE ALSO
    http://github.com/aatxe/meep
"#, root=MEEP_ROOT)
}

#[post("/", data = "<data>")]
pub fn paste(conn: DbConn, data: Data) -> Result<String> {
    use models::*;
    use schema::*;

    let mut buf = Vec::new();
    let _ = data.stream_to(&mut buf)?;
    let str_data = String::from_utf8(buf)?;

    let id = rand::thread_rng().gen_ascii_chars().take(4).collect();
    let url = format!("{}/{}", MEEP_ROOT, &id);

    let paste = Paste {
        id: id,
        data: str_data,
    };

    diesel::insert_into(pastes::table)
        .values(&paste)
        .execute(&*conn)?;

    Ok(url)
}

#[get("/<pid>")]
pub fn view(conn: DbConn, pid: PasteId) -> Result<String> {
    use models::*;
    use schema::pastes::dsl::*;

    let paste = pastes.find(pid.0)
        .first::<Paste>(&*conn)?;

    Ok(paste.data)
}

#[get("/<pid>/<ext>")]
pub fn view_highlighted(
    conn: DbConn, syntaxes: Syntaxes, themes: Themes, pid: PasteId, ext: Extension, 
) -> Result<Template> {
    impl_view_highlighted(conn, syntaxes, themes, pid, ext, None)
}

#[get("/<pid>/<ext>/<theme>")]
pub fn view_highlighted_themed(
    conn: DbConn, syntaxes: Syntaxes, themes: Themes, pid: PasteId, ext: Extension, theme: Theme,
) -> Result<Template> {
    impl_view_highlighted(conn, syntaxes, themes, pid, ext, Some(theme))
}

pub fn impl_view_highlighted(
    conn: DbConn, syntaxes: Syntaxes, themes: Themes, pid: PasteId, ext: Extension,
    theme: Option<Theme>,
) -> Result<Template> {
    use models::*;
    use schema::pastes::dsl::*;

    let paste = pastes.find(pid.0)
        .first::<Paste>(&*conn)?;

    let theme = themes.themes.get(theme.map(|t| t.str()).unwrap_or(SYNTECT_THEME)).ok_or_else(|| {
        failure::err_msg(format!("could not find theme: {}", SYNTECT_THEME))
    })?;
    let syntax = syntaxes.find_syntax_by_extension(&ext.0).or_else(|| {
        syntaxes.find_syntax_by_first_line(&paste.data)
    });

    let content = match syntax {
        Some(syntax) => html::highlighted_snippet_for_string(&paste.data, &syntax, theme),
        None => paste.data,
    };

    let bg = theme.settings.background.unwrap_or(Color::WHITE);
    let bg_color = format!("#{:02x}{:02x}{:02x}", bg.r, bg.g, bg.b);

    let mut cxt = std::collections::HashMap::new();
    cxt.insert("contents", content);
    cxt.insert("background", bg_color);
    Ok(Template::render("hl_view", cxt))
}

#[error(404)]
fn not_found(req: &Request) -> Template {
    let mut map = std::collections::HashMap::new();
    map.insert("path", req.uri().as_str());
    Template::render("error/404", &map)
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

//              __
//             /\ \
//  ____    ___\ \ \___      __    ___ ___      __
//  /',__\  /'___\ \  _ `\  /'__`\/' __` __`\  /'__`\
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
