// episode_description.rs
//
// Copyright 2021 nee <nee-git@patchouli.garden>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::default::Default;
use std::string::String;

use html5ever::tendril::TendrilSink;
use html5ever::tree_builder::TreeBuilderOpts;
use html5ever::{expanded_name, parse_document, ParseOpts};
use markup5ever_rcdom::{
    self, Handle,
    NodeData::{Document, Element, Text},
    RcDom,
};

fn escape_amp(t: &str) -> String {
    // TODO prevent escaping escape-sequances
    t.replace("&", "&amp;")
}

fn remove_nl(t: &str) -> String {
    t.replace("\n", "")
}

enum NewlineStyle {
    Text,
    Tag,
}
struct ParserState {
    nl_style: NewlineStyle,
    skip_leading_spaces: bool,
}

fn find_newline_style(node: &Handle) -> NewlineStyle {
    match &node.data {
        Document => {
            let children = node.children.borrow();
            for el in children.iter() {
                if let NewlineStyle::Tag = find_newline_style(el) {
                    return NewlineStyle::Tag;
                }
            }
        }
        Element { name, .. } => {
            match name.expanded() {
                expanded_name!(html "p") => {
                    return NewlineStyle::Tag;
                }
                expanded_name!(html "br") => {
                    return NewlineStyle::Tag;
                }
                _ => (),
            };
            let children = node.children.borrow();
            for el in children.iter() {
                if let NewlineStyle::Tag = find_newline_style(el) {
                    return NewlineStyle::Tag;
                }
            }
        }
        _ => (),
    }
    NewlineStyle::Text
}

fn handle_child(buffer: &mut String, node: &Handle, state: &mut ParserState) {
    match &node.data {
        Document => {
            let children = node.children.borrow();
            for el in children.iter() {
                handle_child(buffer, el, state);
            }
        }
        Element { name, attrs, .. } => {
            let mut wrapper_href = None;
            let wrapper_tag;
            let mut is_p_tag = false;
            match name.expanded() {
                // Supported tags in pango markup
                // https://docs.gtk.org/Pango/pango_markup.html
                expanded_name!(html "a") => {
                    let local_name = local_name!("href");
                    wrapper_href = attrs
                        .borrow()
                        .iter()
                        .find(|attr| attr.name.local == local_name)
                        .cloned();
                    wrapper_tag = Some("a")
                }
                expanded_name!(html "p") => {
                    is_p_tag = true;
                    wrapper_tag = None
                }
                expanded_name!(html "br") => {
                    buffer.push('\n');
                    state.skip_leading_spaces = true;
                    wrapper_tag = None
                }
                expanded_name!(html "b") => wrapper_tag = Some("b"),
                expanded_name!(html "i") => wrapper_tag = Some("i"),
                expanded_name!(html "s") => wrapper_tag = Some("s"),
                expanded_name!(html "u") => wrapper_tag = Some("u"),
                expanded_name!(html "tt") => wrapper_tag = Some("tt"),
                expanded_name!(html "pre") => wrapper_tag = Some("tt"),
                expanded_name!(html "code") => wrapper_tag = Some("tt"),
                expanded_name!(html "sub") => wrapper_tag = Some("sub"),
                expanded_name!(html "sup") => wrapper_tag = Some("sup"),
                _ => wrapper_tag = None,
            };
            if let Some(tag) = wrapper_tag {
                buffer.push('<');
                buffer.push_str(tag);
                if let Some(href) = wrapper_href {
                    buffer.push_str(" href=\"");
                    buffer.push_str(&escape_amp(&href.value));
                    buffer.push('"');
                }
                buffer.push('>');

                let children = node.children.borrow();
                for el in children.iter() {
                    handle_child(buffer, el, state);
                }
                buffer.push_str("</");
                buffer.push_str(tag);
                buffer.push('>');
            } else {
                let children = node.children.borrow();
                for el in children.iter() {
                    handle_child(buffer, el, state);
                }
                if is_p_tag {
                    buffer.push_str("\n\n");
                    state.skip_leading_spaces = true;
                }
            }
        }
        Text { contents } => {
            let text = if let NewlineStyle::Tag = state.nl_style {
                remove_nl(&escape_amp(&contents.borrow()))
            } else {
                escape_amp(&contents.borrow())
            };

            if state.skip_leading_spaces {
                state.skip_leading_spaces = false;
                buffer.push_str(text.trim_start());
            } else {
                buffer.push_str(&text);
            }
        }
        _ => (),
    }
}

pub fn html2pango_markup(t: &str) -> String {
    let mut buffer = String::with_capacity(t.len());
    let opts = ParseOpts {
        tree_builder: TreeBuilderOpts {
            drop_doctype: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let dom: RcDom = parse_document(RcDom::default(), opts)
        .from_utf8()
        .read_from(&mut t.as_bytes())
        .unwrap();

    let root: Handle = dom.document;
    let nl_style = find_newline_style(&root);
    handle_child(
        &mut buffer,
        &root,
        &mut ParserState {
            nl_style,
            skip_leading_spaces: false,
        },
    );

    buffer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_based() -> () {
        let description = "<p>Here is an unlocked Britainology to hopefully brighten your weekend. Yes, it was a little bit late when it came out. It was late because Nate and Milo got exposed to covid right before Christmas and couldn\'t record while waiting on test results. But we have in fact discussed Jim Davidson\'s terrible bawdy Cinderella pantomime from 1995, entitled \'Sinderella\', and discussed the concept of pantos in general. Hope you enjoy!</p>\n\n<p>We\'ve also unlocked the Dogging episode of Britainology to the $5 tier--get it here: <a href=\"https://www.patreon.com/posts/51682396\">https://www.patreon.com/posts/51682396</a></p>\n\n<p>And if you want two (2) Britainologies a month, sign up on the $10 tier on Patreon! This month\'s second episode features us discussing the British Army with friend of the show, veteran, leftist, and author Joe Glenton: <a href=\"https://www.patreon.com/posts/56590770\">https://www.patreon.com/posts/56590770</a></p>\n\n<p>*MILO ALERT* Smoke returns for another night of new material from pro-comics featuring Edinburgh Comedy Award winner Jordan Brookes. See it all, for the low price of £5, on September 28 at 8 pm at The Sekforde Arms (34 Sekforde Street London EC1R 0HA): <a href=\"https://www.eventbrite.co.uk/e/smoke-comedy-featuring-jordan-brookes-tickets-171869475227\">https://www.eventbrite.co.uk/e/smoke-comedy-featuring-jordan-brookes-tickets-171869475227</a></p>\n\n<p>Trashfuture are: Riley (<a href=\"https://twitter.com/raaleh\">@raaleh</a>), Milo (<a href=\"https://twitter.com/Milo_Edwards\">@Milo_Edwards</a>), Hussein (<a href=\"https://twitter.com/HKesvani\">@HKesvani</a>), Nate (<a href=\"https://twitter.com/inthesedeserts\">@inthesedeserts</a>), and Alice (<a href=\"https://twitter.com/AliceAvizandum\">@AliceAvizandum</a>)</p>";
        let expected = "Here is an unlocked Britainology to hopefully brighten your weekend. Yes, it was a little bit late when it came out. It was late because Nate and Milo got exposed to covid right before Christmas and couldn't record while waiting on test results. But we have in fact discussed Jim Davidson's terrible bawdy Cinderella pantomime from 1995, entitled 'Sinderella', and discussed the concept of pantos in general. Hope you enjoy!\n\nWe've also unlocked the Dogging episode of Britainology to the $5 tier--get it here: <a href=\"https://www.patreon.com/posts/51682396\">https://www.patreon.com/posts/51682396</a>\n\nAnd if you want two (2) Britainologies a month, sign up on the $10 tier on Patreon! This month's second episode features us discussing the British Army with friend of the show, veteran, leftist, and author Joe Glenton: <a href=\"https://www.patreon.com/posts/56590770\">https://www.patreon.com/posts/56590770</a>\n\n*MILO ALERT* Smoke returns for another night of new material from pro-comics featuring Edinburgh Comedy Award winner Jordan Brookes. See it all, for the low price of £5, on September 28 at 8 pm at The Sekforde Arms (34 Sekforde Street London EC1R 0HA): <a href=\"https://www.eventbrite.co.uk/e/smoke-comedy-featuring-jordan-brookes-tickets-171869475227\">https://www.eventbrite.co.uk/e/smoke-comedy-featuring-jordan-brookes-tickets-171869475227</a>\n\nTrashfuture are: Riley (<a href=\"https://twitter.com/raaleh\">@raaleh</a>), Milo (<a href=\"https://twitter.com/Milo_Edwards\">@Milo_Edwards</a>), Hussein (<a href=\"https://twitter.com/HKesvani\">@HKesvani</a>), Nate (<a href=\"https://twitter.com/inthesedeserts\">@inthesedeserts</a>), and Alice (<a href=\"https://twitter.com/AliceAvizandum\">@AliceAvizandum</a>)\n\n";
        let markup = html2pango_markup(&description);

        assert_eq!(expected, markup);
    }
    #[test]
    fn test_html_based2() -> () {
        let description = "<p>So, in rank defiance of our recent promise to \'get back to the nazis\' instead we continue our James Lindsay coverage.&nbsp; (What... me? Irony? How dare you?)&nbsp; This time, Daniel patiently walks a distracted, slightly hyperactive, and increasingly incredulous Jack through the infamous \'Grievance Studies Hoax\' (AKA \'Sokal Squared\') in which Lindsay and colleagues Helen Pluckrose and Peter Boghossian tried (and then claimed) to prove something or other about modern Humanities academia by submitting a load of stupid fake papers to various feminist and fat studies journals.&nbsp; As Daniel reveals, the episode was an orgy of dishonesty and tactical point-missing that actually proved the opposite of what the team of snickering tricksters thought they were proving.&nbsp; Sadly, however, because we live in Hell, the trio have only raised their profiles as a result.&nbsp; A particular highlight of the episode is Lindsay revealing his staggering ignorance when \'responding\' to criticism.</p> <p>Content warnings, as ever.</p> <p><span>Podcast Notes:</span></p> <p>Please consider donating to help us make the show and stay independent.&nbsp; Patrons get exclusive access to one full extra episode a month.</p> <p>Daniel\'s Patreon: <a href=\"https://www.patreon.com/danielharper\">https://www.patreon.com/danielharper</a></p> <p>Jack\'s Patreon: <a href=\"https://www.patreon.com/user?u=4196618&amp;fan_landing=true\">https://www.patreon.com/user?u=4196618</a></p> <p>IDSG Twitter: <a href=\"https://twitter.com/idsgpod\">https://twitter.com/idsgpod</a></p> <p>Daniel\'s Twitter: <a href=\"https://twitter.com/danieleharper\">@danieleharper</a></p> <p>Jack\'s Twitter: <a href=\"https://twitter.com/_Jack_Graham_\">@_Jack_Graham_</a></p> <p>IDSG on Apple Podcasts: <a href=\"https://podcasts.apple.com/us/podcast/i-dont-speak-german/id1449848509?ls=1\"> https://podcasts.apple.com/us/podcast/i-dont-speak-german/id1449848509?ls=1</a></p> <p>&nbsp;</p> <p><span>Show Notes:</span></p> <p>James Lindsay, New Discourses, \"Why You Can Be Transgender But Not Transracial.\"\" <a href=\"https://newdiscourses.com/2021/06/why-you-can-be-transgender-but-not-transracial/\"> https://newdiscourses.com/2021/06/why-you-can-be-transgender-but-not-transracial/</a></p> <p>James Lindsay has a day job, apparently. \"Maryville man walks path of healing and combat.\" <a href=\"https://www.thedailytimes.com/news/maryville-man-walks-path-of-healing-and-combat/article_5ea3c0ca-2e98-5283-9e59-06861b8588cb.html\"> https://www.thedailytimes.com/news/maryville-man-walks-path-of-healing-and-combat/article_5ea3c0ca-2e98-5283-9e59-06861b8588cb.html</a></p> <p>Areo Magazine, Academic Grievance Studies and the Corruption of Scholarship. <a href=\"https://areomagazine.com/2018/10/02/academic-grievance-studies-and-the-corruption-of-scholarship/\"> https://areomagazine.com/2018/10/02/academic-grievance-studies-and-the-corruption-of-scholarship/</a></p> <p>Full listing of Grievance Studies Papers and Reviews. <a href=\"https://drive.google.com/drive/folders/19tBy_fVlYIHTxxjuVMFxh4pqLHM_en18\"> https://drive.google.com/drive/folders/19tBy_fVlYIHTxxjuVMFxh4pqLHM_en18</a></p> <p>\'Mein Kampf\' and the \'Feminazis\': What Three Academics\' Hitler Hoax Really Reveals About \'Wokeness\'. <a href=\"https://web.archive.org/web/20210328112901/https://www.haaretz.com/us-news/.premium-hitler-hoax-academic-wokeness-culture-war-1.9629759\"> https://web.archive.org/web/20210328112901/https://www.haaretz.com/us-news/.premium-hitler-hoax-academic-wokeness-culture-war-1.9629759</a></p> <p>\"First and foremost, the source material. The chapter the hoaxers chose, not by coincidence, one of the least ideological and racist parts of Hitler\'s book. Chapter 12, probably written in April/May 1925, deals with how the newly refounded NSDAP should rebuild as a party and amplify its program.</p> <p>\"According to their own account, the writers took parts of the chapter and inserted feminist \"buzzwords\"; they \"significantly changed\" the \"original wording and intent” of the text to make the paper \"publishable and about feminism.\" An observant reader might ask: what could possibly remain of any Nazi content after that? But no one in the media, apparently, did.\"</p> <p>New Discourses, \"There Is No Good Part of Hitler\'s Mein Kampf\" <a href=\"https://newdiscourses.com/2021/03/there-is-no-good-part-of-hitlers-mein-kampf/\"> https://newdiscourses.com/2021/03/there-is-no-good-part-of-hitlers-mein-kampf/</a></p> <p>On this episode of the New Discourses Podcast, James Lindsay, who helped to write the paper and perpetrate the Grievance Studies Affair, talks about the project and the creation of this particular paper at unprecedented length and in unprecedented detail, revealing Nilssen not to know what he’s talking about. If you have ever wondered about what the backstory of the creation of the “Feminist Mein Kampf” paper really was, including why its authors did it, you won’t want to miss this long-form discussion and rare response to yet another underinformed critic of Lindsay, Boghossian, and Pluckrose’s work.</p> <p>The Grieveance Studies Affair Revealed. <a href=\"https://www.youtube.com/watch?v=kVk9a5Jcd1k\">https://www.youtube.com/watch?v=kVk9a5Jcd1k</a></p> <p>Reviewer 1 Comments on Dog Park Paper</p> <p>\"page 9 - the human subjects are afforded anonymity and not asked about income, etc for ethical reasons. yet, the author as researcher intruded into the dogs\' spaces to examine and record genitalia. I realize this was necessary to the project, but could the author acknowledge/explain/justify this (arguably, anthropocentric) difference? Indicating that it was necessary to the research would suffice but at least the difference should be acknowledged.\"</p> <p>Nestor de Buen, Anti-Science Humping in the Dog Park. <a href=\"https://conceptualdisinformation.substack.com/p/anti-science-humping-in-the-dog-park\"> https://conceptualdisinformation.substack.com/p/anti-science-humping-in-the-dog-park</a></p> <p>\"What is even more striking is that if the research had actually been conducted and the results showed what the paper says they show, there is absolutely no reason why it should not have been published. And moreover, what it proves is the opposite of what its intention is. It shows that one can make scientifically testable claims based on the conceptual framework of gender studies, and that the field has all the markings of a perfectly functional research programme.\"</p> <p>\"Yes, the dog park paper is based on false data and, like Sokal’s, contains a lot of unnecessary jargon, but it is not nonsense, and the distinction is far from trivial. Nonsense implies one cannot even obtain a truth value from a proposition. In fact, the paper being false, if anything, proves that it is not nonsense, yet the grievance hoaxers try to pass falsity as nonsense. Nonsense is something like Chomsky’s famous sentence “colorless green ideas sleep furiously.” It is nonsense because it is impossible to decide how one might evaluate whether it is true. A false sentence would be “the moon is cubical.” It has a definite meaning, it just happens not to be true.&nbsp;</p> <p>\"So, if the original Sokal Hoax is like Chomsky’s sentence, the dog park paper is much more like “the moon is cubical.” And in fact, a more accurate analogy would be “the moon is cubical and here is a picture that proves it,” and an attached doctored picture of the cubical moon.\"</p> <p>Reviewer 2 Comments on the Dog-Park Paper</p> <p>\"I am a bit curious about your methodology. Can you say more? You describe your methods here (procedures for collecting data), but not really your overall approach to methodology. Did you just show up, observe, write copious notes, talk to people when necessary, and then leave? If so, it might be helpful to explicitly state this. It sounds to me like you did a kind of ethnography (methodology — maybe multispecies ethnography?) but that’s not entirely clear here. Or are you drawing on qualitative methods in social behaviorism/symbolic interactionism? In either case, the methodology chosen should be a bit more clearly articulated.\"</p> <p>Counterweight. <a href=\"https://counterweightsupport.com/\">https://counterweightsupport.com/</a></p> <p>\"Welcome to Counterweight, the home of scholarship and advice on [Critical Social Justice](<a href=\"https://counterweightsupport.com/2021/02/17/what-do-we-mean-by-critical-social-justice/\">https://counterweightsupport.com/2021/02/17/what-do-we-mean-by-critical-social-justice/</a>) ideology. We are here to connect you with the resources, advice and guidance you need to address CSJ beliefs as you encounter them in your day-to-day life. The Counterweight community is a non-partisan, grassroots movement advocating for liberal concepts of social justice including individualism, universalism, viewpoint diversity and the free exchange of ideas. [Subscribe](https://counterweightsupport.com/subscribe-to-counterweight/) today to become part of the Counterweight movement.\"\"</p> <p>Inside Higher Ed, \"Blowback Against a Hoax.\" <a href=\"https://www.insidehighered.com/news/2019/01/08/author-recent-academic-hoax-faces-disciplinary-action-portland-state\"> https://www.insidehighered.com/news/2019/01/08/author-recent-academic-hoax-faces-disciplinary-action-portland-state</a></p> <p>Peter Boghossian Resignation Latter from PSU. <a href=\"https://bariweiss.substack.com/p/my-university-sacrificed-ideas-for\"> https://bariweiss.substack.com/p/my-university-sacrificed-ideas-for</a></p> <p>&nbsp;</p>";
        let expected = "So, in rank defiance of our recent promise to 'get back to the nazis' instead we continue our James Lindsay coverage.\u{a0} (What... me? Irony? How dare you?)\u{a0} This time, Daniel patiently walks a distracted, slightly hyperactive, and increasingly incredulous Jack through the infamous 'Grievance Studies Hoax' (AKA 'Sokal Squared') in which Lindsay and colleagues Helen Pluckrose and Peter Boghossian tried (and then claimed) to prove something or other about modern Humanities academia by submitting a load of stupid fake papers to various feminist and fat studies journals.\u{a0} As Daniel reveals, the episode was an orgy of dishonesty and tactical point-missing that actually proved the opposite of what the team of snickering tricksters thought they were proving.\u{a0} Sadly, however, because we live in Hell, the trio have only raised their profiles as a result.\u{a0} A particular highlight of the episode is Lindsay revealing his staggering ignorance when 'responding' to criticism.\n\nContent warnings, as ever.\n\nPodcast Notes:\n\nPlease consider donating to help us make the show and stay independent.\u{a0} Patrons get exclusive access to one full extra episode a month.\n\nDaniel's Patreon: <a href=\"https://www.patreon.com/danielharper\">https://www.patreon.com/danielharper</a>\n\nJack's Patreon: <a href=\"https://www.patreon.com/user?u=4196618&amp;fan_landing=true\">https://www.patreon.com/user?u=4196618</a>\n\nIDSG Twitter: <a href=\"https://twitter.com/idsgpod\">https://twitter.com/idsgpod</a>\n\nDaniel's Twitter: <a href=\"https://twitter.com/danieleharper\">@danieleharper</a>\n\nJack's Twitter: <a href=\"https://twitter.com/_Jack_Graham_\">@_Jack_Graham_</a>\n\nIDSG on Apple Podcasts: <a href=\"https://podcasts.apple.com/us/podcast/i-dont-speak-german/id1449848509?ls=1\"> https://podcasts.apple.com/us/podcast/i-dont-speak-german/id1449848509?ls=1</a>\n\n\u{a0}\n\nShow Notes:\n\nJames Lindsay, New Discourses, \"Why You Can Be Transgender But Not Transracial.\"\" <a href=\"https://newdiscourses.com/2021/06/why-you-can-be-transgender-but-not-transracial/\"> https://newdiscourses.com/2021/06/why-you-can-be-transgender-but-not-transracial/</a>\n\nJames Lindsay has a day job, apparently. \"Maryville man walks path of healing and combat.\" <a href=\"https://www.thedailytimes.com/news/maryville-man-walks-path-of-healing-and-combat/article_5ea3c0ca-2e98-5283-9e59-06861b8588cb.html\"> https://www.thedailytimes.com/news/maryville-man-walks-path-of-healing-and-combat/article_5ea3c0ca-2e98-5283-9e59-06861b8588cb.html</a>\n\nAreo Magazine, Academic Grievance Studies and the Corruption of Scholarship. <a href=\"https://areomagazine.com/2018/10/02/academic-grievance-studies-and-the-corruption-of-scholarship/\"> https://areomagazine.com/2018/10/02/academic-grievance-studies-and-the-corruption-of-scholarship/</a>\n\nFull listing of Grievance Studies Papers and Reviews. <a href=\"https://drive.google.com/drive/folders/19tBy_fVlYIHTxxjuVMFxh4pqLHM_en18\"> https://drive.google.com/drive/folders/19tBy_fVlYIHTxxjuVMFxh4pqLHM_en18</a>\n\n'Mein Kampf' and the 'Feminazis': What Three Academics' Hitler Hoax Really Reveals About 'Wokeness'. <a href=\"https://web.archive.org/web/20210328112901/https://www.haaretz.com/us-news/.premium-hitler-hoax-academic-wokeness-culture-war-1.9629759\"> https://web.archive.org/web/20210328112901/https://www.haaretz.com/us-news/.premium-hitler-hoax-academic-wokeness-culture-war-1.9629759</a>\n\n\"First and foremost, the source material. The chapter the hoaxers chose, not by coincidence, one of the least ideological and racist parts of Hitler's book. Chapter 12, probably written in April/May 1925, deals with how the newly refounded NSDAP should rebuild as a party and amplify its program.\n\n\"According to their own account, the writers took parts of the chapter and inserted feminist \"buzzwords\"; they \"significantly changed\" the \"original wording and intent” of the text to make the paper \"publishable and about feminism.\" An observant reader might ask: what could possibly remain of any Nazi content after that? But no one in the media, apparently, did.\"\n\nNew Discourses, \"There Is No Good Part of Hitler's Mein Kampf\" <a href=\"https://newdiscourses.com/2021/03/there-is-no-good-part-of-hitlers-mein-kampf/\"> https://newdiscourses.com/2021/03/there-is-no-good-part-of-hitlers-mein-kampf/</a>\n\nOn this episode of the New Discourses Podcast, James Lindsay, who helped to write the paper and perpetrate the Grievance Studies Affair, talks about the project and the creation of this particular paper at unprecedented length and in unprecedented detail, revealing Nilssen not to know what he’s talking about. If you have ever wondered about what the backstory of the creation of the “Feminist Mein Kampf” paper really was, including why its authors did it, you won’t want to miss this long-form discussion and rare response to yet another underinformed critic of Lindsay, Boghossian, and Pluckrose’s work.\n\nThe Grieveance Studies Affair Revealed. <a href=\"https://www.youtube.com/watch?v=kVk9a5Jcd1k\">https://www.youtube.com/watch?v=kVk9a5Jcd1k</a>\n\nReviewer 1 Comments on Dog Park Paper\n\n\"page 9 - the human subjects are afforded anonymity and not asked about income, etc for ethical reasons. yet, the author as researcher intruded into the dogs' spaces to examine and record genitalia. I realize this was necessary to the project, but could the author acknowledge/explain/justify this (arguably, anthropocentric) difference? Indicating that it was necessary to the research would suffice but at least the difference should be acknowledged.\"\n\nNestor de Buen, Anti-Science Humping in the Dog Park. <a href=\"https://conceptualdisinformation.substack.com/p/anti-science-humping-in-the-dog-park\"> https://conceptualdisinformation.substack.com/p/anti-science-humping-in-the-dog-park</a>\n\n\"What is even more striking is that if the research had actually been conducted and the results showed what the paper says they show, there is absolutely no reason why it should not have been published. And moreover, what it proves is the opposite of what its intention is. It shows that one can make scientifically testable claims based on the conceptual framework of gender studies, and that the field has all the markings of a perfectly functional research programme.\"\n\n\"Yes, the dog park paper is based on false data and, like Sokal’s, contains a lot of unnecessary jargon, but it is not nonsense, and the distinction is far from trivial. Nonsense implies one cannot even obtain a truth value from a proposition. In fact, the paper being false, if anything, proves that it is not nonsense, yet the grievance hoaxers try to pass falsity as nonsense. Nonsense is something like Chomsky’s famous sentence “colorless green ideas sleep furiously.” It is nonsense because it is impossible to decide how one might evaluate whether it is true. A false sentence would be “the moon is cubical.” It has a definite meaning, it just happens not to be true.\u{a0}\n\n\"So, if the original Sokal Hoax is like Chomsky’s sentence, the dog park paper is much more like “the moon is cubical.” And in fact, a more accurate analogy would be “the moon is cubical and here is a picture that proves it,” and an attached doctored picture of the cubical moon.\"\n\nReviewer 2 Comments on the Dog-Park Paper\n\n\"I am a bit curious about your methodology. Can you say more? You describe your methods here (procedures for collecting data), but not really your overall approach to methodology. Did you just show up, observe, write copious notes, talk to people when necessary, and then leave? If so, it might be helpful to explicitly state this. It sounds to me like you did a kind of ethnography (methodology — maybe multispecies ethnography?) but that’s not entirely clear here. Or are you drawing on qualitative methods in social behaviorism/symbolic interactionism? In either case, the methodology chosen should be a bit more clearly articulated.\"\n\nCounterweight. <a href=\"https://counterweightsupport.com/\">https://counterweightsupport.com/</a>\n\n\"Welcome to Counterweight, the home of scholarship and advice on [Critical Social Justice](<a href=\"https://counterweightsupport.com/2021/02/17/what-do-we-mean-by-critical-social-justice/\">https://counterweightsupport.com/2021/02/17/what-do-we-mean-by-critical-social-justice/</a>) ideology. We are here to connect you with the resources, advice and guidance you need to address CSJ beliefs as you encounter them in your day-to-day life. The Counterweight community is a non-partisan, grassroots movement advocating for liberal concepts of social justice including individualism, universalism, viewpoint diversity and the free exchange of ideas. [Subscribe](https://counterweightsupport.com/subscribe-to-counterweight/) today to become part of the Counterweight movement.\"\"\n\nInside Higher Ed, \"Blowback Against a Hoax.\" <a href=\"https://www.insidehighered.com/news/2019/01/08/author-recent-academic-hoax-faces-disciplinary-action-portland-state\"> https://www.insidehighered.com/news/2019/01/08/author-recent-academic-hoax-faces-disciplinary-action-portland-state</a>\n\nPeter Boghossian Resignation Latter from PSU. <a href=\"https://bariweiss.substack.com/p/my-university-sacrificed-ideas-for\"> https://bariweiss.substack.com/p/my-university-sacrificed-ideas-for</a>\n\n\u{a0}\n\n";
        let markup = html2pango_markup(&description);

        assert_eq!(expected, markup);
    }

    #[test]
    fn test_newline_based() -> () {
        let description = "Also available in video form at https://youtu.be/NUPWY_evu30\n\nIn a recent view by Contrapoints, she goes over her account of envy and its connection with online politics. In doing so she utilizes Nietzsche (alongside a critique of Nietzsche). How accurate is this account to Nietzsche\'s work and where does it go wrong?  \n\nThank you to We\'re in Hell, BadEmpanada, and Chelsea Manning for the voice lines! \n\nEdited by Lexi Fontaine: https://twitter.com/softgothoutlaw \n\nMusic by Alex Ballantyne: https://transistorriot.bandcamp.com \n\nThis was an early release to my patrons at https://pateron.com/livagar \n\nWatch me stream on twitch at https://twitch.tv/livagar \n\nAll of my links at https:// livagar.com";
        let expected = "Also available in video form at https://youtu.be/NUPWY_evu30\n\nIn a recent view by Contrapoints, she goes over her account of envy and its connection with online politics. In doing so she utilizes Nietzsche (alongside a critique of Nietzsche). How accurate is this account to Nietzsche's work and where does it go wrong?  \n\nThank you to We're in Hell, BadEmpanada, and Chelsea Manning for the voice lines! \n\nEdited by Lexi Fontaine: https://twitter.com/softgothoutlaw \n\nMusic by Alex Ballantyne: https://transistorriot.bandcamp.com \n\nThis was an early release to my patrons at https://pateron.com/livagar \n\nWatch me stream on twitch at https://twitch.tv/livagar \n\nAll of my links at https:// livagar.com";
        let markup = html2pango_markup(&description);

        assert_eq!(expected, markup);
    }

    #[test]
    fn test_newline_based2() -> () {
        let description = "We’re back to a normal-style ep after a week of interviews. We’re taking a look at the fast-tracked aid package to intelligence agents suffering unreality issues, the Biden administration addressing just the optics at the border, and AOC addressing just the optics of the Iron Dome bill. Finally, we having a reading series that functions as a bit of a coda to Will and Matt’s visit to Ozy Fest way back in 2018.\n\nOne last time, go subscribe to https://www.youtube.com/chapotraphouse\n\nAnd go grab some of Simon Roy’s great posters over at https://shop.chapotraphouse.com/\nMore merch coming soon!";
        let expected = "We’re back to a normal-style ep after a week of interviews. We’re taking a look at the fast-tracked aid package to intelligence agents suffering unreality issues, the Biden administration addressing just the optics at the border, and AOC addressing just the optics of the Iron Dome bill. Finally, we having a reading series that functions as a bit of a coda to Will and Matt’s visit to Ozy Fest way back in 2018.\n\nOne last time, go subscribe to https://www.youtube.com/chapotraphouse\n\nAnd go grab some of Simon Roy’s great posters over at https://shop.chapotraphouse.com/\nMore merch coming soon!";
        let markup = html2pango_markup(&description);

        assert_eq!(expected, markup);
    }
}
