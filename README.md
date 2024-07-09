# GNOME Podcasts

### A Podcast application for GNOME.
Listen to your favorite podcasts, right from your desktop.

![Episodes view](./screenshots/home_view.png)
![Shows view](./screenshots/shows_view.png)
![Show widget](./screenshots/show_widget.png)

## Available on Flathub

[![Get it from Flathub!](https://flathub.org/api/badge?svg&locale=en)](https://flathub.org/apps/details/org.gnome.Podcasts)

## Quick start

GNOME Podcasts can be built and run with [GNOME Builder][builder] >= 41.
You can get Builder from [here][get_builder].

You will also need to install the rust-stable extension from flathub.

```sh
flatpak install --user flathub org.freedesktop.Sdk.Extension.rust-stable//21.08
```

Then from Builder, just clone the repo and hit the run button!

## Broken Feeds

Found a feed that does not work in GNOME Podcasts?
Please [open an issue][new_issue] and choose the `BrokenFeed` template so we will know and fix it!

## Getting in Touch

If you have any questions regarding the use or development of GNOME Podcasts,
want to discuss design or simply hang out, please join us on our [matrix][matrix] channel.

## Building

### Flatpak

Flatpak is the recommended way of building and installing GNOME Podcasts.
Here are the dependencies you will need.

```sh
# Add flathub and the gnome-nightly repo
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak remote-add --user --if-not-exists gnome-nightly https://nightly.gnome.org/gnome-nightly.flatpakrepo

# Install the gnome-nightly Sdk and Platform runtime
flatpak install --user gnome-nightly org.gnome.Sdk org.gnome.Platform

# Install the required rust-stable extension from flathub
flatpak install --user flathub org.freedesktop.Sdk.Extension.rust-stable//20.08
```

To install the resulting flatpak you can do:

```bash
flatpak-builder --user --install --force-clean --repo=repo podcasts org.gnome.Podcasts.Devel.json
```

### Building from source

```sh
git clone https://gitlab.gnome.org/World/podcasts.git
cd podcasts/
meson --prefix=/usr build
ninja -C build
sudo ninja -C build install
```

#### Dependencies

* Rust stable 1.34 or later along with cargo.
* Gtk 4.0.0 or later
* Gstreamer 1.16 or later
* libadwaita 1.0.0 or later
* Meson
* A network connection

Offline build are possible too, but [`cargo-vendor`][vendor] would have to be setup first

## Contributing

There are a lot of things yet to be done.

If you want to contribute, please check the [Contributions Guidelines][contribution-guidelines].

You can start by taking a look at [Issues][issues] or by opening a [New issue][new_issue].

There are also some minor tasks tagged with `TODO:` and `FIXME:` in the source code.

[contribution-guidelines]: https://gitlab.gnome.org/World/podcasts/blob/main/CONTRIBUTING.md

### Translations

Helping to translate Podcasts or adding support to a new language is very
welcome. You can find everything you need at:
[l10n.gnome.org/module/podcasts/](https://l10n.gnome.org/module/podcasts/)


## Overview

```sh
$ tree -d
├── screenshots         # png's used in the README.md
├── podcasts-data        # Storate related stuff, SQLite, XDG setup, RSS Parser.
│   ├── migrations      # Diesel SQL migrations.
│   │   └── ...
│   ├── src
│   └── tests
│       └── feeds       # Raw RSS Feeds used for tests.
├── podcasts-gtk         # The Gtk+ Client
│   ├── resources       # GResources folder
│   │   └── gtk         # Contains the glade.ui files.
│   └── src
│       ├── stacks      # Contains the gtk Stacks that hold all the different views.
│       └── widgets     # Contains custom widgets such as Show and Episode.
```

## A note about the project's name

The project used to be called Hammond, after Allan Moore's character [Evey Hammond][hammond] from the graphic novel V for Vendetta.
It was renamed to GNOME Podcasts on 2018/07/24 shortly before its first public release.

## Acknowledgments

GNOME Podcasts's design is heavily inspired by [GNOME Music][music] and [Vocal][vocal].

We also copied some elements from [GNOME News][news].

And almost the entirety of the build system is copied from the [Fractal][fractal] project.

[vendor]: https://doc.rust-lang.org/cargo/commands/cargo-vendor.html
[matrix]: https://matrix.to/#/#podcasts:gnome.org
[flatpak_setup]: https://flatpak.org/setup/
[music]: https://apps.gnome.org/Music/
[vocal]: http://vocalproject.net/
[news]: https://wiki.gnome.org/Design/Apps/Potential/News
[fractal]: https://gitlab.gnome.org/World/fractal
[hammond]: https://en.wikipedia.org/wiki/Evey_Hammond
[issues]: https://gitlab.gnome.org/World/podcasts/issues
[new_issue]: https://gitlab.gnome.org/World/podcasts/issues/new
[builder]: https://apps.gnome.org/Builder/
[get_builder]: https://flathub.org/apps/org.gnome.Builder
