## TODOs:

**General:**

- [ ] Add CONTRIBUTING.md
- [ ] Write docs


## Priorities:

**Would be nice:**

- [x] Use GResource for assets like the banner.png etc, instead of hardcoded paths.
- [x] Possibly Convert description labels to textview.
- [ ] Make Podcast cover fetchng and loading not block the execution of the program at startup.
- [ ] Re-design EpisodeWidget.
- [ ] Lazy evaluate episode loading based on the podcast_widget's view scrolling.
- [ ] Headerbar back button and stack switching
- [x] New episode notifier on podcast_flowbox_child, like the one vocal has
- [ ] Polish the flowbox_child banner.
- [x] Update on startup


**Unhack stuff:**

- [ ] Url sanitization
- [x] Fix downloader .ext parsing


**FIXME:**

- [x] Fix Flowbox child activation. [#1](https://gitlab.gnome.org/alatiera/Hammond/issues/1)
- [ ] Fix Etag/Last-modified implementation. [#2](https://gitlab.gnome.org/alatiera/Hammond/issues/2)


**DB changes:**

- [x] episodes: add watched field
- [x] Podcast deletion
- [x] Download cleaner


## Secondary:

- [ ] Discuss and decide when to schedule the download cleaner. [#3](https://gitlab.gnome.org/alatiera/Hammond/issues/3)
- [ ] Unplayed Only and Downloaded only view.
- [ ] Auto-updater
- [ ] Make use of file metadas, [This](https://github.com/GuillaumeGomez/audio-video-metadata) might be helpfull.
- [ ] OPML import/export // Probably need to create a crate.

**DB changes:**

- [ ] Mark episodes/podcast for archival
- [ ] Mark stuff as Favorite. Maybe auto-archive favorites?


## Third:

- [ ] Notifications
- [ ] Episode queue
- [ ] Embedded player
- [ ] MPRIS integration
- [ ] Search Implementation


## Fourth:

- [ ] soundcloud and itunes feeds // [This](http://getrssfeed.com) seems intresting.
- [ ] Integrate with Itunes API for various crap
- [ ] YoutubeFeeds