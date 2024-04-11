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

use linkify::LinkFinder;
use linkify::LinkKind;
use regex::Regex;

use crate::i18n::i18n_f;
use html5ever::tendril::TendrilSink;
use html5ever::tree_builder::TreeBuilderOpts;
use html5ever::{expanded_name, parse_document, ParseOpts};
use markup5ever_rcdom::{
    Handle,
    NodeData::{Document, Element, Text},
    RcDom,
};

const INDENT: i32 = 4; // used by li tags

#[derive(Clone)]
enum NewlineHandling {
    ToSpace,
    Remove,
    Keep,
}

fn escape_text(t: &str, newline_handling: NewlineHandling) -> String {
    // TODO prevent escaping escape-sequances
    let escaped = t
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let newlined_text = match newline_handling {
        NewlineHandling::ToSpace => escaped.replace('\n', " "),
        NewlineHandling::Remove => escaped.replace('\n', ""),
        NewlineHandling::Keep => escaped,
    };
    collapse_whitespaces(newlined_text)
}

// remove spaces that follow on another space or a newline
fn collapse_whitespaces(string: String) -> String {
    let mut was_space = false;
    string
        .chars()
        .filter(|c| {
            let is_space = c.eq(&' ');
            if is_space && was_space {
                return false;
            }
            was_space = is_space || c.eq(&'\n');
            true
        })
        .collect()
}

// Does the description use \n Text newlines or <br> <p> Tag newlines
#[derive(Debug)]
enum NewlineStyle {
    Text,
    Tag,
}
enum ListStyle {
    Ordered(i32),
    Unordered,
}
struct ParserState {
    nl_style: NewlineStyle,
    skip_leading_spaces: bool,
    indent: i32,
    list_style: Vec<ListStyle>,
    inside_link: i32,
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
            let mut is_p_tag = false;
            let mut is_list_tag = false;
            let wrapper_tag = match name.expanded() {
                // Supported tags in pango markup
                // https://docs.gtk.org/Pango/pango_markup.html
                expanded_name!(html "a") => {
                    let local_name = local_name!("href");
                    wrapper_href = attrs
                        .borrow()
                        .iter()
                        .find(|attr| attr.name.local == local_name)
                        .cloned();
                    // Pango does not support a tags without href,
                    // so return a None in that case
                    wrapper_href.as_ref().map(|_| "a")
                }
                expanded_name!(html "p") => {
                    is_p_tag = true;
                    None
                }
                expanded_name!(html "br") => {
                    buffer.push('\n');
                    state.skip_leading_spaces = true;
                    None
                }
                expanded_name!(html "img") => {
                    let local_name = local_name!("alt");
                    let alt = attrs
                        .borrow()
                        .iter()
                        .find(|attr| attr.name.local == local_name)
                        .cloned();
                    if let Some(alt_text) = alt {
                        let escaped_alt = escape_text(&alt_text.value, NewlineHandling::ToSpace);
                        buffer.push('[');
                        buffer.push_str(escaped_alt.as_str());
                        buffer.push_str("]\n");
                        state.skip_leading_spaces = true;
                    }
                    None
                }
                expanded_name!(html "ol") => {
                    state.list_style.push(ListStyle::Ordered(1));
                    state.indent += INDENT;
                    is_list_tag = true;
                    is_p_tag = true;
                    None
                }
                expanded_name!(html "ul") => {
                    state.list_style.push(ListStyle::Unordered);
                    state.indent += INDENT;
                    is_list_tag = true;
                    is_p_tag = true;
                    None
                }
                expanded_name!(html "li") => {
                    buffer.push('\n');
                    for _ in 0..state.indent {
                        buffer.push(' ');
                    }
                    if let Some(style) = state.list_style.last_mut() {
                        match style {
                            ListStyle::Unordered => buffer.push_str("• "),
                            ListStyle::Ordered(i) => {
                                buffer.push_str(&format!("{}. ", i));
                                *style = ListStyle::Ordered(*i + 1);
                            }
                        }
                    }
                    state.skip_leading_spaces = true;
                    None
                }
                expanded_name!(html "b") => Some("b"),
                expanded_name!(html "i") => Some("i"),
                expanded_name!(html "s") => Some("s"),
                expanded_name!(html "u") => Some("u"),
                expanded_name!(html "tt") => Some("tt"),
                expanded_name!(html "pre") => Some("tt"),
                expanded_name!(html "code") => Some("tt"),
                expanded_name!(html "sub") => Some("sub"),
                expanded_name!(html "sup") => Some("sup"),
                _ => None,
            };
            // Invalid link tag, links that point to # lead nowhere, skip the tag.
            let skip_tag = if let Some(href) = wrapper_href.as_ref() {
                wrapper_tag == Some("a")
                    && (href.value.trim_start().starts_with('#')
                        || href.value.trim().is_empty()
                        || href.value.trim_start().starts_with("jump:"))
            } else {
                false
            };
            let wrote_tag = if skip_tag {
                false
            } else if let Some(tag) = wrapper_tag {
                buffer.push('<');
                buffer.push_str(tag);
                let is_link;
                if let Some(href) = wrapper_href {
                    buffer.push_str(" href=\"");
                    buffer.push_str(&escape_text(&href.value, NewlineHandling::Remove));
                    buffer.push('"');
                    state.inside_link += 1;
                    is_link = true;
                } else {
                    is_link = false;
                }

                buffer.push('>');

                let children = node.children.borrow();
                for el in children.iter() {
                    handle_child(buffer, el, state);
                }
                buffer.push_str("</");
                buffer.push_str(tag);
                buffer.push('>');

                if is_link {
                    state.inside_link -= 1;
                }
                true
            } else {
                false
            };

            if !wrote_tag {
                let children = node.children.borrow();
                for el in children.iter() {
                    handle_child(buffer, el, state);
                }
                if is_p_tag {
                    buffer.push_str("\n\n");
                    state.skip_leading_spaces = true;
                }
                if is_list_tag {
                    state.indent -= INDENT;
                    state.list_style.pop();
                }
            }
        }
        Text { contents } => {
            let nl_handling = match state.nl_style {
                NewlineStyle::Tag => NewlineHandling::ToSpace,
                NewlineStyle::Text => NewlineHandling::Keep,
            };

            if state.skip_leading_spaces {
                let text = escape_text(contents.borrow().trim_start(), nl_handling.clone());
                if !text.is_empty() {
                    state.skip_leading_spaces = false;
                }
                if state.inside_link > 0 {
                    // avoid nested links
                    push_remaining_text(buffer, &text)
                } else {
                    push_timestamped_text(buffer, &text, nl_handling)
                }
            } else {
                let text = escape_text(&contents.borrow(), nl_handling.clone());
                if state.inside_link > 0 {
                    // avoid nested links
                    push_remaining_text(buffer, &text)
                } else {
                    push_timestamped_text(buffer, &text, nl_handling)
                }
            }
        }
        _ => (),
    }
}

fn push_timestamped_text(buffer: &mut String, text: &str, nl_handling: NewlineHandling) {
    let mut position = 0;
    if let Ok(re) = Regex::new(r"([0-9]+):([0-9]+)(?::([0-9]+))?") {
        for captures in re.captures_iter(text) {
            let first: Option<i32> = captures.get(1).and_then(|c| c.as_str().parse().ok());
            let second: Option<i32> = captures.get(2).and_then(|c| c.as_str().parse().ok());
            let third: Option<i32> = captures.get(3).and_then(|c| c.as_str().parse().ok());
            if let (Some(hours), Some(minutes), Some(seconds)) = (first, second, third) {
                let jump_time = (hours * 60 * 60) + (minutes * 60) + seconds;
                // Jump to Hours:Minutes:Seconds
                let title = i18n_f(
                    "Jump to {}:{}:{}",
                    &[
                        &format!("{:02}", hours),
                        &format!("{:02}", minutes),
                        &format!("{:02}", seconds),
                    ],
                );
                let range = captures.get(0).unwrap().range();

                push_text_with_links(buffer, &text[position..range.start], nl_handling.clone());
                buffer
                    .push_str(format!("<a href=\"jump:{jump_time}\" title=\"{title}\">").as_str());
                buffer.push_str(&text[range.start..range.end]);
                buffer.push_str("</a>");
                position = range.end;
            } else if let (Some(minutes), Some(seconds)) = (first, second) {
                let jump_time = (minutes * 60) + seconds;
                // Jump to Minutes:Seconds
                let title = i18n_f(
                    "Jump to {}:{}",
                    &[&format!("{:02}", minutes), &format!("{:02}", seconds)],
                );
                let range = captures.get(0).unwrap().range();

                push_text_with_links(buffer, &text[position..range.start], nl_handling.clone());
                buffer.push_str(
                    format!("<a href=\"jump:{}\" title=\"{title}\">", jump_time).as_str(),
                );
                buffer.push_str(&text[range.start..range.end]);
                buffer.push_str("</a>");
                position = range.end;
            }
        }
        push_text_with_links(buffer, &text[position..], nl_handling);
    } else {
        push_text_with_links(buffer, text, nl_handling);
    }
}

fn push_text_with_links(buffer: &mut String, text: &str, nl_handling: NewlineHandling) {
    let mut finder = LinkFinder::new();
    finder.url_must_have_scheme(false);
    let mut position = 0;
    for link in finder.links(text) {
        let link_str = link.as_str();
        let remaining_link_text = escape_text(&text[position..link.start()], nl_handling.clone());
        push_remaining_text(buffer, &remaining_link_text);

        match link.kind() {
            LinkKind::Email => {
                buffer.push_str(format!("<a href=\"mailto:{}\">", link_str).as_str());
            }
            LinkKind::Url => {
                if link.as_str().starts_with("http://") || link_str.starts_with("https://") {
                    buffer.push_str(format!("<a href=\"{}\">", link_str).as_str());
                } else {
                    buffer.push_str(format!("<a href=\"https:{}\">", link_str).as_str());
                }
            }
            _ => {
                buffer.push_str(format!("<a href=\"{}\">", link_str).as_str());
            }
        }
        push_remaining_text(buffer, &text[(link.start())..(link.end())]);
        buffer.push_str("</a>");
        position = link.end();
    }

    let end_text = escape_text(&text[position..], nl_handling);
    push_remaining_text(buffer, &end_text);
}

pub fn push_remaining_text(buffer: &mut String, text: &str) {
    // start adding new plaintext replacements here
    buffer.push_str(text);
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
            skip_leading_spaces: true,
            indent: 0,
            list_style: vec![],
            inside_link: 0,
        },
    );
    buffer
}

#[cfg(test)]
mod tests {
    use super::*;

    // feeds to test the UI
    //
    // html/images/escape-codes/lots of spacing & weirdly placed newlines:
    // http://faif.us/feeds/cast-ogg/
    //
    // html/jump-codes:
    // https://soundcloud.com/jimquisition
    //
    // html/jump-codes/lists:
    // https://rustacean-station.org/podcast.rss
    //
    // text-newlines:
    // https://podcasts.apple.com/de/podcast/liv-agar/id1535599001
    // https://soundcloud.com/qanonanonymous
    // https://soundcloud.com/chapo-trap-house
    //
    // html/lists/lots of text & linked sources:
    // http://feeds.libsyn.com/152597/rss

    #[test]
    fn test_html_based() {
        let description = "<p>Here is an unlocked Britainology to hopefully brighten your weekend. Yes, it was a little bit late when it came out. It was late because Nate and Milo got exposed to covid right before Christmas and couldn\'t record while waiting on test results. But we have in fact discussed Jim Davidson\'s terrible bawdy Cinderella pantomime from 1995, entitled \'Sinderella\', and discussed the concept of pantos in general. Hope you enjoy!</p>\n\n<p>We\'ve also unlocked the Dogging episode of Britainology to the $5 tier--get it here: <a href=\"https://www.patreon.com/posts/51682396\">https://www.patreon.com/posts/51682396</a></p>\n\n<p>And if you want two (2) Britainologies a month, sign up on the $10 tier on Patreon! This month\'s second episode features us discussing the British Army with friend of the show, veteran, leftist, and author Joe Glenton: <a href=\"https://www.patreon.com/posts/56590770\">https://www.patreon.com/posts/56590770</a></p>\n\n<p>*MILO ALERT* Smoke returns for another night of new material from pro-comics featuring Edinburgh Comedy Award winner Jordan Brookes. See it all, for the low price of £5, on September 28 at 8 pm at The Sekforde Arms (34 Sekforde Street London EC1R 0HA): <a href=\"https://www.eventbrite.co.uk/e/smoke-comedy-featuring-jordan-brookes-tickets-171869475227\">https://www.eventbrite.co.uk/e/smoke-comedy-featuring-jordan-brookes-tickets-171869475227</a></p>\n\n<p>Trashfuture are: Riley (<a href=\"https://twitter.com/raaleh\">@raaleh</a>), Milo (<a href=\"https://twitter.com/Milo_Edwards\">@Milo_Edwards</a>), Hussein (<a href=\"https://twitter.com/HKesvani\">@HKesvani</a>), Nate (<a href=\"https://twitter.com/inthesedeserts\">@inthesedeserts</a>), and Alice (<a href=\"https://twitter.com/AliceAvizandum\">@AliceAvizandum</a>)</p>";
        let expected = "Here is an unlocked Britainology to hopefully brighten your weekend. Yes, it was a little bit late when it came out. It was late because Nate and Milo got exposed to covid right before Christmas and couldn't record while waiting on test results. But we have in fact discussed Jim Davidson's terrible bawdy Cinderella pantomime from 1995, entitled 'Sinderella', and discussed the concept of pantos in general. Hope you enjoy!\n\nWe've also unlocked the Dogging episode of Britainology to the $5 tier--get it here: <a href=\"https://www.patreon.com/posts/51682396\">https://www.patreon.com/posts/51682396</a>\n\nAnd if you want two (2) Britainologies a month, sign up on the $10 tier on Patreon! This month's second episode features us discussing the British Army with friend of the show, veteran, leftist, and author Joe Glenton: <a href=\"https://www.patreon.com/posts/56590770\">https://www.patreon.com/posts/56590770</a>\n\n*MILO ALERT* Smoke returns for another night of new material from pro-comics featuring Edinburgh Comedy Award winner Jordan Brookes. See it all, for the low price of £5, on September 28 at 8 pm at The Sekforde Arms (34 Sekforde Street London EC1R 0HA): <a href=\"https://www.eventbrite.co.uk/e/smoke-comedy-featuring-jordan-brookes-tickets-171869475227\">https://www.eventbrite.co.uk/e/smoke-comedy-featuring-jordan-brookes-tickets-171869475227</a>\n\nTrashfuture are: Riley (<a href=\"https://twitter.com/raaleh\">@raaleh</a>), Milo (<a href=\"https://twitter.com/Milo_Edwards\">@Milo_Edwards</a>), Hussein (<a href=\"https://twitter.com/HKesvani\">@HKesvani</a>), Nate (<a href=\"https://twitter.com/inthesedeserts\">@inthesedeserts</a>), and Alice (<a href=\"https://twitter.com/AliceAvizandum\">@AliceAvizandum</a>)\n\n";
        let markup = html2pango_markup(description);

        assert_eq!(expected, markup);
    }
    #[test]
    fn test_html_based2() {
        let description = "<p>So, in rank defiance of our recent promise to \'get back to the nazis\' instead we continue our James Lindsay coverage.&nbsp; (What... me? Irony? How dare you?)&nbsp; This time, Daniel patiently walks a distracted, slightly hyperactive, and increasingly incredulous Jack through the infamous \'Grievance Studies Hoax\' (AKA \'Sokal Squared\') in which Lindsay and colleagues Helen Pluckrose and Peter Boghossian tried (and then claimed) to prove something or other about modern Humanities academia by submitting a load of stupid fake papers to various feminist and fat studies journals.&nbsp; As Daniel reveals, the episode was an orgy of dishonesty and tactical point-missing that actually proved the opposite of what the team of snickering tricksters thought they were proving.&nbsp; Sadly, however, because we live in Hell, the trio have only raised their profiles as a result.&nbsp; A particular highlight of the episode is Lindsay revealing his staggering ignorance when \'responding\' to criticism.</p> <p>Content warnings, as ever.</p> <p><span>Podcast Notes:</span></p> <p>Please consider donating to help us make the show and stay independent.&nbsp; Patrons get exclusive access to one full extra episode a month.</p> <p>Daniel\'s Patreon: <a href=\"https://www.patreon.com/danielharper\">https://www.patreon.com/danielharper</a></p> <p>Jack\'s Patreon: <a href=\"https://www.patreon.com/user?u=4196618&amp;fan_landing=true\">https://www.patreon.com/user?u=4196618</a></p> <p>IDSG Twitter: <a href=\"https://twitter.com/idsgpod\">https://twitter.com/idsgpod</a></p> <p>Daniel\'s Twitter: <a href=\"https://twitter.com/danieleharper\">@danieleharper</a></p> <p>Jack\'s Twitter: <a href=\"https://twitter.com/_Jack_Graham_\">@_Jack_Graham_</a></p> <p>IDSG on Apple Podcasts: <a href=\"https://podcasts.apple.com/us/podcast/i-dont-speak-german/id1449848509?ls=1\"> https://podcasts.apple.com/us/podcast/i-dont-speak-german/id1449848509?ls=1</a></p> <p>&nbsp;</p> <p><span>Show Notes:</span></p> <p>James Lindsay, New Discourses, \"Why You Can Be Transgender But Not Transracial.\"\" <a href=\"https://newdiscourses.com/2021/06/why-you-can-be-transgender-but-not-transracial/\"> https://newdiscourses.com/2021/06/why-you-can-be-transgender-but-not-transracial/</a></p> <p>James Lindsay has a day job, apparently. \"Maryville man walks path of healing and combat.\" <a href=\"https://www.thedailytimes.com/news/maryville-man-walks-path-of-healing-and-combat/article_5ea3c0ca-2e98-5283-9e59-06861b8588cb.html\"> https://www.thedailytimes.com/news/maryville-man-walks-path-of-healing-and-combat/article_5ea3c0ca-2e98-5283-9e59-06861b8588cb.html</a></p> <p>Areo Magazine, Academic Grievance Studies and the Corruption of Scholarship. <a href=\"https://areomagazine.com/2018/10/02/academic-grievance-studies-and-the-corruption-of-scholarship/\"> https://areomagazine.com/2018/10/02/academic-grievance-studies-and-the-corruption-of-scholarship/</a></p> <p>Full listing of Grievance Studies Papers and Reviews. <a href=\"https://drive.google.com/drive/folders/19tBy_fVlYIHTxxjuVMFxh4pqLHM_en18\"> https://drive.google.com/drive/folders/19tBy_fVlYIHTxxjuVMFxh4pqLHM_en18</a></p> <p>\'Mein Kampf\' and the \'Feminazis\': What Three Academics\' Hitler Hoax Really Reveals About \'Wokeness\'. <a href=\"https://web.archive.org/web/20210328112901/https://www.haaretz.com/us-news/.premium-hitler-hoax-academic-wokeness-culture-war-1.9629759\"> https://web.archive.org/web/20210328112901/https://www.haaretz.com/us-news/.premium-hitler-hoax-academic-wokeness-culture-war-1.9629759</a></p> <p>\"First and foremost, the source material. The chapter the hoaxers chose, not by coincidence, one of the least ideological and racist parts of Hitler\'s book. Chapter 12, probably written in April/May 1925, deals with how the newly refounded NSDAP should rebuild as a party and amplify its program.</p> <p>\"According to their own account, the writers took parts of the chapter and inserted feminist \"buzzwords\"; they \"significantly changed\" the \"original wording and intent” of the text to make the paper \"publishable and about feminism.\" An observant reader might ask: what could possibly remain of any Nazi content after that? But no one in the media, apparently, did.\"</p> <p>New Discourses, \"There Is No Good Part of Hitler\'s Mein Kampf\" <a href=\"https://newdiscourses.com/2021/03/there-is-no-good-part-of-hitlers-mein-kampf/\"> https://newdiscourses.com/2021/03/there-is-no-good-part-of-hitlers-mein-kampf/</a></p> <p>On this episode of the New Discourses Podcast, James Lindsay, who helped to write the paper and perpetrate the Grievance Studies Affair, talks about the project and the creation of this particular paper at unprecedented length and in unprecedented detail, revealing Nilssen not to know what he’s talking about. If you have ever wondered about what the backstory of the creation of the “Feminist Mein Kampf” paper really was, including why its authors did it, you won’t want to miss this long-form discussion and rare response to yet another underinformed critic of Lindsay, Boghossian, and Pluckrose’s work.</p> <p>The Grieveance Studies Affair Revealed. <a href=\"https://www.youtube.com/watch?v=kVk9a5Jcd1k\">https://www.youtube.com/watch?v=kVk9a5Jcd1k</a></p> <p>Reviewer 1 Comments on Dog Park Paper</p> <p>\"page 9 - the human subjects are afforded anonymity and not asked about income, etc for ethical reasons. yet, the author as researcher intruded into the dogs\' spaces to examine and record genitalia. I realize this was necessary to the project, but could the author acknowledge/explain/justify this (arguably, anthropocentric) difference? Indicating that it was necessary to the research would suffice but at least the difference should be acknowledged.\"</p> <p>Nestor de Buen, Anti-Science Humping in the Dog Park. <a href=\"https://conceptualdisinformation.substack.com/p/anti-science-humping-in-the-dog-park\"> https://conceptualdisinformation.substack.com/p/anti-science-humping-in-the-dog-park</a></p> <p>\"What is even more striking is that if the research had actually been conducted and the results showed what the paper says they show, there is absolutely no reason why it should not have been published. And moreover, what it proves is the opposite of what its intention is. It shows that one can make scientifically testable claims based on the conceptual framework of gender studies, and that the field has all the markings of a perfectly functional research programme.\"</p> <p>\"Yes, the dog park paper is based on false data and, like Sokal’s, contains a lot of unnecessary jargon, but it is not nonsense, and the distinction is far from trivial. Nonsense implies one cannot even obtain a truth value from a proposition. In fact, the paper being false, if anything, proves that it is not nonsense, yet the grievance hoaxers try to pass falsity as nonsense. Nonsense is something like Chomsky’s famous sentence “colorless green ideas sleep furiously.” It is nonsense because it is impossible to decide how one might evaluate whether it is true. A false sentence would be “the moon is cubical.” It has a definite meaning, it just happens not to be true.&nbsp;</p> <p>\"So, if the original Sokal Hoax is like Chomsky’s sentence, the dog park paper is much more like “the moon is cubical.” And in fact, a more accurate analogy would be “the moon is cubical and here is a picture that proves it,” and an attached doctored picture of the cubical moon.\"</p> <p>Reviewer 2 Comments on the Dog-Park Paper</p> <p>\"I am a bit curious about your methodology. Can you say more? You describe your methods here (procedures for collecting data), but not really your overall approach to methodology. Did you just show up, observe, write copious notes, talk to people when necessary, and then leave? If so, it might be helpful to explicitly state this. It sounds to me like you did a kind of ethnography (methodology — maybe multispecies ethnography?) but that’s not entirely clear here. Or are you drawing on qualitative methods in social behaviorism/symbolic interactionism? In either case, the methodology chosen should be a bit more clearly articulated.\"</p> <p>Counterweight. <a href=\"https://counterweightsupport.com/\">https://counterweightsupport.com/</a></p> <p>\"Welcome to Counterweight, the home of scholarship and advice on [Critical Social Justice](<a href=\"https://counterweightsupport.com/2021/02/17/what-do-we-mean-by-critical-social-justice/\">https://counterweightsupport.com/2021/02/17/what-do-we-mean-by-critical-social-justice/</a>) ideology. We are here to connect you with the resources, advice and guidance you need to address CSJ beliefs as you encounter them in your day-to-day life. The Counterweight community is a non-partisan, grassroots movement advocating for liberal concepts of social justice including individualism, universalism, viewpoint diversity and the free exchange of ideas. [Subscribe](https://counterweightsupport.com/subscribe-to-counterweight/) today to become part of the Counterweight movement.\"\"</p> <p>Inside Higher Ed, \"Blowback Against a Hoax.\" <a href=\"https://www.insidehighered.com/news/2019/01/08/author-recent-academic-hoax-faces-disciplinary-action-portland-state\"> https://www.insidehighered.com/news/2019/01/08/author-recent-academic-hoax-faces-disciplinary-action-portland-state</a></p> <p>Peter Boghossian Resignation Latter from PSU. <a href=\"https://bariweiss.substack.com/p/my-university-sacrificed-ideas-for\"> https://bariweiss.substack.com/p/my-university-sacrificed-ideas-for</a></p> <p>&nbsp;</p>";

        let expected = "So, in rank defiance of our recent promise to 'get back to the nazis' instead we continue our James Lindsay coverage.\u{a0} (What... me? Irony? How dare you?)\u{a0} This time, Daniel patiently walks a distracted, slightly hyperactive, and increasingly incredulous Jack through the infamous 'Grievance Studies Hoax' (AKA 'Sokal Squared') in which Lindsay and colleagues Helen Pluckrose and Peter Boghossian tried (and then claimed) to prove something or other about modern Humanities academia by submitting a load of stupid fake papers to various feminist and fat studies journals.\u{a0} As Daniel reveals, the episode was an orgy of dishonesty and tactical point-missing that actually proved the opposite of what the team of snickering tricksters thought they were proving.\u{a0} Sadly, however, because we live in Hell, the trio have only raised their profiles as a result.\u{a0} A particular highlight of the episode is Lindsay revealing his staggering ignorance when 'responding' to criticism.\n\nContent warnings, as ever.\n\nPodcast Notes:\n\nPlease consider donating to help us make the show and stay independent.\u{a0} Patrons get exclusive access to one full extra episode a month.\n\nDaniel's Patreon: <a href=\"https://www.patreon.com/danielharper\">https://www.patreon.com/danielharper</a>\n\nJack's Patreon: <a href=\"https://www.patreon.com/user?u=4196618&amp;fan_landing=true\">https://www.patreon.com/user?u=4196618</a>\n\nIDSG Twitter: <a href=\"https://twitter.com/idsgpod\">https://twitter.com/idsgpod</a>\n\nDaniel's Twitter: <a href=\"https://twitter.com/danieleharper\">@danieleharper</a>\n\nJack's Twitter: <a href=\"https://twitter.com/_Jack_Graham_\">@_Jack_Graham_</a>\n\nIDSG on Apple Podcasts: <a href=\"https://podcasts.apple.com/us/podcast/i-dont-speak-german/id1449848509?ls=1\"> https://podcasts.apple.com/us/podcast/i-dont-speak-german/id1449848509?ls=1</a>\n\n\n\nShow Notes:\n\nJames Lindsay, New Discourses, \"Why You Can Be Transgender But Not Transracial.\"\" <a href=\"https://newdiscourses.com/2021/06/why-you-can-be-transgender-but-not-transracial/\"> https://newdiscourses.com/2021/06/why-you-can-be-transgender-but-not-transracial/</a>\n\nJames Lindsay has a day job, apparently. \"Maryville man walks path of healing and combat.\" <a href=\"https://www.thedailytimes.com/news/maryville-man-walks-path-of-healing-and-combat/article_5ea3c0ca-2e98-5283-9e59-06861b8588cb.html\"> https://www.thedailytimes.com/news/maryville-man-walks-path-of-healing-and-combat/article_5ea3c0ca-2e98-5283-9e59-06861b8588cb.html</a>\n\nAreo Magazine, Academic Grievance Studies and the Corruption of Scholarship. <a href=\"https://areomagazine.com/2018/10/02/academic-grievance-studies-and-the-corruption-of-scholarship/\"> https://areomagazine.com/2018/10/02/academic-grievance-studies-and-the-corruption-of-scholarship/</a>\n\nFull listing of Grievance Studies Papers and Reviews. <a href=\"https://drive.google.com/drive/folders/19tBy_fVlYIHTxxjuVMFxh4pqLHM_en18\"> https://drive.google.com/drive/folders/19tBy_fVlYIHTxxjuVMFxh4pqLHM_en18</a>\n\n'Mein Kampf' and the 'Feminazis': What Three Academics' Hitler Hoax Really Reveals About 'Wokeness'. <a href=\"https://web.archive.org/web/20210328112901/https://www.haaretz.com/us-news/.premium-hitler-hoax-academic-wokeness-culture-war-1.9629759\"> https://web.archive.org/web/20210328112901/https://www.haaretz.com/us-news/.premium-hitler-hoax-academic-wokeness-culture-war-1.9629759</a>\n\n\"First and foremost, the source material. The chapter the hoaxers chose, not by coincidence, one of the least ideological and racist parts of Hitler's book. Chapter 12, probably written in April/May 1925, deals with how the newly refounded NSDAP should rebuild as a party and amplify its program.\n\n\"According to their own account, the writers took parts of the chapter and inserted feminist \"buzzwords\"; they \"significantly changed\" the \"original wording and intent” of the text to make the paper \"publishable and about feminism.\" An observant reader might ask: what could possibly remain of any Nazi content after that? But no one in the media, apparently, did.\"\n\nNew Discourses, \"There Is No Good Part of Hitler's Mein Kampf\" <a href=\"https://newdiscourses.com/2021/03/there-is-no-good-part-of-hitlers-mein-kampf/\"> https://newdiscourses.com/2021/03/there-is-no-good-part-of-hitlers-mein-kampf/</a>\n\nOn this episode of the New Discourses Podcast, James Lindsay, who helped to write the paper and perpetrate the Grievance Studies Affair, talks about the project and the creation of this particular paper at unprecedented length and in unprecedented detail, revealing Nilssen not to know what he’s talking about. If you have ever wondered about what the backstory of the creation of the “Feminist Mein Kampf” paper really was, including why its authors did it, you won’t want to miss this long-form discussion and rare response to yet another underinformed critic of Lindsay, Boghossian, and Pluckrose’s work.\n\nThe Grieveance Studies Affair Revealed. <a href=\"https://www.youtube.com/watch?v=kVk9a5Jcd1k\">https://www.youtube.com/watch?v=kVk9a5Jcd1k</a>\n\nReviewer 1 Comments on Dog Park Paper\n\n\"page 9 - the human subjects are afforded anonymity and not asked about income, etc for ethical reasons. yet, the author as researcher intruded into the dogs' spaces to examine and record genitalia. I realize this was necessary to the project, but could the author acknowledge/explain/justify this (arguably, anthropocentric) difference? Indicating that it was necessary to the research would suffice but at least the difference should be acknowledged.\"\n\nNestor de Buen, Anti-Science Humping in the Dog Park. <a href=\"https://conceptualdisinformation.substack.com/p/anti-science-humping-in-the-dog-park\"> https://conceptualdisinformation.substack.com/p/anti-science-humping-in-the-dog-park</a>\n\n\"What is even more striking is that if the research had actually been conducted and the results showed what the paper says they show, there is absolutely no reason why it should not have been published. And moreover, what it proves is the opposite of what its intention is. It shows that one can make scientifically testable claims based on the conceptual framework of gender studies, and that the field has all the markings of a perfectly functional research programme.\"\n\n\"Yes, the dog park paper is based on false data and, like Sokal’s, contains a lot of unnecessary jargon, but it is not nonsense, and the distinction is far from trivial. Nonsense implies one cannot even obtain a truth value from a proposition. In fact, the paper being false, if anything, proves that it is not nonsense, yet the grievance hoaxers try to pass falsity as nonsense. Nonsense is something like Chomsky’s famous sentence “colorless green ideas sleep furiously.” It is nonsense because it is impossible to decide how one might evaluate whether it is true. A false sentence would be “the moon is cubical.” It has a definite meaning, it just happens not to be true.\u{a0}\n\n\"So, if the original Sokal Hoax is like Chomsky’s sentence, the dog park paper is much more like “the moon is cubical.” And in fact, a more accurate analogy would be “the moon is cubical and here is a picture that proves it,” and an attached doctored picture of the cubical moon.\"\n\nReviewer 2 Comments on the Dog-Park Paper\n\n\"I am a bit curious about your methodology. Can you say more? You describe your methods here (procedures for collecting data), but not really your overall approach to methodology. Did you just show up, observe, write copious notes, talk to people when necessary, and then leave? If so, it might be helpful to explicitly state this. It sounds to me like you did a kind of ethnography (methodology — maybe multispecies ethnography?) but that’s not entirely clear here. Or are you drawing on qualitative methods in social behaviorism/symbolic interactionism? In either case, the methodology chosen should be a bit more clearly articulated.\"\n\nCounterweight. <a href=\"https://counterweightsupport.com/\">https://counterweightsupport.com/</a>\n\n\"Welcome to Counterweight, the home of scholarship and advice on [Critical Social Justice](<a href=\"https://counterweightsupport.com/2021/02/17/what-do-we-mean-by-critical-social-justice/\">https://counterweightsupport.com/2021/02/17/what-do-we-mean-by-critical-social-justice/</a>) ideology. We are here to connect you with the resources, advice and guidance you need to address CSJ beliefs as you encounter them in your day-to-day life. The Counterweight community is a non-partisan, grassroots movement advocating for liberal concepts of social justice including individualism, universalism, viewpoint diversity and the free exchange of ideas. [Subscribe](<a href=\"https://counterweightsupport.com/subscribe-to-counterweight/\">https://counterweightsupport.com/subscribe-to-counterweight/</a>) today to become part of the Counterweight movement.\"\"\n\nInside Higher Ed, \"Blowback Against a Hoax.\" <a href=\"https://www.insidehighered.com/news/2019/01/08/author-recent-academic-hoax-faces-disciplinary-action-portland-state\"> https://www.insidehighered.com/news/2019/01/08/author-recent-academic-hoax-faces-disciplinary-action-portland-state</a>\n\nPeter Boghossian Resignation Latter from PSU. <a href=\"https://bariweiss.substack.com/p/my-university-sacrificed-ideas-for\"> https://bariweiss.substack.com/p/my-university-sacrificed-ideas-for</a>\n\n\n\n";
        let markup = html2pango_markup(description);

        assert_eq!(expected, markup);
    }

    #[test]
    fn test_newline_based() {
        let description = "Also available in video form at https://youtu.be/NUPWY_evu30\n\
\n\
In a recent view by Contrapoints, she goes over her account of envy and its connection with online politics. In doing so she utilizes Nietzsche (alongside a critique of Nietzsche). How accurate is this account to Nietzsche\'s work and where does it go wrong?  \n\
\n\
Thank you to We\'re in Hell, BadEmpanada, and Chelsea Manning for the voice lines! \n\
\n\
Edited by Lexi Fontaine: https://twitter.com/softgothoutlaw \n\
\n\
Music by Alex Ballantyne: https://transistorriot.bandcamp.com \n\
\n\
This was an early release to my patrons at https://pateron.com/livagar \n\
\n\
Watch me stream on twitch at https://twitch.tv/livagar \n\
\n\
All of my links at https:// livagar.com";

        let expected = "Also available in video form at <a href=\"https://youtu.be/NUPWY_evu30\">https://youtu.be/NUPWY_evu30</a>\n\
\n\
In a recent view by Contrapoints, she goes over her account of envy and its connection with online politics. In doing so she utilizes Nietzsche (alongside a critique of Nietzsche). How accurate is this account to Nietzsche's work and where does it go wrong? \n\
\n\
Thank you to We're in Hell, BadEmpanada, and Chelsea Manning for the voice lines! \n\
\n\
Edited by Lexi Fontaine: <a href=\"https://twitter.com/softgothoutlaw\">https://twitter.com/softgothoutlaw</a> \n\
\n\
Music by Alex Ballantyne: <a href=\"https://transistorriot.bandcamp.com\">https://transistorriot.bandcamp.com</a> \n\
\n\
This was an early release to my patrons at <a href=\"https://pateron.com/livagar\">https://pateron.com/livagar</a> \n\
\n\
Watch me stream on twitch at <a href=\"https://twitch.tv/livagar\">https://twitch.tv/livagar</a> \n\
\n\
All of my links at https:// <a href=\"https:livagar.com\">livagar.com</a>";
        let markup = html2pango_markup(description);

        assert_eq!(expected, markup);
    }

    #[test]
    fn test_newline_based2() {
        let description = "We’re back to a normal-style ep after a week of interviews. We’re taking a look at the fast-tracked aid package to intelligence agents suffering unreality issues, the Biden administration addressing just the optics at the border, and AOC addressing just the optics of the Iron Dome bill. Finally, we having a reading series that functions as a bit of a coda to Will and Matt’s visit to Ozy Fest way back in 2018.\n\nOne last time, go subscribe to https://www.youtube.com/chapotraphouse\n\nAnd go grab some of Simon Roy’s great posters over at https://shop.chapotraphouse.com/\nMore merch coming soon!";
        let expected = "We’re back to a normal-style ep after a week of interviews. We’re taking a look at the fast-tracked aid package to intelligence agents suffering unreality issues, the Biden administration addressing just the optics at the border, and AOC addressing just the optics of the Iron Dome bill. Finally, we having a reading series that functions as a bit of a coda to Will and Matt’s visit to Ozy Fest way back in 2018.\n\nOne last time, go subscribe to <a href=\"https://www.youtube.com/chapotraphouse\">https://www.youtube.com/chapotraphouse</a>\n\nAnd go grab some of Simon Roy’s great posters over at <a href=\"https://shop.chapotraphouse.com/\">https://shop.chapotraphouse.com/</a>\nMore merch coming soon!";
        let markup = html2pango_markup(description);

        assert_eq!(expected, markup);
    }

    #[test]
    fn test_list_ordered() {
        let description = "<ol><li>first</li><li>second</li><li>third</li></ol>";
        let expected = "\n    1. first\n    2. second\n    3. third\n\n";
        let markup = html2pango_markup(description);

        assert_eq!(expected, markup);
    }

    #[test]
    fn test_list_unordered() {
        let description = "<ul><li>first</li><li>second</li><li>third</li></ul>";
        let expected = "\n    • first\n    • second\n    • third\n\n";
        let markup = html2pango_markup(description);

        assert_eq!(expected, markup);
    }

    #[test]
    fn test_list_ordered_nested() {
        let description = "<ol><li>first</li><li>second<ol><li> sub list first</li><li> sub list second</li></li></ol><li>third</li></ol>";
        let expected = "\n    1. first\n    2. second\n        1. sub list first\n        2. sub list second\n\n\n    3. third\n\n";
        let markup = html2pango_markup(description);

        assert_eq!(expected, markup);
    }

    #[test]
    fn test_timecode() {
        let description = "<ul>\
  <li>Twitter: <a href=\"https://twitter.com/rustaceanfm\">@rustaceanfm</a></li>\
  <li>Discord: <a href=\"https://discord.gg/cHc3Gyc\">Rustacean Station</a></li>\
  <li>Github: <a href=\"https://github.com/rustacean-station/\">@rustacean-station</a></li>\
  <li>Email: <a href=\"mailto:hello@rustacean-station.org\">hello@rustacean-station.org</a></li>\
</ul>\
\
<h2 id=\"timestamps\">Timestamps</h2>\
<ul>\
  <li>[@0:33] - Daniel’s introduction</li>\
  <li>[@3:38] - Tauri’s focus on safety and security</li>\
  <li>[@6:50] - Tauri’s mission to reduce their footprint</li>\
  <li>[@14:48] - How does Tauri handles features that are not supported across different platforms</li>\
  <li>[@23:56] - How does Tauri monetize to keep the project going?</li>\
  <li>[@26:16] - Why choose Tauri over other solutions?</li>\
  <li>[@28:57] - What are the tools being built with Tauri?</li>\
  <li>[@31:09] - Tyler’s programming background</li>\
  <li>[@35:11] - Tauri’s future release and features</li>\
  <li>[@38:38] - ‘Tauri Foundations’ book by Daniel Thompson-Yvetot and Lucas Nogueira</li>\
  <li>[@40:00] - Requirement on building a Tauri app</li>\
  <li>[@43:13] - Parting thoughts</li>\
</ul>";
        let expected = "\n\
\x20   • Twitter: <a href=\"https://twitter.com/rustaceanfm\">@rustaceanfm</a>\n\
\x20   • Discord: <a href=\"https://discord.gg/cHc3Gyc\">Rustacean Station</a>\n\
\x20   • Github: <a href=\"https://github.com/rustacean-station/\">@rustacean-station</a>\n\
\x20   • Email: <a href=\"mailto:hello@rustacean-station.org\">hello@rustacean-station.org</a>\n\
\n\
Timestamps\n\
\x20   • [@<a href=\"jump:33\" title=\"Jump to 00:33\">0:33</a>] - Daniel’s introduction\n\
\x20   • [@<a href=\"jump:218\" title=\"Jump to 03:38\">3:38</a>] - Tauri’s focus on safety and security\n\
\x20   • [@<a href=\"jump:410\" title=\"Jump to 06:50\">6:50</a>] - Tauri’s mission to reduce their footprint\n\
\x20   • [@<a href=\"jump:888\" title=\"Jump to 14:48\">14:48</a>] - How does Tauri handles features that are not supported across different platforms\n\
\x20   • [@<a href=\"jump:1436\" title=\"Jump to 23:56\">23:56</a>] - How does Tauri monetize to keep the project going?\n\
\x20   • [@<a href=\"jump:1576\" title=\"Jump to 26:16\">26:16</a>] - Why choose Tauri over other solutions?\n\
\x20   • [@<a href=\"jump:1737\" title=\"Jump to 28:57\">28:57</a>] - What are the tools being built with Tauri?\n\
\x20   • [@<a href=\"jump:1869\" title=\"Jump to 31:09\">31:09</a>] - Tyler’s programming background\n\
\x20   • [@<a href=\"jump:2111\" title=\"Jump to 35:11\">35:11</a>] - Tauri’s future release and features\n\
\x20   • [@<a href=\"jump:2318\" title=\"Jump to 38:38\">38:38</a>] - ‘Tauri Foundations’ book by Daniel Thompson-Yvetot and Lucas Nogueira\n\
\x20   • [@<a href=\"jump:2400\" title=\"Jump to 40:00\">40:00</a>] - Requirement on building a Tauri app\n\
\x20   • [@<a href=\"jump:2593\" title=\"Jump to 43:13\">43:13</a>] - Parting thoughts\n\
\n\
";
        let markup = html2pango_markup(description);
        assert_eq!(expected, markup);
    }

    #[test]
    fn test_escape_lt_gt_and_img() {
        let description = "<p>\
<a href=\"http://faif.us/cast-media/FaiF_FOSSY-2023.ogg\"><img alt=\"[Direct download of cast in Ogg/Vorbis\n\
                                          format]\" src=\"http://faif.us/img/cast/audio_ogg_button.png\"></a>\n\
<a href=\"http://faif.us/cast-media/FaiF_FOSSY-2023.mp3\"><img alt=\"[Direct download of cast in MP3 format]\" src=\"http://faif.us/img/cast/audio_mp3_button.png\"></a>\n\
</p>\n\
<p>\n\
Come to <a href=\"https://2023.fossy.us/\">FOSSY 2023</a>!\n\
</p>\n\
<h3>Show Notes:</h3>\n\
\n\
<a href=\"https://2023.fossy.us/\">FOSSY 2023</a> will happen next week in Portland, OR, USA.\n\
\n\
<hr width=\"80%\">\n\
\n\
<p>Send feedback and comments on the cast\n\
to <a href=\"mailto:cast@faif.us\">&lt;oggcast@faif.us&gt;</a>.\n\
You can keep in touch with <a href=\"http://faif.us\">Free as in Freedom</a>\n\
  by <a href=\"https://twitter.com/conservancy\">following Conservancy on\n\
    on Twitter</a> and <a href=\"https://twitter.com/faifcast\">and FaiF on Twitter</a>.  We are working on setting up a group chat again, too!</p>\n\
\n\
<p>Free as in Freedom is produced by <a href=\"http://danlynch.org/blog/\">Dan Lynch</a>\n\
  of <a href=\"http://danlynch.org/\">danlynch.org</a>.\n\
Theme\n\
  music written and performed\n\
  by <a href=\"http://www.miketarantino.com\">Mike Tarantino</a>\n\
  with <a href=\"http://www.charliepaxson.com\">Charlie Paxson</a> on drums.</p>\n\
\n\
<p><a href=\"https://creativecommons.org/licenses/by-sa/4.0/\"><img alt=\"Creative Commons License\" src=\"http://i.creativecommons.org/l/by-sa/4.0/88x31.png\"></a>\n\
   The content\n\
   of <span>this\n\
   audcast</span>, and the accompanying show notes and music are licensed\n\
   under the <a href=\"https://creativecommons.org/licenses/by-sa/4.0/\">Creative\n\
   Commons Attribution-Share-Alike 4.0 license (CC BY-SA 4.0)</a>.\n\
   </p>";
        let expected = "<a href=\"http://faif.us/cast-media/FaiF_FOSSY-2023.ogg\">[[Direct download of cast in Ogg/Vorbis format]]\n\
</a><a href=\"http://faif.us/cast-media/FaiF_FOSSY-2023.mp3\">[[Direct download of cast in MP3 format]]\n\
</a>\n\
\n\
Come to <a href=\"https://2023.fossy.us/\">FOSSY 2023</a>! \n\
\n\
Show Notes: <a href=\"https://2023.fossy.us/\">FOSSY 2023</a> will happen next week in Portland, OR, USA.  Send feedback and comments on the cast to <a href=\"mailto:cast@faif.us\">&lt;oggcast@faif.us&gt;</a>. You can keep in touch with <a href=\"http://faif.us\">Free as in Freedom</a> by <a href=\"https://twitter.com/conservancy\">following Conservancy on on Twitter</a> and <a href=\"https://twitter.com/faifcast\">and FaiF on Twitter</a>. We are working on setting up a group chat again, too!\n\
\n\
Free as in Freedom is produced by <a href=\"http://danlynch.org/blog/\">Dan Lynch</a> of <a href=\"http://danlynch.org/\">danlynch.org</a>. Theme music written and performed by <a href=\"http://www.miketarantino.com\">Mike Tarantino</a> with <a href=\"http://www.charliepaxson.com\">Charlie Paxson</a> on drums.\n\
\n\
<a href=\"https://creativecommons.org/licenses/by-sa/4.0/\">[Creative Commons License]\n\
</a>The content of this audcast, and the accompanying show notes and music are licensed under the <a href=\"https://creativecommons.org/licenses/by-sa/4.0/\">Creative Commons Attribution-Share-Alike 4.0 license (CC BY-SA 4.0)</a>. \n\n";

        let markup = html2pango_markup(description);
        assert_eq!(expected, markup);
    }

    #[test]
    fn test_hash_invalid_link() {
        let description = r#"<p><a href=" #whatever">🤷</a></p>"#;
        let expected = "🤷\n\n";
        let markup = html2pango_markup(description);

        assert_eq!(expected, markup);
    }

    #[test]
    fn test_empty_link() {
        let description = r#"<p><a href="">🤷</a></p>"#;
        let expected = "🤷\n\n";
        let markup = html2pango_markup(description);

        assert_eq!(expected, markup);
    }

    #[test]
    fn test_hash_timestamp_link() {
        let description = r##"<p><a href="#t=13:12">13:12</a></p>"##;
        let expected = "<a href=\"jump:792\" title=\"Jump to 13:12\">13:12</a>\n\n";
        let markup = html2pango_markup(description);

        assert_eq!(expected, markup);
    }
}
