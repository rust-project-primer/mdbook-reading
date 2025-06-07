use anyhow::{Context as _, Result, bail};
use log::*;
use mdbook::{
    BookItem,
    book::{Book, Chapter},
    errors::Result as MdbookResult,
    preprocess::{Preprocessor, PreprocessorContext},
};
use pulldown_cmark::{CodeBlockKind, CowStr, Event, Options, Parser, Tag, TagEnd};
use pulldown_cmark_to_cmark::cmark;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
};
use tera::Tera;
use toml::value::Value;
use url::Url;

#[derive(Deserialize, Debug)]
pub struct Config {
    /// Base path where archives are stored.
    archives: Option<String>,
}

#[derive(Debug)]
pub struct Instance {
    templates: Tera,
    config: Config,
}

impl Instance {
    pub fn new(config: Config) -> Self {
        let mut templates = Tera::default();
        templates
            .add_raw_template("output", include_str!("output.tera"))
            .unwrap();
        Self { templates, config }
    }

    fn map(&self, book: Book) -> Result<Book> {
        let mut book = book;
        book.sections = std::mem::take(&mut book.sections)
            .into_iter()
            .map(|section| self.map_book_item(section))
            .collect::<Result<_, _>>()?;
        Ok(book)
    }

    fn map_book_item(&self, item: BookItem) -> Result<BookItem> {
        let result = match item {
            BookItem::Chapter(chapter) => BookItem::Chapter(self.map_chapter(chapter)?),
            other => other,
        };

        Ok(result)
    }

    fn map_chapter(&self, mut chapter: Chapter) -> Result<Chapter> {
        chapter.content = self.map_markdown(&chapter.content)?;
        chapter.sub_items = std::mem::take(&mut chapter.sub_items)
            .into_iter()
            .map(|item| self.map_book_item(item))
            .collect::<Result<_, _>>()?;
        Ok(chapter)
    }

    fn map_markdown(&self, markdown: &str) -> Result<String> {
        let mut parser = Parser::new_ext(markdown, Options::all());
        let mut events = vec![];

        loop {
            let next = parser.next();
            match next {
                None => break,
                Some(Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(label))))
                    if &*label == "reading" =>
                {
                    let mapped = match parser.next() {
                        Some(Event::Text(code)) => {
                            self.map_code(&*label, code).context("Mapping code")?
                        }
                        other => unreachable!("Got {other:?}"),
                    };

                    for event in mapped.into_iter() {
                        events.push(event);
                    }

                    parser.next();
                }
                Some(event) => events.push(event),
            }
        }

        let mut buf = String::with_capacity(markdown.len());
        let output = cmark(events.iter(), &mut buf).map(|_| buf)?;
        Ok(output)
    }

    fn map_code(&self, label: &str, code: CowStr<'_>) -> Result<Vec<Event<'static>>> {
        let (header, content) = code.split_once("---").unwrap();
        let header: Header = serde_yaml::from_str(header)?;

        let title = header.title(&self.config);

        let mut events: Vec<Event<'static>> = Vec::new();
        events.push(Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(title.into()))));
        events.push(Event::Text(content.to_string().into()));
        events.push(Event::End(TagEnd::CodeBlock));
        Ok(events)
    }

}

#[derive(Deserialize, Debug)]
pub struct Header {
    style: String,
    title: String,
    author: String,
    url: Url,
    archived: Option<String>,
}

impl Header {
    pub fn title(&self, config: &Config) -> String {
        let Self { style, title, author, url, archived } = &self;
        let mut title = format!("<a href='{url}'>{title}</a>");
        if let Some(archived) = &archived {
            let prefix = config.archives.as_deref().unwrap_or("");
            let archived = format!("{prefix}{archived}");
            title.push_str(&format!(" (<a href='{archived}'>archived</a>)"));
        }
        title.push_str(&format!(" by {author}"));
        let output = format!("admonish {style} title=\"{title}\"");
        output
    }
}

#[derive(Clone, Debug, Default)]
pub struct ReadingPreprocessor;

impl Preprocessor for ReadingPreprocessor {
    fn name(&self) -> &str {
        "reading"
    }

    fn run(&self, ctx: &PreprocessorContext, book: Book) -> MdbookResult<Book> {
        let config = ctx.config.get_preprocessor(self.name()).unwrap();
        let config: Config = Value::Table(config.clone()).try_into().unwrap();
        let instance = Instance::new(config);
        instance.map(book)
    }
}
