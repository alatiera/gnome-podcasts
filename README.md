# Hammond
## Prototype of a multithreaded, safe, and reliable Gtk+ Podcast client.
Description...

## Quick start
Flatpak instructions...

## Dependancies:

* Rust stable 1.21 or later.
* Gtk+ 3.22 or later

**Debian/Ubuntu**:
```sh
apt-get update -yqq
apt-get install -yqq --no-install-recommends build-essential
apt-get install -yqq --no-install-recommends libgtk-3-dev
```

**Fedora**:
```sh
dnf install -y gtk3-devel openssl-devel sqlite-devel
```

If you happen to build it on other distributions please let me know the names of the corresponding libraries. Feel free to open a PR or an Issue to note it.

## Building:

```sh
git clone https://gitlab.gnome.org/alatiera/Hammond.git
cd Hammond/
cargo run -p hammond-gtk --release
```

## Overview:
foo

## Contributing:
to be added: CONTRIBUTING.md