use chrono::NaiveDate;
use comrak::plugins::syntect::SyntectAdapter;
use comrak::{markdown_to_html_with_plugins, ComrakOptions, ComrakPlugins};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::Path;
use std::slice::Iter;
use tinytemplate::{format_unescaped, TinyTemplate};

mod templates;

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq)]
pub enum Tag {
    Nix,
}

impl Tag {
    pub fn iterator() -> Iter<'static, Tag> {
        static TAGS: [Tag; 1] = [Tag::Nix];
        TAGS.iter()
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tag = match self {
            Tag::Nix => "Nix",
        };
        write!(f, "{}", tag)
    }
}

#[derive(Clone, Serialize, Debug)]
struct TagPage<'a> {
    tag: Tag,
    posts: Vec<Post>,
    config: &'a Config,
}

#[derive(Clone, Deserialize, Serialize, Debug)]
struct Post {
    title: String,
    published: NaiveDate,
    slug: String,
    tags: Vec<Tag>,
    content: String,
    #[serde(default = "default_read_time")]
    read_time: usize,
}

#[derive(Clone, Deserialize, Serialize, Debug)]
struct PostPage {
    post: Post,
    config: Config,
}

#[derive(Clone, Deserialize, Serialize, Debug)]
struct Config {
    drv: String,
    build_dir: String,
    repo: String,
}

#[derive(Clone, Serialize, Debug)]
struct Index<'a> {
    posts: &'a Vec<Post>,
    config: &'a Config,
}

fn default_read_time() -> usize {
    0
}

fn estimate_read_time(s: &String) -> usize {
    let wpm = 200;
    let mut total_words = 0;
    let mut previous_char = char::MAX;
    for chr in s.chars() {
        if previous_char.is_ascii_whitespace()
            && (chr.is_ascii_alphabetic() || chr.is_ascii_digit() || chr.is_ascii_punctuation())
        {
            total_words += 1;
        }
        previous_char = chr;
    }

    let quotient = total_words / wpm;
    let remainder = total_words % wpm;

    if remainder < (wpm / 2) {
        quotient
    } else {
        quotient + 1
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let config: Config = serde_dhall::from_file("./dhall/config.dhall").parse()?;

    if Path::new(&config.build_dir).exists() {
        fs::remove_dir_all(&config.build_dir)?;
    }
    fs::create_dir(&config.build_dir)?;

    let mut tt = TinyTemplate::new();
    tt.set_default_formatter(&format_unescaped);
    tt.add_template("index", templates::INDEX)?;
    tt.add_template("post", templates::POST)?;
    tt.add_template("tag", templates::TAG)?;
    tt.add_template("footer", templates::FOOTER)?;

    let mut options = ComrakOptions::default();
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.header_ids = Some("".to_string());
    options.extension.footnotes = true;
    options.render.github_pre_lang = true;
    options.render.escape = true;

    let adapter = SyntectAdapter::new("base16-eighties.dark");
    let mut plugins = ComrakPlugins::default();
    plugins.render.codefence_syntax_highlighter = Some(&adapter);

    let paths = fs::read_dir("./dhall/hord")?;

    let mut posts: Vec<Post> = Vec::new();
    for path in paths {
        let mut post: Post = serde_dhall::from_file(path.unwrap().path()).parse()?;
        post.read_time = estimate_read_time(&post.content);
        post.content = markdown_to_html_with_plugins(&post.content, &options, &plugins);
        posts.push(post);
    }
    posts.sort_by(|a, b| b.published.cmp(&a.published));

    let index = Index {
        posts: &posts,
        config: &config,
    };
    let rendered_index = tt.render("index", &index)?;
    fs::write(format!("{}/index.html", config.build_dir), rendered_index)?;

    fs::create_dir(format!("{}/gewritu", &config.build_dir))?;
    for post in &posts {
        let post_page: PostPage = PostPage {
            post: post.clone(),
            config: config.clone(),
        };
        let rendered = tt.render("post", &post_page)?;
        fs::write(
            format!("{}/gewritu/{}.html", &config.build_dir, post.slug),
            rendered,
        )?;
    }

    fs::create_dir(format!("{}/tags", &config.build_dir))?;
    for tag in Tag::iterator() {
        let tag_page = TagPage {
            tag: tag.clone(),
            posts: posts
                .clone()
                .into_iter()
                .filter(|p| p.tags.contains(tag))
                .collect(),
            config: &config,
        };
        let rendered_tag = tt.render("tag", &tag_page)?;
        fs::write(
            format!("{}/tags/{}.html", config.build_dir, tag),
            rendered_tag,
        )?;
    }

    Ok(())
}
