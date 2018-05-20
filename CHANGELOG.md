# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added:
### Changed:
### Fixed:
### Removed:

## [0.3.4] - 2018-05-20
### Fixed:
- Flatpak can now access the Home folder. This fixes the OPML import feature from
not being able to access any file.

## [0.3.3] - 2018-05-19
### Added:
- Initial functionality for importing shows from an OPML file was implemented.
- ShowsView now rembmers the vertical alignment of the scrollbar between refreshes. 4d2b64e79d8518454b3677612664cd32044cf837

### Changed:
- Minimum `rustc` version requirment was bumped to `1.26`
- Some animations should be smoother now. 7d598bb1d08b05fd5ab532657acdad967c0afbc3
- InAppNotification now can be used to propagate some erros to the user. 7035fe05c4741b3e7ccce6827f72766226d5fc0a and 118dac5a1ab79c0b4ebe78e88256a4a38b138c04

### Fixed:
- Fixed a of by one bug in the `ShowsView` where the last show was never shown. bd12b09cbc8132fd39a266fd091e24bc6c3c040f

## [0.3.2] - 2018-05-07
### Added:
- Vies now have a new fancy scrolling animation when they are refereshed.

### Changed:
- Downlaoding and loading images now is done asynchronously and is not blocking programs execution.
[#7](https://gitlab.gnome.org/World/hammond/issues/7)
- Bold, italics links and some other `html` tags can now be rendered in the Show Description.
[#25](https://gitlab.gnome.org/World/hammond/issues/25)
- `Rayon` Threadpools are now used instead of unlimited one-off threads.
- `EpisdeWidget`s are now loaded asynchronously accross views.
- `EpisodeWidget`s no longer trigger a `View` refresh for trivial stuff 03bd95184808ccab3e0ea0e3713a52ee6b7c9ab4
- `ShowWidget` layout was changed 9a5cc1595d982f3232ee7595b83b6512ac8f6c88
- `ShowWidget` Description is inside a scrolled window now

### Fixed:
- `EpisodeWidget` Height now is consistent accros views [#57](https://gitlab.gnome.org/World/hammond/issues/57)
- Implemented a tail-recursion loop to follow-up when a feed redirects to another url. c6a24e839a8ba77d09673f299cfc1e64ba7078f3

### Removed:
- Removed the custom configuration file and replaced instructions to just use meson. 1f1d4af8ba7db8f56435d13a1c191ecff3d4a85b

## [0.3.1] - 2018-03-28
### Added:
- Ability to mark all episodes of a Show as watched.
[#47](https://gitlab.gnome.org/World/hammond/issues/47)
- Now you are able to subscribe to itunes™ podcasts by using the itunes link of the show.
[#49](https://gitlab.gnome.org/World/hammond/issues/49)
- Hammond now remembers the window size and position. (Rowan Lewis)
[#50](https://gitlab.gnome.org/World/hammond/issues/50)
- Implemnted the initial work for integrating with GSettings and storing preferences. (Rowan Lewis)
[!22](https://gitlab.gnome.org/World/hammond/merge_requests/22) [!23](https://gitlab.gnome.org/World/hammond/merge_requests/23)
- Shows without episodes now display an empty message similar to EmptyView.
[#44](https://gitlab.gnome.org/World/hammond/issues/44)

### Changed:
- EpisdeWidget has been reimplemented as a compile time state machine.
[!18](https://gitlab.gnome.org/World/hammond/merge_requests/18)
- Content Views no longer scroll horizontally when shrunk bellow their minimum size.
[#35](https://gitlab.gnome.org/World/hammond/issues/35)
- Some requests now use the Tor Browser's user agent. (Rowan Lewis)
[#53](https://gitlab.gnome.org/World/hammond/issues/53)

### Fixed:
- Double border aroun the main window was fixed. (Rowan Lewis)
[#52](https://gitlab.gnome.org/World/hammond/issues/52)

## [0.3.0] - 2018-02-11
- Tobias Bernard Redesigned the whole Gtk+ client.
- Complete re-write of hammond-data and hammond-gtk modules.
- Error handling for all crates was migrated from error-chain to Failure.
- Hammond-data now uses futures to parse feeds.
- Custom gtk-widgets are now composed structs as opposed to functions returning Gtk widgets.

## [0.2.0] - 2017-11-28
- Database Schema Breaking Changes.
- Added url sanitization. #4.
- Reworked and refactored of the hammond-data API.
- Added some more unit tests
- Documented hammond-data public API.

## [0.1.1] - 2017-11-13
- Added appdata.xml file

## [0.1.0] - 2017-11-13
- Initial Release