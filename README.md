# meep [![Build Status][ci-badge]][ci] [![Crates-io][cr-badge]][cr] [![Built with Spacemacs][bws]][sm]

[ci-badge]: https://travis-ci.org/aatxe/meep.svg
[ci]: https://travis-ci.org/aatxe/meep
[cr-badge]: https://img.shields.io/crates/v/meep.svg
[cr]: https://crates.io/crates/meep
[bws]: https://cdn.rawgit.com/syl20bnr/spacemacs/442d025779da2f62fc86c2082703697714db6514/assets/spacemacs-badge.svg
[sm]: http://spacemacs.org

A simple pasting service powered by [Rocket][rocket].

[rocket]: https://rocket.rs/

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
