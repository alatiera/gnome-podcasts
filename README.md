# Hammond
## Multithreaded, safe, and reliable Gtk+ Podcast client.
This is a prototype of a podcast client written in Rust.

[![pipeline status](https://gitlab.gnome.org/alatiera/Hammond/badges/master/pipeline.svg)](https://gitlab.gnome.org/alatiera/Hammond/commits/master)

![podcasts_view](./assets/podcasts_view.png)
![podcast_widget](./assets/podcast_widget.png)

## Getting in Touch
If you have any questions regarding the
use or development of Hammond, want to discuss design or simply hang out, please join us in [#hammond on irc.gnome.org.](irc://irc.gnome.org/#hammond)

Sidenote:

There isn't much documentation yet, so you will probably have question about parts of the Code.

## Quick start
The following steps assume you have a working installation of rustc and cargo.
If you dont take a look at [rustup.rs](rustup.rs)

```sh
git clone https://gitlab.gnome.org/alatiera/hammond.git
cd Hammond/
cargo run -p hammond-gtk --release
```

## Install from soure
```sh
git clone https://gitlab.gnome.org/alatiera/hammond.git
cd Hammond/
./configure --prefix=/usr/local
make && sudo make install
```

**Additionall:**

You can run `sudo make uninstall` for removal

And `make clean` to clean up the enviroment after instalation.

### Flatpak
Flatpak instructions... Soon™.

## Building

###  Dependancies

* Rust stable 1.21 or later.
* Gtk+ 3.22 or later
* Meson

**Debian/Ubuntu**:
```sh
apt-get update -yqq
apt-get install -yqq --no-install-recommends build-essential
apt-get install -yqq --no-install-recommends libgtk-3-dev meson
```

**Fedora**:
```sh
dnf install -y gtk3-devel glib2-devel openssl-devel sqlite-devel meson
```

If you happen to build it on other distributions please let me know the names of the corresponding libraries. Feel free to open a PR or an Issue to note it.
```sh
git clone https://gitlab.gnome.org/alatiera/Hammond.git
cd Hammond/
cargo build --all
```

## Overview

```sh
$ tree -d
├── assets              # png's used in the README.md
├── hammond-data        # Storate related stuff, Sqlite db, XDG setup.
│   ├── migrations      # Diesel migrations.
│   │   └── ...
│   ├── src
│   └── tests
│       └── feeds       # Raw RSS Feeds used for tests.
├── hammond-downloader  # Really basic, Really crappy downloader.
│   └── src
├── hammond-gtk         # The Gtk+ Client
│   ├── resources       # GResources folder
│   │   └── gtk         # Contains the glade.ui files.
│   └── src
│       ├── views       # Currently only contains the Podcasts_view.
│       └── widgets     # Contains custom widgets such as Podcast and Episode.
```

## Contributing
There alot of thins yet to be done.

You can find start by taking a look at [Issues](https://gitlab.gnome.org/alatiera/Hammond/issues) or Opening a [New one](https://gitlab.gnome.org/alatiera/Hammond/issues/new?issue%5Bassignee_id%5D=&issue%5Bmilestone_id%5D=).

You may also want to take a look at [TODO.md](https://gitlab.gnome.org/alatiera/Hammond/blob/master/TODO.md) or grep the source code for `TODO:` and `FIXME:` tags.

If you want to contribute, please check the [Contributions Guidelines][contribution-guidelines].
[contribution-guidelines]: https://gitlab.gnome.org/GNOME/gnome-todo/blob/master/CONTRIBUTING.md

## A note about the project's name

The project was named after Allan Moore's character [Evey Hammond](https://en.wikipedia.org/wiki/Evey_Hammond) from the graphic novel V for Vendetta.

It has nothing to do with the horrible headlines on the news.

## Acknowledgments

Hammond's design is heavily insired by [Gnome-Music](https://wiki.gnome.org/Design/Apps/Music) and [Vocal](http://vocalproject.net/).

We also copied some elements from [Gnome-news](https://wiki.gnome.org/Design/Apps/Potential/News).

And almost the entirety of the build system is copied from the [Fractal](https://gitlab.gnome.org/danigm/fractal) project.

