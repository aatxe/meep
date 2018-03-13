# meep [![Build Status][ci-badge]][ci] [![Crates-io][cr-badge]][cr] [![Built with Spacemacs][bws]][sm]

[ci-badge]: https://travis-ci.org/aatxe/meep.svg
[ci]: https://travis-ci.org/aatxe/meep
[cr-badge]: https://img.shields.io/crates/v/meep.svg
[cr]: https://crates.io/crates/meep
[bws]: https://cdn.rawgit.com/syl20bnr/spacemacs/442d025779da2f62fc86c2082703697714db6514/assets/spacemacs-badge.svg
[sm]: http://spacemacs.org

A simple pasting service powered by [Rocket][rocket].

[rocket]: https://rocket.rs/

## Installation

Setting up a new instance is easy, but it requires using Rust nightly.

1. Install `meep` with `cargo` via `cargo +nightly install --force meep`.
    - If you're not using `rustup` but still have nightly, use `cargo install --force meep` instead.
    - The `--force` flag ensures that you'll update any current installations to the latest version.
2. Run `meep` in a directory where you want it to create `meep.toml` and `meep.db`.
    - You can also create a `Rocket.toml` which can be useful, e.g. for setting production templates.
    - You can set a specific path for the configuration with the `MEEP_CONFIG` environment variable.
3. Install a set of templates to use in `templates/` within your working directory.
    - You can configure this path in `Rocket.toml` (including per-environment).
    - You can use the default templates in the repository as a starting point.
4. Edit the configuration in `meep.toml` and then re-run `meep`.
    - You should always run production instances with `ROCKET_ENV` set to `production`.
    - You should also make sure to deploy `meep` behind a proxy through `nginx` or `apache`.

## Example Configurations

### meep.toml

```toml
maintainer = "Aaron Weiss"
maintainer-email = "awe@pdgn.co"
meep-root = "https://commie.club/m"
database-url = "meep.db"
default-theme = "InspiredGitHub"
extra-syntaxes-path = "syntaxes"
```

### Rocket.toml

```toml
[development]
port = 8080
template_dir = "templates/"

[production]
port = 8080
template_dir = "prod_templates/"
```

## Manpage

```
meep(1)                              MEEP                              meep(1)

                       dMMMMMMMMb dMMMMMP dMMMMMP dMMMMb
                      dMP"dMP"dMPdMP     dMP     dMP.dMP
                     dMP dMP dMPdMMMP   dMMMP   dMMMMP"
                    dMP dMP dMPdMP     dMP     dMP
                   dMP dMP dMPdMMMMMP dMMMMMP dMP

SYNOPSIS
    <command> | curl --data-binary "@-" $MEEP_ROOT

DESCRIPTION
    Simply POST data to $MEEP_ROOT to paste

OPTIONS
    add /<ext> to resulting url for syntax highlighting
    add /<ext>/<theme> for syntax highlighting with a specific theme

THEMES
    default    $DEFAULT_THEME
    gh         InspiredGitHub
    light      Solarized (light)
    dark       Solarized (dark)

EXAMPLES
    (meep) cat src/main.rs | curl --data-binary "@-" $MEEP_ROOT
           $MEEP_ROOT/iVse
    (meep) firefox $MEEP_ROOT/iVse/rs

MAINTAINER
    Instance maintained by $MAINTAINER <$MAINTAINER_EMAIL>

SEE ALSO
    https://github.com/aatxe/meep

meep 0.0.0                                                             meep(1)
```

## Versioning

`meep` uses [semantic versioning](https://semver.org) tied to its Web API. Thus, major number
changes correspond to changes to the validity of existing routes, while minor number changes
correspond to the addition of new routes. Patch number changes will correspond to any changes that
do not affect the Web API.

## License

```
meep - a simple pasting service
Copyright (C) 2018 Aaron Weiss 

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
```
