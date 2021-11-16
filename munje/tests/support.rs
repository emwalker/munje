#![allow(dead_code)]

use actix_identity::{CookieIdentityPolicy, Identity, IdentityService};
use actix_web::{dev::ServiceResponse, http, test, web, App, HttpRequest};
use munje::{
    error::Error,
    prelude::*,
    questions, queues, routes,
    types::{AppState, Config, Pool},
    users,
};
use scraper::{ElementRef, Html, Selector};
use sqlx::postgres::PgPoolOptions;
use std::str;

#[cfg(test)]
#[ctor::ctor]
fn init() {
    env_logger::init();
}

pub type TestResult = Result<(), Error>;

#[derive(Debug, Clone)]
pub struct Document {
    source: String,
    doc: Html,
}

pub struct HttpResult {
    pub doc: Document,
    pub status: http::StatusCode,
}

pub struct Matches<'a> {
    selector: Selector,
    matches: Vec<ElementRef<'a>>,
}

impl<'a> Matches<'a> {
    pub fn first(&mut self) -> Option<&ElementRef> {
        self.matches.iter().next()
    }

    pub fn exists(&mut self) -> bool {
        self.matches.iter().next() != None
    }

    pub fn none(&mut self) -> bool {
        !self.exists()
    }
}

impl Document {
    fn from(html: &str) -> Self {
        Self {
            source: html.to_string(),
            doc: Html::parse_document(html),
        }
    }

    pub fn to_string(&self) -> String {
        self.source.to_string()
    }

    pub fn select_text(&self, selector: &str) -> Option<String> {
        match self.css(selector).unwrap().first() {
            Some(elem) => Some(elem.inner_html()),
            None => None,
        }
    }

    pub fn css(&self, selector_str: &str) -> Result<Matches, Error> {
        let selector = Selector::parse(selector_str).unwrap();
        Ok(Matches {
            selector: selector.clone(),
            matches: self.doc.select(&selector).collect(),
        })
    }
}

pub struct Runner {
    pub db: Pool,
    pub user: User,
    pub is_authenticated: bool,
    pub config: Config,
}

pub struct RunnerBuilder {
    is_autheticated: bool,
}

impl RunnerBuilder {
    pub fn auth(&mut self) -> &mut Self {
        self.is_autheticated = true;
        self
    }

    pub async fn to_runner(&self) -> Runner {
        let config = Config::test().expect("Failed to load test config");
        let db = Self::fetch_db(&config.database_url).await;

        if !self.is_autheticated {
            return Runner {
                db,
                config,
                user: User::guest(),
                is_authenticated: false,
            };
        }

        let user = User::find_by_handle("gnusto", &db)
            .await
            .expect("Failed to fetch user");

        Runner {
            db,
            config,
            user,
            is_authenticated: true,
        }
    }

    async fn fetch_db(database_url: &str) -> Pool {
        let db = PgPoolOptions::new()
            .max_connections(1)
            .connect(database_url)
            .await
            .expect("Failed to fetch database pool");
        sqlx::migrate!("./migrations")
            .run(&db)
            .await
            .expect("Failed to run migrations");

        db
    }
}

impl Runner {
    pub fn build() -> RunnerBuilder {
        RunnerBuilder {
            is_autheticated: false,
        }
    }

    pub async fn reset_database(&self) -> Result<(), Error> {
        sqlx::query("delete from last_answers")
            .execute(&self.db)
            .await?;
        sqlx::query("delete from answers").execute(&self.db).await?;
        sqlx::query("delete from users where handle <> 'gnusto'")
            .execute(&self.db)
            .await?;
        Ok(())
    }

    async fn call_service(&self, mut req: test::TestRequest) -> ServiceResponse {
        let policy = CookieIdentityPolicy::new(self.config.session_key.as_bytes())
            .name("auth-cookie")
            .secure(true);

        let app = App::new()
            .app_data(web::Data::new(AppState {
                db: self.db.clone(),
            }))
            .wrap(IdentityService::new(policy))
            .service(web::resource("/login/{handle}").to(
                |id: Identity, path: web::Path<String>, request: HttpRequest| async move {
                    let db = request.db().expect("Failed to fetch database handle");
                    let user = User::find_by_handle(&path.into_inner(), db)
                        .await
                        .expect("Unable to find user");
                    let string = serde_json::to_string(&user).expect("Failed to serialize user");
                    id.remember(string);
                    HttpResponse::Ok()
                },
            ))
            .configure(routes::register)
            .configure(users::routes::register)
            .configure(questions::routes::register)
            .configure(queues::routes::register);

        let srv = test::init_service(app).await;

        if self.is_authenticated {
            let login_path = format!("/login/{}", self.user.handle);
            let auth_req = test::TestRequest::with_uri(&login_path).to_request();
            let res = test::call_service(&srv, auth_req).await;
            assert_eq!(http::StatusCode::OK, res.status());
            let cookie = res.response().cookies().next().unwrap().to_owned();
            req = req.cookie(cookie)
        }

        test::call_service(&srv, req.to_request()).await
    }

    pub async fn call(&self, req: test::TestRequest) -> HttpResult {
        let res = self.call_service(req).await;
        let status = res.status();
        let body = test::read_body(res).await;
        let html = str::from_utf8(&body).expect("Failed to decode body");

        HttpResult {
            doc: Document::from(html),
            status,
        }
    }

    pub async fn get(&self, path: &str) -> HttpResult {
        let req = test::TestRequest::with_uri(path);
        self.call(req).await
    }
}
