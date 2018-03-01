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

#![feature(plugin)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate diesel;
extern crate dotenv;
#[macro_use]
extern crate dotenv_codegen;
extern crate r2d2_diesel;
extern crate r2d2;
extern crate rocket;
extern crate syntect;

use diesel::sqlite::SqliteConnection;
use r2d2_diesel::ConnectionManager;

//    __             ___                 __
//   /\ \          /'___\ __          __/\ \__  __
//   \_\ \     __ /\ \__//\_\    ___ /\_\ \ ,_\/\_\    ___     ___     ____
//   /'_` \  /'__`\ \ ,__\/\ \ /' _ `\/\ \ \ \/\/\ \  / __`\ /' _ `\  /',__\
//  /\ \L\ \/\  __/\ \ \_/\ \ \/\ \/\ \ \ \ \ \_\ \ \/\ \L\ \/\ \/\ \/\__, `\
//  \ \___,_\ \____\\ \_\  \ \_\ \_\ \_\ \_\ \__\\ \_\ \____/\ \_\ \_\/\____/
//   \/__,_ /\/____/ \/_/   \/_/\/_/\/_/\/_/\/__/ \/_/\/___/  \/_/\/_/\/___/


type Pool = r2d2::Pool<ConnectionManager<SqliteConnection>>;
pub struct DbConn(pub r2d2::PooledConnection<ConnectionManager<SqliteConnection>>);

static DATABASE_URL: &'static str = dotenv!("DATABASE_URL");

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
        .mount("/", routes![index])
        .launch();
}

fn init_pool() -> Pool {
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
fn index() -> &'static str {
    "Hello, world!"
}
