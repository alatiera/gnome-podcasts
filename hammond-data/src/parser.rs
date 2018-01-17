use ammonia;
use rss::{Channel, Item};

use models::insertables::{NewPodcast, NewPodcastBuilder};
use utils::url_cleaner;
use utils::replace_extra_spaces;

// use errors::*;

/// Parses a `rss::Channel` into a `NewPodcast` Struct.
pub(crate) fn new_podcast(chan: &Channel, source_id: i32) -> NewPodcast {
    let title = chan.title().trim();

    // Prefer itunes summary over rss.description since many feeds put html into rss.description.
    let summary = chan.itunes_ext().map(|s| s.summary()).and_then(|s| s);
    let description = if let Some(sum) = summary {
        replace_extra_spaces(&ammonia::clean(sum))
    } else {
        replace_extra_spaces(&ammonia::clean(chan.description()))
    };

    let link = url_cleaner(chan.link());
    let x = chan.itunes_ext().map(|s| s.image());
    let image_uri = if let Some(img) = x {
        img.map(|s| s.to_owned())
    } else {
        chan.image().map(|foo| foo.url().to_owned())
    };

    NewPodcastBuilder::default()
        .title(title)
        .description(description)
        .link(link)
        .image_uri(image_uri)
        .source_id(source_id)
        .build()
        .unwrap()
}

/// Parses an Item Itunes extension and returns it's duration value in seconds.
// FIXME: Rafactor
// TODO: Write tests
#[allow(non_snake_case)]
pub(crate) fn parse_itunes_duration(item: &Item) -> Option<i32> {
    let duration = item.itunes_ext().map(|s| s.duration())??;

    // FOR SOME FUCKING REASON, IN THE APPLE EXTENSION SPEC
    // THE DURATION CAN BE EITHER AN INT OF SECONDS OR
    // A STRING OF THE FOLLOWING FORMATS:
    // HH:MM:SS, H:MM:SS, MM:SS, M:SS
    // LIKE WHO THE FUCK THOUGH THAT WOULD BE A GOOD IDEA.
    if let Ok(NO_FUCKING_LOGIC) = duration.parse::<i32>() {
        return Some(NO_FUCKING_LOGIC);
    };

    let mut seconds = 0;
    let fk_apple = duration.split(':').collect::<Vec<_>>();
    if fk_apple.len() == 3 {
        seconds += fk_apple[0].parse::<i32>().unwrap_or(0) * 3600;
        seconds += fk_apple[1].parse::<i32>().unwrap_or(0) * 60;
        seconds += fk_apple[2].parse::<i32>().unwrap_or(0);
    } else if fk_apple.len() == 2 {
        seconds += fk_apple[0].parse::<i32>().unwrap_or(0) * 60;
        seconds += fk_apple[1].parse::<i32>().unwrap_or(0);
    }

    Some(seconds)
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::BufReader;
    use rss;
    use models::insertables::{NewEpisode, NewEpisodeBuilder};

    use super::*;

    #[test]
    fn test_itunes_duration() {
        use rss::extension::itunes::ITunesItemExtensionBuilder;

        // Input is a String<Int>
        let extension = ITunesItemExtensionBuilder::default()
            .duration(Some("3370".into()))
            .build()
            .unwrap();
        let item = rss::ItemBuilder::default()
            .itunes_ext(Some(extension))
            .build()
            .unwrap();
        assert_eq!(parse_itunes_duration(&item), Some(3370));

        // Input is a String<M:SS>
        let extension = ITunesItemExtensionBuilder::default()
            .duration(Some("6:10".into()))
            .build()
            .unwrap();
        let item = rss::ItemBuilder::default()
            .itunes_ext(Some(extension))
            .build()
            .unwrap();
        assert_eq!(parse_itunes_duration(&item), Some(370));

        // Input is a String<MM:SS>
        let extension = ITunesItemExtensionBuilder::default()
            .duration(Some("56:10".into()))
            .build()
            .unwrap();
        let item = rss::ItemBuilder::default()
            .itunes_ext(Some(extension))
            .build()
            .unwrap();
        assert_eq!(parse_itunes_duration(&item), Some(3370));

        // Input is a String<H:MM:SS>
        let extension = ITunesItemExtensionBuilder::default()
            .duration(Some("1:56:10".into()))
            .build()
            .unwrap();
        let item = rss::ItemBuilder::default()
            .itunes_ext(Some(extension))
            .build()
            .unwrap();
        assert_eq!(parse_itunes_duration(&item), Some(6970));

        // Input is a String<HH:MM:SS>
        let extension = ITunesItemExtensionBuilder::default()
            .duration(Some("01:56:10".into()))
            .build()
            .unwrap();
        let item = rss::ItemBuilder::default()
            .itunes_ext(Some(extension))
            .build()
            .unwrap();
        assert_eq!(parse_itunes_duration(&item), Some(6970));
    }

    #[test]
    fn test_new_podcast_intercepted() {
        let file = File::open("tests/feeds/Intercepted.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();

        let descr = "The people behind The Intercept’s fearless reporting and incisive \
                     commentary—Jeremy Scahill, Glenn Greenwald, Betsy Reed and others—discuss \
                     the crucial issues of our time: national security, civil liberties, foreign \
                     policy, and criminal justice. Plus interviews with artists, thinkers, and \
                     newsmakers who challenge our preconceptions about the world we live in.";

        let pd = new_podcast(&channel, 0);
        let expected = NewPodcastBuilder::default()
            .title("Intercepted with Jeremy Scahill")
            .link("https://theintercept.com/podcasts")
            .description(descr)
            .image_uri(Some(String::from(
                "http://static.megaphone.fm/podcasts/d5735a50-d904-11e6-8532-73c7de466ea6/image/\
                 uploads_2F1484252190700-qhn5krasklbce3dh-a797539282700ea0298a3a26f7e49b0b_\
                 2FIntercepted_COVER%2B_281_29.png")
            ))
            .build()
            .unwrap();

        assert_eq!(pd, expected);
    }

    #[test]
    fn test_new_podcast_breakthrough() {
        let file = File::open("tests/feeds/TheBreakthrough.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();

        let descr = "The podcast that takes you behind the scenes with journalists to hear how \
                     they nailed their biggest stories.";
        let pd = new_podcast(&channel, 0);

        let expected = NewPodcastBuilder::default()
            .title("The Breakthrough")
            .link("http://www.propublica.org/podcast")
            .description(descr)
            .image_uri(Some(String::from(
                "http://www.propublica.org/images/podcast_logo_2.png",
            )))
            .build()
            .unwrap();

        assert_eq!(pd, expected);
    }

    #[test]
    fn test_new_podcast_lup() {
        let file = File::open("tests/feeds/LinuxUnplugged.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();

        let descr = "An open show powered by community LINUX Unplugged takes the best attributes \
                     of open collaboration and focuses them into a weekly lifestyle show about \
                     Linux.";
        let pd = new_podcast(&channel, 0);

        let expected = NewPodcastBuilder::default()
            .title("LINUX Unplugged Podcast")
            .link("http://www.jupiterbroadcasting.com/")
            .description(descr)
            .image_uri(Some(String::from(
                "http://www.jupiterbroadcasting.com/images/LASUN-Badge1400.jpg",
            )))
            .build()
            .unwrap();

        assert_eq!(pd, expected);
    }

    #[test]
    fn test_new_podcast_r4explanation() {
        let file = File::open("tests/feeds/R4Explanation.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();

        let pd = new_podcast(&channel, 0);

        let expected = NewPodcastBuilder::default()
            .title("Request For Explanation")
            .link("https://request-for-explanation.github.io/podcast/")
            .description("A weekly discussion of Rust RFCs")
            .image_uri(Some(String::from(
                "https://request-for-explanation.github.io/podcast/podcast.png",
            )))
            .build()
            .unwrap();

        assert_eq!(pd, expected);
    }

    #[test]
    fn test_new_episode_intercepted() {
        let file = File::open("tests/feeds/Intercepted.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();

        let firstitem = channel.items().first().unwrap();
        let descr = "NSA whistleblower Edward Snowden discusses the massive Equifax data breach \
                     and allegations of Russian interference in the US election. Commentator \
                     Shaun King explains his call for a boycott of the NFL and talks about his \
                     campaign to bring violent neo-Nazis to justice. Rapper Open Mike Eagle \
                     performs.";

        let ep = NewEpisode::new(&firstitem, 0).unwrap();
        let expected = NewEpisodeBuilder::default()
            .title("The Super Bowl of Racism")
            .uri(Some(String::from(
                "http://traffic.megaphone.fm/PPY6458293736.mp3",
            )))
            .description(Some(String::from(descr)))
            .guid(Some(String::from("7df4070a-9832-11e7-adac-cb37b05d5e24")))
            .length(Some(66738886))
            .epoch(1505296800)
            .duration(Some(4171))
            .build()
            .unwrap();

        assert_eq!(ep, expected);

        let second = channel.items().iter().nth(1).unwrap();
        let ep = NewEpisode::new(&second, 0).unwrap();

        let descr = "This week on Intercepted: Jeremy gives an update on the aftermath of \
                     Blackwater’s 2007 massacre of Iraqi civilians. Intercept reporter Lee Fang \
                     lays out how a network of libertarian think tanks called the Atlas Network \
                     is insidiously shaping political infrastructure in Latin America. We speak \
                     with attorney and former Hugo Chavez adviser Eva Golinger about the \
                     Venezuela\'s political turmoil.And we hear Claudia Lizardo of the \
                     Caracas-based band, La Pequeña Revancha, talk about her music and hopes for \
                     Venezuela.";

        let expected = NewEpisodeBuilder::default()
            .title("Atlas Golfed — U.S.-Backed Think Tanks Target Latin America")
            .uri(Some(String::from(
                "http://traffic.megaphone.fm/FL5331443769.mp3",
            )))
            .description(Some(String::from(descr)))
            .guid(Some(String::from("7c207a24-e33f-11e6-9438-eb45dcf36a1d")))
            .length(Some(67527575))
            .epoch(1502272800)
            .duration(Some(4220))
            .build()
            .unwrap();

        assert_eq!(ep, expected);
    }

    #[test]
    fn test_new_episode_breakthrough() {
        let file = File::open("tests/feeds/TheBreakthrough.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();

        let firstitem = channel.items().first().unwrap();
        let descr =
            "A reporter finds that homes meant to replace New York’s troubled psychiatric \
             hospitals might be just as bad.";
        let ep = NewEpisode::new(&firstitem, 0).unwrap();

        let expected = NewEpisodeBuilder::default()
            .title("The Breakthrough: Hopelessness and Exploitation Inside Homes for Mentally Ill")
            .uri(Some(String::from("http://tracking.feedpress.it/link/10581/6726758/20170908-cliff-levy.mp3")))
            .description(Some(String::from(descr)))
            .guid(Some(String::from("https://www.propublica.org/podcast/\
                 the-breakthrough-hopelessness-exploitation-homes-for-mentally-ill#134472")))
            .length(Some(33396551))
            .epoch(1504872000)
            .duration(Some(1670))
            .build()
            .unwrap();

        assert_eq!(ep, expected);

        let second = channel.items().iter().nth(1).unwrap();
        let ep = NewEpisode::new(&second, 0).unwrap();
        let descr =
            "Jonathan Allen and Amie Parnes didn’t know their book would be called \
             ‘Shattered,’ or that their extraordinary access would let them chronicle the \
             mounting signs of a doomed campaign.";

        let expected =
            NewEpisodeBuilder::default()
                .title(
                    "The Breakthrough: Behind the Scenes of Hillary Clinton’s Failed Bid for \
                     President",
                )
                .uri(Some(String::from(
                    "http://tracking.feedpress.it/link/10581/6726759/16_JohnAllen-CRAFT.mp3",
                )))
                .description(Some(String::from(descr)))
                .guid(Some(String::from(
                    "https://www.propublica.\
                     org/podcast/the-breakthrough-hillary-clinton-failed-presidential-bid#133721",
                )))
                .length(Some(17964071))
                .epoch(1503662400)
                .duration(Some(1125))
                .build()
                .unwrap();

        assert_eq!(ep, expected);
    }

    #[test]
    fn test_new_episode_lup() {
        let file = File::open("tests/feeds/LinuxUnplugged.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();

        let firstitem = channel.items().first().unwrap();
        let descr = "Audit your network with a couple of easy commands on Kali Linux. Chris \
                     decides to blow off a little steam by attacking his IoT devices, Wes has the \
                     scope on Equifax blaming open source &amp; the Beard just saved the show. \
                     It’s a really packed episode!";
        let ep = NewEpisode::new(&firstitem, 0).unwrap();

        let expected = NewEpisodeBuilder::default()
            .title("Hacking Devices with Kali Linux | LUP 214")
            .uri(Some(String::from(
                "http://www.podtrac.com/pts/redirect.mp3/traffic.libsyn.com/jnite/lup-0214.mp3",
            )))
            .description(Some(String::from(descr)))
            .guid(Some(String::from("78A682B4-73E8-47B8-88C0-1BE62DD4EF9D")))
            .length(Some(46479789))
            .epoch(1505280282)
            .duration(Some(5733))
            .build()
            .unwrap();

        assert_eq!(ep, expected);

        let second = channel.items().iter().nth(1).unwrap();
        let ep = NewEpisode::new(&second, 0).unwrap();

        let descr =
            "The Gnome project is about to solve one of our audience's biggest Wayland’s \
             concerns. But as the project takes on a new level of relevance, decisions for the \
             next version of Gnome have us worried about the future.\nPlus we chat with Wimpy \
             about the Ubuntu Rally in NYC, Microsoft’s sneaky move to turn Windows 10 into the \
             “ULTIMATE LINUX RUNTIME”, community news &amp; more!";

        let expected = NewEpisodeBuilder::default()
            .title("Gnome Does it Again | LUP 213")
            .uri(Some(String::from(
                "http://www.podtrac.com/pts/redirect.mp3/traffic.libsyn.com/jnite/lup-0213.mp3",
            )))
            .description(Some(String::from(descr)))
            .guid(Some(String::from("1CE57548-B36C-4F14-832A-5D5E0A24E35B")))
            .length(Some(36544272))
            .epoch(1504670247)
            .duration(Some(4491))
            .build()
            .unwrap();

        assert_eq!(ep, expected);
    }

    #[test]
    fn test_new_episode_r4expanation() {
        let file = File::open("tests/feeds/R4Explanation.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();

        let firstitem = channel.items().iter().nth(9).unwrap();
        let descr = "This week we look at <a href=\"https://github.com/rust-lang/rfcs/pull/2094\" \
                     rel=\"noopener noreferrer\">RFC 2094</a> \"Non-lexical lifetimes\"";
        let ep = NewEpisode::new(&firstitem, 0).unwrap();

        let expected = NewEpisodeBuilder::default()
            .title("Episode #9 - A Once in a Lifetime RFC")
            .uri(Some(String::from(
                "http://request-for-explanation.github.\
                 io/podcast/ep9-a-once-in-a-lifetime-rfc/episode.mp3",
            )))
            .description(Some(String::from(descr)))
            .guid(Some(String::from(
                "https://request-for-explanation.github.io/podcast/ep9-a-once-in-a-lifetime-rfc/",
            )))
            .length(Some(15077388))
            .epoch(1503957600)
            .duration(Some(2533))
            .build()
            .unwrap();

        assert_eq!(ep, expected);

        let second = channel.items().iter().nth(8).unwrap();
        let ep = NewEpisode::new(&second, 0).unwrap();

        let descr = "This week we look at <a href=\"https://github.com/rust-lang/rfcs/pull/2071\" \
                     rel=\"noopener noreferrer\">RFC 2071</a> \"Add impl Trait type alias and \
                     variable declarations\"";

        let expected = NewEpisodeBuilder::default()
            .title("Episode #8 - An Existential Crisis")
            .uri(Some(String::from(
                "http://request-for-explanation.github.\
                 io/podcast/ep8-an-existential-crisis/episode.mp3",
            )))
            .description(Some(String::from(descr)))
            .guid(Some(String::from(
                "https://request-for-explanation.github.io/podcast/ep8-an-existential-crisis/",
            )))
            .length(Some(13713219))
            .epoch(1502841600)
            .duration(Some(2313))
            .build()
            .unwrap();

        assert_eq!(ep, expected);
    }
}
