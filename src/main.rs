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
extern crate syntect;

use std::borrow::Cow;
use std::ops::Deref;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use r2d2_diesel::ConnectionManager;
use rand::Rng;
use rocket::{Data, Outcome, Request, State};
use rocket::http::{RawStr, Status};
use rocket::request::{self, FromParam, FromRequest};

//    __             ___                 __
//   /\ \          /'___\ __          __/\ \__  __
//   \_\ \     __ /\ \__//\_\    ___ /\_\ \ ,_\/\_\    ___     ___     ____
//   /'_` \  /'__`\ \ ,__\/\ \ /' _ `\/\ \ \ \/\/\ \  / __`\ /' _ `\  /',__\
//  /\ \L\ \/\  __/\ \ \_/\ \ \/\ \/\ \ \ \ \ \_\ \ \/\ \L\ \/\ \/\ \/\__, `\
//  \ \___,_\ \____\\ \_\  \ \_\ \_\ \_\ \_\ \__\\ \_\ \____/\ \_\ \_\/\____/
//   \/__,_ /\/____/ \/_/   \/_/\/_/\/_/\/_/\/__/ \/_/\/___/  \/_/\/_/\/___/

pub static DATABASE_URL: &'static str = dotenv!("DATABASE_URL");

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

//                 ___
//   __          /'___\
//  /\_\    ___ /\ \__/  _ __    __
//  \/\ \ /' _ `\ \ ,__\/\`'__\/'__`\
//   \ \ \/\ \/\ \ \ \_/\ \ \//\ \L\.\_
//    \ \_\ \_\ \_\ \_\  \ \_\\ \__/.\_\
//     \/_/\/_/\/_/\/_/   \/_/ \/__/\/_/

fn main() {
    rocket::ignite()
        .manage(init_pool())
        .mount("/", routes![index, paste, view])
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
pub fn index() -> &'static str {
    r#"meep(1)                              MEEP                               meep(1)

     dMMMMMMMMb dMMMMMP dMMMMMP dMMMMb
    dMP"dMP"dMPdMP     dMP     dMP.dMP
   dMP dMP dMPdMMMP   dMMMP   dMMMMP"
  dMP dMP dMPdMP     dMP     dMP
 dMP dMP dMPdMMMMMP dMMMMMP dMP

SYNOPSIS
    <command> | curl --data-binary "@-" https://commie.club/meep

DESCRIPTION
    add ?<lang> to resulting url for line numbers and syntax highlighting

EXAMPLES
    [to do]

SEE ALSO
    http://github.com/aatxe/meep
"#
}

#[post("/", data = "<data>")]
pub fn paste(conn: DbConn, data: Data) -> Result<String> {
    use models::*;
    use schema::*;

    let mut buf = Vec::new();
    let _ = data.stream_to(&mut buf)?;
    let str_data = String::from_utf8(buf)?;

    let id = rand::thread_rng().gen_ascii_chars().take(4).collect();
    let url = format!("https://commie.club/meep/{}", &id);

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
    
pub mod schema {
    table! {
        pastes (id) {
            id -> Text,
            data -> Text,
        }
    }
}
