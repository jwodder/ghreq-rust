[![Project Status: WIP – Initial development is in progress, but there has not yet been a stable, usable release suitable for the public.](https://www.repostatus.org/badges/latest/wip.svg)](https://www.repostatus.org/#wip)
[![CI Status](https://github.com/jwodder/ghreq-rust/actions/workflows/test.yml/badge.svg)](https://github.com/jwodder/ghreq-rust/actions/workflows/test.yml)
[![codecov.io](https://codecov.io/gh/jwodder/ghreq-rust/branch/main/graph/badge.svg)](https://codecov.io/gh/jwodder/ghreq-rust)
[![Minimum Supported Rust Version](https://img.shields.io/badge/MSRV-1.79-orange)](https://www.rust-lang.org)
[![MIT License](https://img.shields.io/github/license/jwodder/ghreq-rust.svg)](https://opensource.org/licenses/MIT)

[GitHub](https://github.com/jwodder/ghreq-rust) | [Issues](https://github.com/jwodder/ghreq-rust/issues)

`ghreq` is an extensible sync & async [Rust](https://www.rust-lang.org) client
library for the [GitHub REST API](https://docs.github.com/en/rest).

Most GitHub REST API libraries have a problem: They define exactly what fields
you can send in requests and exactly what fields they'll parse from responses.
Unfortunately, the GitHub REST API changes often, frequently adding request &
response fields, and the libraries don't always stay up to date.  Even worse is
when the API adds a whole new endpoint and your favorite library doesn't
support making any requests at all to it!

<!-- TODO: Distinguish from <https://github.com/XAMPPRocky/octocrab> -->

<!-- Based on architecture described in <https://users.rust-lang.org/t/34567/3> -->
