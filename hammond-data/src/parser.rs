use rss::{Channel, Item};
use rfc822_sanitizer::parse_from_rfc2822_with_fallback;

use models::{NewEpisode, NewPodcast};
use utils::url_cleaner;

use errors::*;

// TODO: Extend the support for parsing itunes extensions
/// Parses a `rss::Channel` into a `NewPodcast` Struct.
pub fn new_podcast(chan: &Channel, source_id: i32) -> NewPodcast {
    let title = chan.title().trim().to_owned();
    let description = chan.description().trim().to_owned();

    let link = url_cleaner(chan.link()).to_owned();
    let x = chan.itunes_ext().map(|s| s.image());
    let image_uri = if let Some(img) = x {
        img.map(|s| url_cleaner(s))
    } else {
        chan.image().map(|foo| url_cleaner(foo.url()))
    };

    NewPodcast {
        title,
        link,
        description,
        image_uri,
        source_id,
    }
}

/// Parses an `rss::Item` into a `NewEpisode` Struct.
pub fn new_episode(item: &Item, parent_id: i32) -> Result<NewEpisode> {
    let title = item.title().map(|s| s.trim().to_owned());
    let description = item.description().map(|s| s.trim().to_owned());
    let guid = item.guid().map(|s| s.value().trim().to_owned());

    // Its kinda weird this being an Option type.
    // Rss 2.0 specified that it's optional.
    // Though the db scema has a requirment of episode uri being Unique && Not Null.
    // TODO: Restructure
    let x = item.enclosure().map(|s| url_cleaner(s.url()));
    let uri = if x.is_some() {
        x.unwrap()
    } else if item.link().is_some() {
        item.link().map(|s| url_cleaner(s)).unwrap()
    } else {
        bail!("No url specified for the item.")
    };

    let date = parse_from_rfc2822_with_fallback(
        // Default to rfc2822 represantation of epoch 0.
        item.pub_date().unwrap_or("Thu, 1 Jan 1970 00:00:00 +0000"),
    );

    // Should treat information from the rss feeds as invalid by default.
    // Case: Thu, 05 Aug 2016 06:00:00 -0400 <-- Actually that was friday.
    let pub_date = date.map(|x| x.to_rfc2822()).ok();
    let epoch = date.map(|x| x.timestamp() as i32).unwrap_or(0);

    let length = item.enclosure().map(|x| x.length().parse().unwrap_or(0));

    Ok(NewEpisode {
        title,
        uri,
        description,
        length,
        published_date: pub_date,
        epoch,
        guid,
        podcast_id: parent_id,
    })
}


#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::BufReader;
    use rss::Channel;

    use super::*;

    #[test]
    fn test_new_podcast_intercepted() {
        let file = File::open("tests/feeds/Intercepted.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();

        let descr = "The people behind The Intercept’s fearless reporting and incisive \
                     commentary—Jeremy Scahill, Glenn Greenwald, Betsy Reed and others—discuss \
                     the crucial issues of our time: national security, civil liberties, foreign \
                     policy, and criminal justice.  Plus interviews with artists, thinkers, and \
                     newsmakers who challenge our preconceptions about the world we live in.";
        let pd = new_podcast(&channel, 0);

        assert_eq!(pd.title, "Intercepted with Jeremy Scahill".to_string());
        assert_eq!(pd.link, "https://theintercept.com/podcasts".to_string());
        assert_eq!(pd.description, descr.to_string());
        assert_eq!(
            pd.image_uri,
            Some(
                "http://static.megaphone.fm/podcasts/d5735a50-d904-11e6-8532-73c7de466ea6/image/\
                 uploads_2F1484252190700-qhn5krasklbce3dh-a797539282700ea0298a3a26f7e49b0b_\
                 2FIntercepted_COVER%2B_281_29.png"
                    .to_string()
            ),
        );
    }

    #[test]
    fn test_new_podcast_breakthrough() {
        let file = File::open("tests/feeds/TheBreakthrough.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();

        let descr = "Latest Articles and Investigations from ProPublica, an independent, \
                     non-profit newsroom that produces investigative journalism in the public \
                     interest.";
        let pd = new_podcast(&channel, 0);

        assert_eq!(pd.title, "The Breakthrough".to_string());
        assert_eq!(pd.link, "http://www.propublica.org/podcast".to_string());
        assert_eq!(pd.description, descr.to_string());
        assert_eq!(
            pd.image_uri,
            Some("http://www.propublica.org/images/podcast_logo_2.png".to_string()),
        );
    }

    #[test]
    fn test_new_podcast_lup() {
        let file = File::open("tests/feeds/LinuxUnplugged.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();

        let descr = "An open show powered by community LINUX Unplugged takes the best attributes \
                     of open collaboration and focuses them into a weekly lifestyle show about \
                     Linux.";
        let pd = new_podcast(&channel, 0);

        assert_eq!(pd.title, "LINUX Unplugged Podcast".to_string());
        assert_eq!(pd.link, "http://www.jupiterbroadcasting.com/".to_string());
        assert_eq!(pd.description, descr.to_string());
        assert_eq!(
            pd.image_uri,
            Some("http://www.jupiterbroadcasting.com/images/LASUN-Badge1400.jpg".to_string(),)
        );
    }

    #[test]
    fn test_new_podcast_r4explanation() {
        let file = File::open("tests/feeds/R4Explanation.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();

        let pd = new_podcast(&channel, 0);
        let descr = "A weekly discussion of Rust RFCs";

        assert_eq!(pd.title, "Request For Explanation".to_string());
        assert_eq!(
            pd.link,
            "https://request-for-explanation.github.io/podcast/".to_string()
        );
        assert_eq!(pd.description, descr.to_string());
        assert_eq!(
            pd.image_uri,
            Some("https://request-for-explanation.github.io/podcast/podcast.png".to_string()),
        );
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
        let i = new_episode(&firstitem, 0).unwrap();

        assert_eq!(i.title, Some("The Super Bowl of Racism".to_string()));
        assert_eq!(
            i.uri,
            "http://traffic.megaphone.fm/PPY6458293736.mp3".to_string()
        );
        assert_eq!(i.description, Some(descr.to_string()));
        assert_eq!(i.length, Some(66738886));
        assert_eq!(
            i.guid,
            Some("7df4070a-9832-11e7-adac-cb37b05d5e24".to_string())
        );
        assert_eq!(
            i.published_date,
            Some("Wed, 13 Sep 2017 10:00:00 +0000".to_string())
        );
        assert_eq!(i.epoch, 1505296800);

        let second = channel.items().iter().nth(1).unwrap();
        let i2 = new_episode(&second, 0).unwrap();

        let descr2 = "This week on Intercepted: Jeremy gives an update on the aftermath of \
                      Blackwater’s 2007 massacre of Iraqi civilians. Intercept reporter Lee Fang \
                      lays out how a network of libertarian think tanks called the Atlas Network \
                      is insidiously shaping political infrastructure in Latin America. We speak \
                      with attorney and former Hugo Chavez adviser Eva Golinger about the \
                      Venezuela\'s political turmoil.And we hear Claudia Lizardo of the \
                      Caracas-based band, La Pequeña Revancha, talk about her music and hopes for \
                      Venezuela.";
        assert_eq!(
            i2.title,
            Some("Atlas Golfed — U.S.-Backed Think Tanks Target Latin America".to_string(),)
        );
        assert_eq!(
            i2.uri,
            "http://traffic.megaphone.fm/FL5331443769.mp3".to_string()
        );
        assert_eq!(i2.description, Some(descr2.to_string()));
        assert_eq!(i2.length, Some(67527575));
        assert_eq!(
            i2.guid,
            Some("7c207a24-e33f-11e6-9438-eb45dcf36a1d".to_string())
        );
        assert_eq!(
            i2.published_date,
            Some("Wed,  9 Aug 2017 10:00:00 +0000".to_string())
        );
        assert_eq!(i2.epoch, 1502272800);
    }

    #[test]
    fn test_new_episode_breakthrough() {
        let file = File::open("tests/feeds/TheBreakthrough.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();

        let firstitem = channel.items().first().unwrap();
        let descr = "<p>A reporter finds that homes meant to replace New York’s troubled \
                     psychiatric hospitals might be just as bad.</p>";
        let i = new_episode(&firstitem, 0).unwrap();

        assert_eq!(
            i.title,
            Some(
                "The Breakthrough: Hopelessness and Exploitation Inside Homes for Mentally Ill"
                    .to_string(),
            )
        );
        assert_eq!(
            i.uri,
            "http://tracking.feedpress.it/link/10581/6726758/20170908-cliff-levy.mp3".to_string(),
        );
        assert_eq!(i.description, Some(descr.to_string()));
        assert_eq!(i.length, Some(33396551));
        assert_eq!(
            i.guid,
            Some(
                "https://www.propublica.org/podcast/\
                 the-breakthrough-hopelessness-exploitation-homes-for-mentally-ill#134472"
                    .to_string(),
            )
        );
        assert_eq!(
            i.published_date,
            Some("Fri,  8 Sep 2017 12:00:00 +0000".to_string())
        );
        assert_eq!(i.epoch, 1504872000);

        let second = channel.items().iter().nth(1).unwrap();
        let i2 = new_episode(&second, 0).unwrap();
        let descr2 = "<p>Jonathan Allen and Amie Parnes didn’t know their book would be called \
                      ‘Shattered,’ or that their extraordinary access would let them chronicle \
                      the mounting signs of a doomed campaign.</p>";

        assert_eq!(
            i2.title,
            Some("The Breakthrough: Behind the Scenes of Hillary Clinton’s Failed Bid for \
                  President".to_string())
        );
        assert_eq!(
            i2.uri,
            "http://tracking.feedpress.it/link/10581/6726759/16_JohnAllen-CRAFT.mp3".to_string(),
        );
        assert_eq!(i2.description, Some(descr2.to_string()));
        assert_eq!(i2.length, Some(17964071));
        assert_eq!(
            i2.guid,
            Some(
                "https://www.propublica.\
                 org/podcast/the-breakthrough-hillary-clinton-failed-presidential-bid#133721"
                    .to_string(),
            )
        );
        assert_eq!(
            i2.published_date,
            Some("Fri, 25 Aug 2017 12:00:00 +0000".to_string())
        );
        assert_eq!(i2.epoch, 1503662400);
    }

    #[test]
    fn test_new_episode_lup() {
        let file = File::open("tests/feeds/LinuxUnplugged.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();

        let firstitem = channel.items().first().unwrap();
        let descr =
            "Audit your network with a couple of easy commands on Kali Linux. Chris decides to \
             blow off a little steam by attacking his IoT devices, Wes has the scope on Equifax \
             blaming open source & the Beard just saved the show. It’s a really packed episode!";
        let i = new_episode(&firstitem, 0).unwrap();

        assert_eq!(
            i.title,
            Some("Hacking Devices with Kali Linux | LUP 214".to_string())
        );
        assert_eq!(
            i.uri,
            "http://www.podtrac.com/pts/redirect.mp3/traffic.libsyn.com/jnite/lup-0214.mp3"
                .to_string(),
        );
        assert_eq!(i.description, Some(descr.to_string()));
        assert_eq!(i.length, Some(46479789));
        assert_eq!(
            i.guid,
            Some("78A682B4-73E8-47B8-88C0-1BE62DD4EF9D".to_string())
        );
        assert_eq!(
            i.published_date,
            Some("Tue, 12 Sep 2017 22:24:42 -0700".to_string())
        );
        assert_eq!(i.epoch, 1505280282);

        let second = channel.items().iter().nth(1).unwrap();
        let i2 = new_episode(&second, 0).unwrap();

        let descr2 = "<p>The Gnome project is about to solve one of our audience's biggest \
                      Wayland’s concerns. But as the project takes on a new level of relevance, \
                      decisions for the next version of Gnome have us worried about the \
                      future.</p>\n\n<p>Plus we chat with Wimpy about the Ubuntu Rally in NYC, \
                      Microsoft’s sneaky move to turn Windows 10 into the “ULTIMATE LINUX \
                      RUNTIME”, community news & more!</p>";
        assert_eq!(i2.title, Some("Gnome Does it Again | LUP 213".to_string()));
        assert_eq!(
            i2.uri,
            "http://www.podtrac.com/pts/redirect.mp3/traffic.libsyn.com/jnite/lup-0213.mp3"
                .to_string(),
        );
        assert_eq!(i2.description, Some(descr2.to_string()));
        assert_eq!(i2.length, Some(36544272));
        assert_eq!(
            i2.guid,
            Some("1CE57548-B36C-4F14-832A-5D5E0A24E35B".to_string())
        );
        assert_eq!(
            i2.published_date,
            Some("Tue,  5 Sep 2017 20:57:27 -0700".to_string())
        );
        assert_eq!(i2.epoch, 1504670247);
    }

    #[test]
    fn test_new_episode_r4expanation() {
        let file = File::open("tests/feeds/R4Explanation.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();

        let firstitem = channel.items().iter().nth(9).unwrap();
        let descr = "This week we look at <a \
                     href=\"https://github.com/rust-lang/rfcs/pull/2094\">RFC 2094</a> \
                     \"Non-lexical lifetimes\"";
        let i = new_episode(&firstitem, 0).unwrap();

        assert_eq!(
            i.title,
            Some("Episode #9 - A Once in a Lifetime RFC".to_string())
        );
        assert_eq!(
            i.uri,
            "http://request-for-explanation.github.\
             io/podcast/ep9-a-once-in-a-lifetime-rfc/episode.mp3"
                .to_string(),
        );
        assert_eq!(i.description, Some(descr.to_string()));
        assert_eq!(i.length, Some(15077388));
        assert_eq!(
            i.guid,
            Some(
                "https://request-for-explanation.github.io/podcast/ep9-a-once-in-a-lifetime-rfc/"
                    .to_string(),
            )
        );
        assert_eq!(
            i.published_date,
            Some("Mon, 28 Aug 2017 15:00:00 -0700".to_string())
        );
        assert_eq!(i.epoch, 1503957600);

        let second = channel.items().iter().nth(8).unwrap();
        let i2 = new_episode(&second, 0).unwrap();

        let descr2 = "This week we look at <a \
                      href=\"https://github.com/rust-lang/rfcs/pull/2071\">RFC 2071</a> \"Add \
                      impl Trait type alias and variable declarations\"";
        assert_eq!(
            i2.title,
            Some("Episode #8 - An Existential Crisis".to_string())
        );
        assert_eq!(
            i2.uri,
            "http://request-for-explanation.github.io/podcast/ep8-an-existential-crisis/episode.\
             mp3"
                .to_string(),
        );
        assert_eq!(i2.description, Some(descr2.to_string()));
        assert_eq!(i2.length, Some(13713219));
        assert_eq!(
            i2.guid,
            Some(
                "https://request-for-explanation.github.io/podcast/ep8-an-existential-crisis/"
                    .to_string(),
            )
        );
        assert_eq!(
            i2.published_date,
            Some("Tue, 15 Aug 2017 17:00:00 -0700".to_string())
        );
        assert_eq!(i2.epoch, 1502841600);
    }
}
