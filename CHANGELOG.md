# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

* Downlaoding and loading images now is done asynchronously and is not blocking programs execution.
[#7](https://gitlab.gnome.org/alatiera/Hammond/issues/7)

## [0.3.1] - 2018-03-28

* Ability to mark all episodes of a Show as watched.
[#47](https://gitlab.gnome.org/alatiera/Hammond/issues/47)
* Now you are able to subscribe to itunes™ podcasts by using the itunes link of the show.
[#49](https://gitlab.gnome.org/alatiera/Hammond/issues/49)
* EpisdeWidget has been reimplemented as a compile time state machine.
[!18](https://gitlab.gnome.org/alatiera/Hammond/merge_requests/18)
* Content Views no longer scroll horizontally when shrunk bellow their minimum size.
[#35](https://gitlab.gnome.org/alatiera/Hammond/issues/35)
* Double border aroun the main window was fixed. (Rowan Lewis)
[#52](https://gitlab.gnome.org/alatiera/Hammond/issues/52)
* Some requests now use the Tor Browser's user agent. (Rowan Lewis)
[#53](https://gitlab.gnome.org/alatiera/Hammond/issues/53)
* Hammond now remembers the window size and position. (Rowan Lewis)
[#50](https://gitlab.gnome.org/alatiera/Hammond/issues/50)
* Implemnted the initial work for integrating with GSettings and storing preferences. (Rowan Lewis)
[!22](https://gitlab.gnome.org/alatiera/Hammond/merge_requests/22) [!23](https://gitlab.gnome.org/alatiera/Hammond/merge_requests/23)
* Shows without episodes now display an empty message similar to EmptyView.
[#44](https://gitlab.gnome.org/alatiera/Hammond/issues/44)

## [0.3.0] - 2018-02-11

* Tobias Bernard Redesigned the whole Gtk+ client.
* Complete re-write of hammond-data and hammond-gtk modules.
* Error handling for all crates was migrated from error-chain to Failure.
* Hammond-data now uses futures to parse feeds.
* Custom gtk-widgets are now composed structs as opposed to functions returning Gtk widgets.

## [0.2.0] - 2017-11-28

* Database Schema Breaking Changes.
* Added url sanitization. #4.
* Reworked and refactored of the hammond-data API.
* Added some more unit tests
* Documented hammond-data public API.

## [0.1.1] - 2017-11-13

* Added appdata.xml file

## [0.1.0] - 2017-11-13

Initial Release