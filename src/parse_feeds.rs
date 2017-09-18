use rss::{Channel, Item};
use models;
use errors::*;

pub fn parse_podcast<'a>(pd_chan: &'a Channel, uri: &'a str) -> Result<models::NewPodcast<'a>> {
    let title = pd_chan.title();

    let link = Some(pd_chan.link());
    let description = Some(pd_chan.description());

    let image_uri = match pd_chan.image() {
        Some(foo) => Some(foo.url()),
        None => None,
    };

    let foo = models::NewPodcast {
        title,
        uri,
        link,
        description,
        image_uri,
    };
    Ok(foo)
}

pub fn parse_episode<'a>(item: &'a Item, parent_id: i32) -> Result<models::NewEpisode<'a>> {

    let title = item.title().unwrap();

    let description = item.description();
    let guid = Some(item.guid().unwrap().value());

    let uri = item.enclosure().unwrap().url();

    // FIXME:
    // probably needs to be removed from NewEpisode,
    // and have seperate logic to handle local_files
    let local_uri = None;

    let pub_date = item.pub_date().unwrap();

    // FIXME: parse pub_date to epoch later
    let epoch = 0;

    let length = Some(item.enclosure().unwrap().length().parse().unwrap());

    let foo = models::NewEpisode {
        title,
        uri,
        local_uri,
        description,
        length,
        published_date: pub_date,
        epoch,
        guid,
        podcast_id: parent_id,
    };
    Ok(foo)
}


#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::BufReader;
    use rss::Channel;

    use super::*;

    #[test]
    fn test_parse_podcast() {
        // Intercepted feed
        let file = File::open("tests/feeds/Intercepted.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();
        let uri = "https://feeds.feedburner.com/InterceptedWithJeremyScahill";

        // println!("{:#?}", channel);
        let descr = "The people behind The Intercept’s fearless reporting and incisive commentary—Jeremy Scahill, Glenn Greenwald, Betsy Reed and others—discuss the crucial issues of our time: national security, civil liberties, foreign policy, and criminal justice.  Plus interviews with artists, thinkers, and newsmakers who challenge our preconceptions about the world we live in.";
        let pd = parse_podcast(&channel, uri).unwrap();

        assert_eq!(pd.title, "Intercepted with Jeremy Scahill");
        // assert_eq!(
        //     pd.uri,
        //     "https://feeds.feedburner.com/InterceptedWithJeremyScahill"
        // );
        assert_eq!(pd.link, Some("https://theintercept.com/podcasts"));
        assert_eq!(pd.description, Some(descr));
        assert_eq!(pd.image_uri, None);


        // Linux Unplugged Feed
        let file = File::open("tests/feeds/LinuxUnplugged.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();
        let uri = "http://feeds.feedburner.com/linuxunplugged";

        // println!("{:#?}", channel);
        let descr = "An open show powered by community LINUX Unplugged takes the best attributes of open collaboration and focuses them into a weekly lifestyle show about Linux.";
        let pd = parse_podcast(&channel, uri).unwrap();

        assert_eq!(pd.title, "LINUX Unplugged Podcast");
        assert_eq!(pd.link, Some("http://www.jupiterbroadcasting.com/"));
        assert_eq!(pd.description, Some(descr));
        assert_eq!(
            pd.image_uri,
            Some("http://michaeltunnell.com/images/linux-unplugged.jpg")
        );


        // The Breakthrough Feed
        let file = File::open("tests/feeds/TheBreakthrough.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();
        let uri = "http://feeds.propublica.org/propublica/main";

        // println!("{:#?}", channel);
        let descr = "Latest Articles and Investigations from ProPublica, an independent, non-profit newsroom that produces investigative journalism in the public interest.";
        let pd = parse_podcast(&channel, uri).unwrap();

        assert_eq!(pd.title, "Articles and Investigations - ProPublica");
        assert_eq!(pd.link, Some("https://www.propublica.org/feeds/54Ghome"));
        assert_eq!(pd.description, Some(descr));
        assert_eq!(
            pd.image_uri,
            Some("https://assets.propublica.org/propublica-rss-logo.png")
        );
    }

    #[test]
    fn test_parse_episode() {
        let file = File::open("tests/feeds/Intercepted.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();
        let firstitem = channel.items().first().unwrap();
        let descr = "NSA whistleblower Edward Snowden discusses the massive Equifax data breach and allegations of Russian interference in the US election. Commentator Shaun King explains his call for a boycott of the NFL and talks about his campaign to bring violent neo-Nazis to justice. Rapper Open Mike Eagle performs.";

        // println!("{:#?}", firstitem);
        let it = parse_episode(&firstitem, 0).unwrap();

        assert_eq!(it.title, "The Super Bowl of Racism");
        assert_eq!(it.uri, "http://traffic.megaphone.fm/PPY6458293736.mp3");
        assert_eq!(it.description, Some(descr));
        assert_eq!(it.length, Some(66738886));
        assert_eq!(it.guid, Some("7df4070a-9832-11e7-adac-cb37b05d5e24"));
        assert_eq!(it.published_date, "Wed, 13 Sep 2017 10:00:00 -0000");

        let second = channel.items().iter().nth(1).unwrap();
        // println!("{:#?}", second);
        let i2 = parse_episode(&second, 0).unwrap();

        let descr = "This week on Intercepted: Jeremy gives an update on the aftermath of Blackwater’s 2007 massacre of Iraqi civilians. Intercept reporter Lee Fang lays out how a network of libertarian think tanks called the Atlas Network is insidiously shaping political infrastructure in Latin America. We speak with attorney and former Hugo Chavez adviser Eva Golinger about the Venezuela\'s political turmoil.And we hear Claudia Lizardo of the Caracas-based band, La Pequeña Revancha, talk about her music and hopes for Venezuela.";
        assert_eq!(
            i2.title,
            "Atlas Golfed — U.S.-Backed Think Tanks Target Latin America"
        );
        assert_eq!(i2.uri, "http://traffic.megaphone.fm/FL5331443769.mp3");
        assert_eq!(i2.description, Some(descr));
        assert_eq!(i2.length, Some(67527575));
        assert_eq!(i2.guid, Some("7c207a24-e33f-11e6-9438-eb45dcf36a1d"));
        assert_eq!(i2.published_date, "Wed, 09 Aug 2017 10:00:00 -0000");

    }
}