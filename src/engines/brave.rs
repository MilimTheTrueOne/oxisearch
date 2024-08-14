//! The `brave` module handles the scraping of results from the brave search engine
//! by querying the upstream brave search engine with user provided query and with a page
//! number if provided.

use std::collections::HashMap;

use reqwest::{header::HeaderMap, Client};
use scraper::Html;

use crate::models::aggregation_models::SearchResult;
use error_stack::{Report, Result, ResultExt};

use crate::models::engine_models::{EngineError, SearchEngine};

use super::search_result_parser::SearchResultParser;

/// Scrapes the results from the Brave search engine.  
pub struct Brave {
    /// Utilises generic logic for parsing search results.
    parser: SearchResultParser,
}

impl Brave {
    /// Creates the Brave parser.
    pub fn new() -> Result<Brave, EngineError> {
        Ok(Self {
            parser: SearchResultParser::new(
                "#results h4",
                "#results [data-pos]",
                "a > .url",
                "a",
                ".snippet-description",
            )?,
        })
    }
}

#[async_trait::async_trait]
impl SearchEngine for Brave {
    async fn results(
        &self,
        query: &str,
        page: u32,
        user_agent: &str,
        client: &Client,
    ) -> Result<Vec<(String, SearchResult)>, EngineError> {
        let url = format!("https://search.brave.com/search?q={query}&offset={page}");

        let header_map = HeaderMap::try_from(&HashMap::from([
            ("User-Agent".to_string(), user_agent.to_string()),
            (
                "Content-Type".to_string(),
                "application/x-www-form-urlencoded".to_string(),
            ),
            ("Referer".to_string(), "https://google.com/".to_string()),
            ("Cookie".to_string(), "safe_search=off".into()),
        ]))
        .change_context(EngineError::UnexpectedError)?;

        let document: Html = Html::parse_document(
            &Brave::fetch_html_from_upstream(self, &url, header_map, client).await?,
        );

        if let Some(no_result_msg) = self.parser.parse_for_no_results(&document).nth(0) {
            if no_result_msg
                .inner_html()
                .contains("Not many great matches came back for your search")
            {
                return Err(Report::new(EngineError::EmptyResultSet));
            }
        }

        self.parser
            .parse_for_results(&document, |title, url, desc| {
                url.value().attr("href").map(|url| {
                    SearchResult::new(
                        title.text().collect::<Vec<_>>().join("").trim(),
                        url.trim(),
                        desc.inner_html().trim(),
                        &["brave"],
                    )
                })
            })
    }
}
