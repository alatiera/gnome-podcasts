use rss::{Channel, Item};
use chrono::DateTime;
use models;
use errors::*;

pub fn parse_podcast(chan: &Channel, source_id: i32) -> Result<models::NewPodcast> {

    let title = chan.title().to_owned();
    let link = chan.link().to_owned();

    let description = chan.description().to_owned();

    // let image_uri = match chan.image() {
    //     Some(foo) => Some(foo.url().to_owned()),
    //     None => None,
    // };
    // Same as the above match expression.
    let image_uri = chan.image().map(|foo| foo.url().to_owned());

    let foo = models::NewPodcast {
        title,
        link,
        description,
        image_uri,
        source_id,
    };
    Ok(foo)
}

pub fn parse_episode<'a>(item: &'a Item, parent_id: i32) -> Result<models::NewEpisode<'a>> {

    let title = item.title();

    let description = item.description();
    let guid = item.guid().map(|x| x.value());

    let mut uri = item.enclosure().map(|x| x.url());

    if uri == None {
        uri = item.link();
    }

    // FIXME:
    // probably needs to be removed from NewEpisode,
    // and have seperate logic to handle local_files
    let local_uri = None;

    let pub_date = item.pub_date();

    let epoch = match pub_date{
        Some(foo) => {
            // info!("{}", foo);
            let date = DateTime::parse_from_rfc2822(foo);
            match date {
                Ok(bar) => bar.timestamp() as i32,
                Err(baz) => {
                    error!("Error while trying to parse \"{}\" as date.", foo);
                    error!("{}", baz);
                    debug!("Falling back to default 0");
                    0
                }
            }
        }
        _ => 0
    };

    let length = item.enclosure().map(|x| x.length().parse().unwrap_or_default());

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
    fn test_parse_podcast_intercepted() {
        let file = File::open("tests/feeds/Intercepted.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();

        let descr = "The people behind The Intercept’s fearless reporting and incisive commentary—Jeremy Scahill, Glenn Greenwald, Betsy Reed and others—discuss the crucial issues of our time: national security, civil liberties, foreign policy, and criminal justice.  Plus interviews with artists, thinkers, and newsmakers who challenge our preconceptions about the world we live in.";
        let pd = parse_podcast(&channel, 0).unwrap();

        assert_eq!(pd.title, "Intercepted with Jeremy Scahill".to_string());
        assert_eq!(pd.link, "https://theintercept.com/podcasts".to_string());
        assert_eq!(pd.description, descr.to_string());
        assert_eq!(pd.image_uri, None);
    }

    #[test]
    fn test_parse_podcast_breakthrough() {
        let file = File::open("tests/feeds/TheBreakthrough.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();

        let descr = "Latest Articles and Investigations from ProPublica, an independent, non-profit newsroom that produces investigative journalism in the public interest.";
        let pd = parse_podcast(&channel, 0).unwrap();

        assert_eq!(
            pd.title,
            "Articles and Investigations - ProPublica".to_string()
        );
        assert_eq!(
            pd.link,
            "https://www.propublica.org/feeds/54Ghome".to_string()
        );
        assert_eq!(pd.description, descr.to_string());
        assert_eq!(
            pd.image_uri,
            Some(
                "https://assets.propublica.org/propublica-rss-logo.png".to_string(),
            )
        );
    }

    #[test]
    fn test_parse_podcast_lup() {
        let file = File::open("tests/feeds/LinuxUnplugged.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();

        let descr = "An open show powered by community LINUX Unplugged takes the best attributes of open collaboration and focuses them into a weekly lifestyle show about Linux.";
        let pd = parse_podcast(&channel, 0).unwrap();

        assert_eq!(pd.title, "LINUX Unplugged Podcast".to_string());
        assert_eq!(pd.link, "http://www.jupiterbroadcasting.com/".to_string());
        assert_eq!(pd.description, descr.to_string());
        assert_eq!(
            pd.image_uri,
            Some(
                "http://michaeltunnell.com/images/linux-unplugged.jpg".to_string(),
            )
        );
    }

    #[test]
    fn test_parse_podcast_r4explanation() {
        let file = File::open("tests/feeds/R4Explanation.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();

        let pd = parse_podcast(&channel, 0).unwrap();
        let descr = "A weekly discussion of Rust RFCs";

        assert_eq!(pd.title, "Request For Explanation".to_string());
        assert_eq!(pd.link, "https://request-for-explanation.github.io/podcast/".to_string());
        assert_eq!(pd.description, descr.to_string());
        assert_eq!(
            pd.image_uri,
            Some(
                "https://request-for-explanation.github.io/podcast/podcast.png".to_string(),
            )
        );
    }

    #[test]
    fn test_parse_episode_intercepted() {
        let file = File::open("tests/feeds/Intercepted.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();

        let firstitem = channel.items().first().unwrap();
        let descr = "NSA whistleblower Edward Snowden discusses the massive Equifax data breach and allegations of Russian interference in the US election. Commentator Shaun King explains his call for a boycott of the NFL and talks about his campaign to bring violent neo-Nazis to justice. Rapper Open Mike Eagle performs.";
        let i = parse_episode(&firstitem, 0).unwrap();

        assert_eq!(i.title, Some("The Super Bowl of Racism"));
        assert_eq!(i.uri, Some("http://traffic.megaphone.fm/PPY6458293736.mp3"));
        assert_eq!(i.description, Some(descr));
        assert_eq!(i.length, Some(66738886));
        assert_eq!(i.guid, Some("7df4070a-9832-11e7-adac-cb37b05d5e24"));
        assert_eq!(i.published_date, Some("Wed, 13 Sep 2017 10:00:00 -0000"));

        let second = channel.items().iter().nth(1).unwrap();
        let i2 = parse_episode(&second, 0).unwrap();

        let descr2 = "This week on Intercepted: Jeremy gives an update on the aftermath of Blackwater’s 2007 massacre of Iraqi civilians. Intercept reporter Lee Fang lays out how a network of libertarian think tanks called the Atlas Network is insidiously shaping political infrastructure in Latin America. We speak with attorney and former Hugo Chavez adviser Eva Golinger about the Venezuela\'s political turmoil.And we hear Claudia Lizardo of the Caracas-based band, La Pequeña Revancha, talk about her music and hopes for Venezuela.";
        assert_eq!(
            i2.title,
            Some(
                "Atlas Golfed — U.S.-Backed Think Tanks Target Latin America",
            )
        );
        assert_eq!(i2.uri, Some("http://traffic.megaphone.fm/FL5331443769.mp3"));
        assert_eq!(i2.description, Some(descr2));
        assert_eq!(i2.length, Some(67527575));
        assert_eq!(i2.guid, Some("7c207a24-e33f-11e6-9438-eb45dcf36a1d"));
        assert_eq!(i2.published_date, Some("Wed, 09 Aug 2017 10:00:00 -0000"));
    }

    #[test]
    fn test_parse_episode_breakthrough() {
        let file = File::open("tests/feeds/TheBreakthrough.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();

        let firstitem = channel.items().first().unwrap();
        let descr = "\n                <p class=\"byline\">\n                by <a class=\"name\" href=\"https://www.propublica.org/people/julia-angwin\">Julia Angwin</a> and <a class=\"name\" href=\"https://www.propublica.org/people/jeff-larson\">Jeff Larson</a>                </p>\n                                <p>California regulators said they have required Nationwide and USAA to adjust their auto insurance rates as a result of <a href=\"https://www.propublica.org/article/minority-neighborhoods-higher-car-insurance-premiums-white-areas-same-risk\">a report by ProPublica and Consumer Reports</a> that many minority neighborhoods were paying more than white areas with the same risk.</p>\n\n<p>The regulators said their review confirmed our finding that linked the pricing disparities to incorrect applications of a provision in California law. The statute allows insurers to cluster neighboring zip codes together into a single rating territory.</p>\n\n<p>“The companies were making some subjective\u{a0}determinations,” as a basis for calculating rates in some zip codes, said Ken Allen, deputy commissioner of the rate regulation branch of the California Department of Insurance. Nationwide and USAA are two of the 10 largest auto insurance providers in the country by market share.</p>\n\n<p>The department said that the adjustments would largely erase the racial disparities we found in the two companies’ pricing. According to our analysis, USAA charged 18 percent more on average, and Nationwide 14 percent more, in poor, minority neighborhoods than in whiter neighborhoods with similarly high accident costs. Allen said it’s not possible to quantify how these adjustments would affect customers’ premiums because the revisions are too complex. In addition, they’re taking effect at the same time as an overall rate increase.</p>\n\n<p>Allen said the department is now requiring more justification from insurers for their measurements of risk in the poor, minority neighborhoods that California designates as “underserved” for auto coverage.</p>\n\n<p>California’s action marks a rare regulatory rebuke of the insurance industry for its longtime practice of charging higher premiums to drivers living in predominantly minority-urban neighborhoods than to drivers with similar safety records living in majority-white neighborhoods. Insurers have traditionally defended their pricing by saying that the risk is greater in those neighborhoods, even for motorists who have never had an accident.\u{a0}</p>\n\n<p>The department’s investigation was prompted by a ProPublica and Consumer Reports <a href=\"https://www.propublica.org/article/minority-neighborhoods-higher-car-insurance-premiums-methodology\">analysis published in April of car insurance premiums</a> in California, Texas, Missouri and Illinois. ProPublica found that some major insurers were charging minority neighborhoods rates as much as 30 percent more than in other areas with similar accident costs.</p>\n\n<p>The disparities were not as widespread in California, which is a highly regulated insurance market, as in the other states. Even so, within California, we found that units of Nationwide, USAA and Liberty Mutual were charging prices in risky minority neighborhoods that were more than 10 percent above similar risky zip codes where more residents were white.</p>\n    \n    <p>California regulators said they approved rate increases from Nationwide and USAA last week that contained corrections to the disparities revealed by ProPublica. The regulators said they are still investigating the proposed rates of Liberty Mutual, which had the largest disparities in ProPublica’s analysis. Liberty Mutual spokesman Glenn Greenberg said the company is cooperating with the investigation.</p>\n\n<p>The rate changes will only affect premiums charged from now on. The insurance commission chose not to look into whether, or the extent to which, drivers in California’s underserved neighborhoods may have been mischarged in the past.</p>\n\n<p>Department spokeswoman Nancy Kincaid said there was no need to examine past rates. “After hundreds of hours of additional analysis, department actuaries and analysts did not find any indication the ProPublica analysis revealed valid legal issues,” she said.</p>\n\n<p>Some consumer advocates disagreed with this approach. “We think the commissioner should go back and seek refunds for people who were covertly overcharged by the discriminatory practices that ProPublica uncovered,” said Harvey Rosenfield, founder of Consumer Watchdog. Consumers Union, the policy and action arm of Consumer Reports, <a href=\"https://www.documentcloud.org/documents/4056435-Final-Consumers-Union-Letter-to-CDI-Sept-2017.html\">has also sent a letter</a> to the department, urging it to examine if any rates were calculated improperly in the past.</p>\n\n<p>The insurance commissions in Missouri, Texas and Illinois did not respond to questions about whether they had taken any actions to address the disparities highlighted in ProPublica’s article. A spokesman for the Illinois Department of Insurance said in a statement that it urges consumers to shop around for the best price on automobile insurance.</p>\n\n<p>ProPublica and Consumer reports analyzed more than 100,000 premiums charged for liability insurance — the combination of bodily injury and property damage that represents the minimum coverage drivers buy in each of the states. To equalize driver-related variables such as age and accident history, we limited our study to one type of customer: a 30-year-old woman with a safe driving record. We then compared those premiums, which were provided by Quadrant Information Services, to the average amounts paid out by insurers for liability claims in each zip code.</p>\n\n<p>When ProPublica published its investigation, the California Department of Insurance criticized the article’s approach and findings, saying that “the study’s flawed methodology results in a flawed conclusion” that some insurers discriminate in rate-setting. Nevertheless, the department subsequently used ProPublica’s methodology as a basis for developing a new way to analyze rate filings. It used its new method to examine the recent Nationwide and USAA rate filings.</p>\n\n<p>In California, when insurers set rates for sparsely populated rural zip codes, which tend to be relatively white, they are allowed to consider risk in contiguous zip codes of their own choosing. In some cases, these clusters led higher risk zip codes to be assigned a lower risk — and therefore, lower premium prices — than the state’s comprehensive analysis of accident costs warranted. The use of contiguous zip codes is also common in Missouri, Texas and Illinois but is less regulated there than in California.</p>\n\n<p>In an interview, deputy insurance commissioner Allen said that Nationwide had made a “procedural error” in its use of the contiguous zip codes provision, and that the regulators required the company to rely more heavily on the state’s risk estimates in those areas.</p>\n\n<p>Nationwide acknowledged that the state required a rate adjustment, but disputed the association with ProPublica’s reporting. “It is inaccurate and misleading for anyone to conclude or imply any connection between Nationwide’s recently approved rating plan and ProPublica’s unsubstantiated findings,” spokesman Eric Hardgrove said. He added that Nationwide is committed to nondiscriminatory rates and “disagrees with any assertion to the contrary.”</p>\n\n<p>On page 2,025 of Nationwide’s most recent California insurance filing, the company disclosed that it provided premium quotes for the “ProPublica risk example” to the California insurance commission.</p>\n\n<p>The improper use of the contiguous zip codes provision was also a factor in the USAA filing, Allen said in an interview. “USAA had failed to apply the updated industry wide factors where they had insufficient data,” he said.</p>\n\n<p>USAA spokesman Roger Wildermuth acknowledged when the company filed its rate plan in August 2016, it did not use California’s most up-to-date risk numbers, which were published eight months earlier in December 2015. The reason, he said, was that the insurer had already “completed months of calculations prior to that update.”</p>\n\n<p>He noted that the department approved that filing, including USAA’s decision to rely on its own data, and has now approved the company’s revised calculations using updated data.</p>\n\n<p>“The department has consistently validated our approach to this rate filing,” he said.</p>\n\n<p>California officials said they will more closely police the clustering algorithms, and their impact on poor and minority neighborhoods, as they review future rate filing applications.</p>\n\n<p>“We will use this analysis going forward,” said Joel Laucher, chief deputy commissioner of the department. “We don’t need to change any rules to do that.”\u{a0}</p>\n                            <img src=\"http://feedpress.me/9499/6854328.gif\" height=\"1\" width=\"1\"/>";
        let i = parse_episode(&firstitem, 0).unwrap();

        assert_eq!(
            i.title,
            Some(
                "California Regulators Require Auto Insurers to Adjust Rates",
            )
        );
        assert_eq!(
            i.uri,
            Some("http://tracking.feedpress.it/link/9499/6854328")
        );
        assert_eq!(i.description, Some(descr));
        assert_eq!(i.length, None);
        assert_eq!(
            i.guid,
            Some(
                "https://www.propublica.org/article/california-regulators-require-auto-insurers-to-adjust-rates#134766",
            )
        );
        assert_eq!(i.published_date, Some("Wed, 20 Sep 2017 19:56:00 +0000"));

        let second = channel.items().iter().nth(1).unwrap();
        // println!("{:#?}", second);
        let i2 = parse_episode(&second, 0).unwrap();

        assert_eq!(
            i2.title,
            Some("Failing Charter Schools Have a Reincarnation Plan")
        );
        assert_eq!(
            i2.uri,
            Some("http://tracking.feedpress.it/link/9499/6841866")
        );
        // Too long
        // assert_eq!(i2.description, Some(descr2));
        assert_eq!(i2.length, None);
        assert_eq!(
            i2.guid,
            Some(
                "https://www.propublica.org/article/failing-charter-schools-have-a-reincarnation-plan#134669",
            )
        );
        assert_eq!(i2.published_date, Some("Tue, 19 Sep 2017 10:00:00 +0000"));
    }

    #[test]
    fn test_parse_episode_lup() {
        let file = File::open("tests/feeds/LinuxUnplugged.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();

        let firstitem = channel.items().first().unwrap();
        let descr = "Audit your network with a couple of easy commands on Kali Linux. Chris decides to blow off a little steam by attacking his IoT devices, Wes has the scope on Equifax blaming open source & the Beard just saved the show. It’s a really packed episode!";
        let i = parse_episode(&firstitem, 0).unwrap();

        assert_eq!(i.title, Some("Hacking Devices with Kali Linux | LUP 214"));
        assert_eq!(
            i.uri,
            Some(
                "http://www.podtrac.com/pts/redirect.mp3/traffic.libsyn.com/jnite/lup-0214.mp3",
            )
        );
        assert_eq!(i.description, Some(descr));
        assert_eq!(i.length, Some(46479789));
        assert_eq!(i.guid, Some("78A682B4-73E8-47B8-88C0-1BE62DD4EF9D"));
        assert_eq!(i.published_date, Some("Tue, 12 Sep 2017 22:24:42 -0700"));

        let second = channel.items().iter().nth(1).unwrap();
        let i2 = parse_episode(&second, 0).unwrap();

        let descr2 = "<p>The Gnome project is about to solve one of our audience's biggest Wayland’s concerns. But as the project takes on a new level of relevance, decisions for the next version of Gnome have us worried about the future.</p>

<p>Plus we chat with Wimpy about the Ubuntu Rally in NYC, Microsoft’s sneaky move to turn Windows 10 into the “ULTIMATE LINUX RUNTIME”, community news & more!</p>";
        assert_eq!(i2.title, Some("Gnome Does it Again | LUP 213"));
        assert_eq!(
            i2.uri,
            Some(
                "http://www.podtrac.com/pts/redirect.mp3/traffic.libsyn.com/jnite/lup-0213.mp3",
            )
        );
        assert_eq!(i2.description, Some(descr2));
        assert_eq!(i2.length, Some(36544272));
        assert_eq!(i2.guid, Some("1CE57548-B36C-4F14-832A-5D5E0A24E35B"));
        assert_eq!(i2.published_date, Some("Tue, 05 Sep 2017 20:57:27 -0700"));
    }

    #[test]
    fn test_parse_episode_r4expanation() {
        let file = File::open("tests/feeds/R4Explanation.xml").unwrap();
        let channel = Channel::read_from(BufReader::new(file)).unwrap();

        let firstitem = channel.items().iter().nth(9).unwrap();
        let descr = "This week we look at <a href=\"https://github.com/rust-lang/rfcs/pull/2094\">RFC 2094</a> \"Non-lexical lifetimes\"";
        let i = parse_episode(&firstitem, 0).unwrap();

        assert_eq!(i.title, Some("Episode #9 - A Once in a Lifetime RFC"));
        assert_eq!(
            i.uri,
            Some(
                "http://request-for-explanation.github.io/podcast/ep9-a-once-in-a-lifetime-rfc/episode.mp3",
            )
        );
        assert_eq!(i.description, Some(descr));
        assert_eq!(i.length, Some(15077388));
        assert_eq!(i.guid, Some("https://request-for-explanation.github.io/podcast/ep9-a-once-in-a-lifetime-rfc/"));
        assert_eq!(i.published_date, Some("Mon, 28 Aug 2017 15:00:00 PDT"));

        let second = channel.items().iter().nth(8).unwrap();
        let i2 = parse_episode(&second, 0).unwrap();

        let descr2 = "This week we look at <a href=\"https://github.com/rust-lang/rfcs/pull/2071\">RFC 2071</a> \"Add impl Trait type alias and variable declarations\"";
        assert_eq!(i2.title, Some("Episode #8 - An Existential Crisis"));
        assert_eq!(
            i2.uri,
            Some(
                "http://request-for-explanation.github.io/podcast/ep8-an-existential-crisis/episode.mp3",
            )
        );
        assert_eq!(i2.description, Some(descr2));
        assert_eq!(i2.length, Some(13713219));
        assert_eq!(i2.guid, Some("https://request-for-explanation.github.io/podcast/ep8-an-existential-crisis/"));
        assert_eq!(i2.published_date, Some("Tue, 15 Aug 2017 17:00:00 PDT"));
    }
}