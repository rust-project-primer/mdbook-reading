use anyhow::{bail, Context as _, Result};
use log::*;
use mdbook::{
    book::{Book, Chapter},
    errors::Result as MdbookResult,
    preprocess::{Preprocessor, PreprocessorContext},
    BookItem,
};
use pulldown_cmark::{CodeBlockKind, CowStr, Event, Options, Parser, Tag};
use pulldown_cmark_to_cmark::cmark;
use serde::Deserialize;
use std::{collections::BTreeMap, fmt::Write};
use tera::Tera;
use toml::value::Value;

#[derive(Clone, Debug)]
pub struct ReadingPreprocessor {
    templates: Tera,
}

impl Default for ReadingPreprocessor {
    fn default() -> Self {
        Self::new()
    }
}

impl ReadingPreprocessor {
    pub fn new() -> Self {
        let mut templates = Tera::default();
        templates
            .add_raw_template("output", include_str!("output.tera"))
            .unwrap();
        Self { templates }
    }
}

impl Preprocessor for ReadingPreprocessor {
    fn name(&self) -> &str {
        "reading"
    }

    fn run(&self, ctx: &PreprocessorContext, book: Book) -> MdbookResult<Book> {
        Ok(book)
    }
}
