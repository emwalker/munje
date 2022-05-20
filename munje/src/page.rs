use anyhow::{anyhow, Error};
use scraper::{Html, Selector};
use url::{ParseError, Url};

pub struct Page {
    link: Url,
    html: Html,
}

impl Page {
    const LEETCODE_LOGO: &'static str =
        "https://assets.leetcode.com/static_assets/public/icons/favicon-96x96.png";
    const REDDIT_LOGO: &'static str =
        "https://www.redditstatic.com/desktop2x/img/favicon/favicon-96x96.png";

    pub fn parse(body: String, link: String) -> Result<Self, Error> {
        let html = Html::parse_fragment(body.as_ref());
        match Url::parse(&link) {
            Ok(parsed_link) => Ok(Self {
                link: parsed_link,
                html,
            }),
            Err(err) => Err(anyhow!("There was a problem: {}", err)),
        }
    }

    pub fn meta_image(&self) -> Option<Url> {
        match self.link.host_str() {
            Some("leetcode.com") => Some(Url::parse(Self::LEETCODE_LOGO).unwrap()),
            Some("www.reddit.com") => Some(Url::parse(Self::REDDIT_LOGO).unwrap()),
            Some("reddit.com") => Some(Url::parse(Self::REDDIT_LOGO).unwrap()),
            _ => {
                let sel = Selector::parse(r#"meta[property="og:image"]"#).unwrap();
                let result = self.html.select(&sel).next();
                match result {
                    None => None,
                    Some(element) => match element.value().attr("content") {
                        None => None,
                        Some(content) => self.extract_meta_image(content),
                    },
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn title(&self) -> Option<String> {
        let sel = Selector::parse("title").unwrap();
        self.html
            .select(&sel)
            .next()
            .map(|element| element.inner_html())
    }

    fn extract_meta_image(&self, content: &str) -> Option<Url> {
        let fetched = Url::parse(content);
        match fetched {
            Ok(fetched_link) => Some(fetched_link),
            Err(ParseError::RelativeUrlWithoutBase) => match self.link.host_str() {
                None => None,
                Some(parent_host) => {
                    let result = Url::parse(
                        format!("{}://{}{}", self.link.scheme(), parent_host, content).as_ref(),
                    );
                    match result {
                        Ok(url) => Some(url),
                        Err(err) => {
                            error!("Error parsing derived url: {}", err);
                            None
                        }
                    }
                }
            },
            Err(err) => {
                error!("Error parsing fetched url: {}", err);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(body: &str) -> Result<Page, Error> {
        Page::parse(body.to_string(), "https://parent/link".to_string())
    }

    #[actix_rt::test]
    async fn absolute_meta_image() -> Result<(), Error> {
        let page = Page::parse(
            r#"
            <head>
                <meta
                    property="og:image"
                    content="https://cdn.sstatic.net/Sites/stackoverflow/Img/apple-touch-icon@2.png"
                />
            </head>
            "#
            .to_string(),
            "https://stackoverflow.com/questions/66272984/parent-page".to_string(),
        )?;
        assert_eq!(
            "https://cdn.sstatic.net/Sites/stackoverflow/Img/apple-touch-icon@2.png".to_string(),
            page.meta_image().unwrap().to_string(),
        );
        Ok(())
    }

    #[actix_rt::test]
    async fn leetcode_meta_image() -> Result<(), Error> {
        let page = Page::parse(
            r#"
            <head>
                <meta property="og:image" content="/static/images/LeetCode_Sharing.png" />
            </head>
            "#
            .to_string(),
            "https://leetcode.com/problems/some-problem".to_string(),
        )?;
        assert_eq!(
            "https://assets.leetcode.com/static_assets/public/icons/favicon-96x96.png".to_string(),
            page.meta_image().unwrap().to_string(),
        );
        Ok(())
    }

    #[actix_rt::test]
    async fn reddit_meta_image() -> Result<(), Error> {
        let page = Page::parse(
            r#"<head></head>"#.to_string(),
            "https://www.reddit.com/r/AskHistorians/comments/qdzeai".to_string(),
        )?;
        assert_eq!(
            "https://www.redditstatic.com/desktop2x/img/favicon/favicon-96x96.png".to_string(),
            page.meta_image().unwrap().to_string(),
        );
        Ok(())
    }

    #[actix_rt::test]
    async fn no_meta_image_content() -> Result<(), Error> {
        let page = parse(r#"<head><meta property="og:image" contents="image-link">"#)?;
        assert_eq!(None, page.meta_image());
        Ok(())
    }

    #[actix_rt::test]
    async fn no_meta_image_element() -> Result<(), Error> {
        let page = parse(r#"<head><meta property="og:images" content="image-link" /></head>"#)?;
        assert_eq!(None, page.meta_image());
        Ok(())
    }

    #[actix_rt::test]
    async fn title() -> Result<(), Error> {
        let page = parse(r#"<head><title>page-title</title></head>"#)?;
        assert_eq!(Some("page-title".to_string()), page.title());
        Ok(())
    }

    #[actix_rt::test]
    async fn no_title() -> Result<(), Error> {
        let page = parse(r#"<head></head>"#)?;
        assert_eq!(None, page.title());
        Ok(())
    }
}
