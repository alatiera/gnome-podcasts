# Hammond

## A Podcast Client for the GNOME Desktop written in Rust.

[![pipeline status](https://gitlab.gnome.org/World/hammond/badges/master/pipeline.svg)](https://gitlab.gnome.org/World/hammond/commits/master)
[![Dependency Status](https://dependencyci.com/github/World/hammond/badge)](https://dependencyci.com/github/World/hammond)

### Features

* TBA

![episdes_view](./screenshots/episodes_view.png)
![shows_view](./screenshots/shows_view.png)
![show_widget](./screenshots/show_widget.png)

## Quick start

Hammond can be built and run with [Gnome Builder](https://wiki.gnome.org/Apps/Builder) >= 3.28.

Get Builder [here](https://wiki.gnome.org/Apps/Builder/Downloads)

## Broken Feeds

Found a feed that does not work in Hammond?
Please [open an issue](https://gitlab.gnome.org/World/hammond/issues/new) and choose the `BrokenFeed` template so we will know and fix it!

## Getting in Touch

If you have any questions regarding the use or development of Hammond,
want to discuss design or simply hang out, please join us in [#hammond on irc.gnome.org.](irc://irc.gnome.org/#hammond)

Note:

There isn't much documentation yet, so you will probably have question about parts of the Code.

## Building

### Flatpak

Flatpak is the reccomended way of building and installing Hammond.

#### Building a Flatpak

Download the `org.gnome.Hammond.json` flatpak manifest from this repo.

```bash
# Add flathub repo
flatpak --user remote-add flathub --if-not-exists https://dl.flathub.org/repo/flathub.flatpakrepo
# Add the gnome-nightly repo
flatpak --user remote-add gnome-nightly --if-not-exists https://sdk.gnome.org/gnome-nightly.flatpakrepo
# Install the gnome-nightly Sdk and Platform runtim
flatpak --user install gnome-nightly org.gnome.Sdk org.gnome.Platform
# Install the required rust-stable extension from flathub
flatpak --user install flathub org.freedesktop.Sdk.Extension.rust-stable
flatpak-builder --user --repo=repo hammond org.gnome.Hammond.json --force-clean
```

To install the resulting flatpak you can do:

```bash
flatpak build-bundle repo hammond.flatpak org.gnome.Hammond
flatpak install --user --bundle hammond.flatpak
```

### Building from soure

```sh
git clone https://gitlab.gnome.org/World/hammond.git
cd hammond/
meson --prefix=/usr build
ninja -C build
sudo ninja -C build install
```

#### Dependencies

* Rust stable 1.22 or later along with cargo.
* Gtk+ 3.22 or later
* Meson
* A network connection

Offline build are possible too, but [`cargo-vendor`][vendor] would have to be setup first

**Debian/Ubuntu**

```sh
apt-get update -yqq
apt-get install -yqq --no-install-recommends build-essential
apt-get install -yqq --no-install-recommends rustc cargo libgtk-3-dev meson
```

**Fedora**

```sh
dnf install -y rust cargo gtk3-devel glib2-devel openssl-devel sqlite-devel meson
```

If you happen to build it on other distributions please let me know the names 
of the corresponding libraries. Feel free to open a MR or an Issue to note it.

## Contributing

There alot of thins yet to be done.

If you want to contribute, please check the [Contributions Guidelines][contribution-guidelines].

You can start by taking a look at [Issues](https://gitlab.gnome.org/World/hammond/issues) or by opening a [New issue](https://gitlab.gnome.org/World/hammond/issues/new?issue%5Bassignee_id%5D=&issue%5Bmilestone_id%5D=).

There are also some minor tasks tagged with `TODO:` and `FIXME:` in the source code.

[contribution-guidelines]: https://gitlab.gnome.org/World/hammond/blob/master/CONTRIBUTING.md


## Overview

```sh
$ tree -d
├── screenshots         # png's used in the README.md
├── hammond-data        # Storate related stuff, SQLite, XDG setup, RSS Parser.
│   ├── migrations      # Diesel SQL migrations.
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
│       ├── views       # Contains the Empty, Episodes and Shows view.
│       └── widgets     # Contains custom widgets such as Show and Episode.
```

## A note about the project's name

The project was named after Allan Moore's character [Evey Hammond](https://en.wikipedia.org/wiki/Evey_Hammond) from the graphic novel V for Vendetta.

It has nothing to do with the horrible headlines on the news.

## Acknowledgments

Hammond's design is heavily insired by [GNOME Music](https://wiki.gnome.org/Design/Apps/Music) and [Vocal](http://vocalproject.net/).

We also copied some elements from [GNOME News](https://wiki.gnome.org/Design/Apps/Potential/News).

And almost the entirety of the build system is copied from the [Fractal](https://gitlab.gnome.org/danigm/fractal) project.

[vendor]: https://github.com/alexcrichton/cargo-vendor