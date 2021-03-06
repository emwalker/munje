mod support;

use actix_web::{http, test, web};
use munje::{
    questions::{CreateQuestion, Question},
    queues::routes::AnswerQuestionForm,
    queues::{CreateQueue, Queue},
};
use serde::Serialize;

use crate::support::{Runner, TestResult};

#[actix_rt::test]
async fn home_unauthenticated() -> TestResult {
    let res = Runner::build().to_runner().await.get("/").await;

    assert_eq!(http::StatusCode::OK, res.status);
    assert_eq!("Munje", res.doc.select_text("p.title").unwrap());
    assert_eq!(
        "Overview",
        res.doc.select_text(".navbar-item.overview").unwrap()
    );
    assert_eq!(
        "Questions",
        res.doc.select_text(".navbar-item.questions").unwrap()
    );
    Ok(())
}

#[actix_rt::test]
async fn home_authenticated() -> TestResult {
    let res = Runner::build().auth().to_runner().await.get("/").await;

    assert_eq!(http::StatusCode::OK, res.status);
    assert_eq!("Munje", res.doc.select_text("p.title").unwrap());
    assert_eq!(
        "Overview",
        res.doc.select_text(".navbar-item.overview").unwrap()
    );
    assert_eq!(
        "Queues",
        res.doc.select_text(".navbar-item.queues").unwrap()
    );
    assert_eq!(
        "Questions",
        res.doc.select_text(".navbar-item.questions").unwrap()
    );
    Ok(())
}

#[actix_rt::test]
async fn overview() -> TestResult {
    let res = Runner::build()
        .auth()
        .to_runner()
        .await
        .get("/overview")
        .await;

    assert_eq!(http::StatusCode::OK, res.status);
    assert_eq!("Overview", res.doc.select_text("h2.title").unwrap());
    Ok(())
}

#[actix_rt::test]
async fn robots() -> TestResult {
    let res = Runner::build().to_runner().await.get("/robots.txt").await;
    assert_eq!(http::StatusCode::OK, res.status);
    assert_eq!("User-agent: * Disallow: /", res.doc.to_string());
    Ok(())
}

#[actix_rt::test]
async fn list_questions() -> TestResult {
    let res = Runner::build().to_runner().await.get("/questions").await;
    assert_eq!(http::StatusCode::OK, res.status);
    assert_eq!("Questions", res.doc.select_text("h2").unwrap());
    Ok(())
}

#[actix_rt::test]
async fn new_question() -> TestResult {
    let res = Runner::build()
        .auth()
        .to_runner()
        .await
        .get("/questions/new")
        .await;

    assert_eq!(res.status, http::StatusCode::OK);
    assert_eq!("Add a question", res.doc.select_text("h2").unwrap());
    Ok(())
}

#[actix_rt::test]
async fn show_unknown_question() -> TestResult {
    let res = Runner::build()
        .to_runner()
        .await
        .get("/questions/unknown")
        .await;

    assert_eq!(res.status, http::StatusCode::NOT_FOUND);
    let title = res.doc.select_text("title").unwrap();
    assert_eq!("Not found", title);
    Ok(())
}

#[actix_rt::test]
async fn show_question() -> TestResult {
    let runner = Runner::build().auth().to_runner().await;
    let question = CreateQuestion {
        author_id: runner.user.id,
        title: "some-title".to_string(),
        link: "some-link".to_string(),
        link_logo: Some("logo-url".to_string()),
    };

    let question = Question::create(question, &runner.db).await?;
    let path = format!("/questions/{}", question.external_id);
    let res = runner.get(&path).await;
    assert_eq!(res.status, http::StatusCode::OK);

    let doc = res.doc;

    assert_eq!(
        "some-title",
        doc.css("span.title-span")?.first().unwrap().inner_html()
    );
    assert!(doc.css("span.link-logo")?.exists());
    assert!(doc.css("button.start-queue")?.exists());
    Ok(())
}

#[actix_rt::test]
async fn start_queue() -> TestResult {
    let runner = Runner::build().auth().to_runner().await;
    let question = CreateQuestion {
        author_id: runner.user.id,
        title: "some-title".to_string(),
        link: "some-link".to_string(),
        link_logo: Some("logo-url".to_string()),
    };

    let question = Question::create(question, &runner.db).await?;
    let req = test::TestRequest::post()
        .uri(format!("/questions/{}/queues", question.external_id).as_ref())
        .append_header(("Content-type", "application/x-www-form-urlencoded"));
    let res = runner.call(req).await;

    assert_eq!(res.status, http::StatusCode::FOUND);
    Ok(())
}

#[actix_rt::test]
async fn show_queue() -> TestResult {
    let runner = Runner::build().auth().to_runner().await;

    let question = Question::create(
        CreateQuestion {
            author_id: runner.user.id,
            title: "some-title".to_string(),
            link: "some-link".to_string(),
            link_logo: Some("logo-url".to_string()),
        },
        &runner.db,
    )
    .await?;

    let queue = Queue::find_or_create(
        CreateQueue {
            user_id: runner.user.id,
            starting_question_external_id: question.external_id.clone(),
            title: "Algorithms and data structures".to_string(),
            description: "A queue".to_string(),
        },
        &runner.db,
    )
    .await?
    .record;

    let path = format!("/{}/queues/{}", runner.user.handle, queue.external_id);
    let res = runner.get(&path).await;

    assert_eq!(http::StatusCode::OK, res.status);
    assert!(res.doc.css(".card")?.exists());

    let action_prefix = format!("{}/questions/", path);
    let mut form = res.doc.css("form.next-question").unwrap();
    let action = form.first().unwrap().value().attr("action").unwrap();
    assert!(
        action.starts_with(&action_prefix),
        "Unexpected action: {}",
        action
    );

    Ok(())
}

#[actix_rt::test]
async fn answer_question() -> TestResult {
    let runner = Runner::build().auth().to_runner().await;
    runner.reset_database().await?;

    let question = Question::create(
        CreateQuestion {
            author_id: runner.user.id,
            title: "some-title".to_string(),
            link: "some-link".to_string(),
            link_logo: Some("logo-url".to_string()),
        },
        &runner.db,
    )
    .await?;

    let queue = Queue::find_or_create(
        CreateQueue {
            user_id: runner.user.id,
            starting_question_external_id: question.external_id.clone(),
            title: "Algorithms and data structures".to_string(),
            description: "A queue".to_string(),
        },
        &runner.db,
    )
    .await?
    .record;

    let form = web::Form(AnswerQuestionForm {
        state: "Correct".to_string(),
    });

    let uri = format!(
        "/{}/queues/{}/questions/{}",
        runner.user.handle, queue.external_id, question.external_id
    );

    let req = test::TestRequest::post().uri(uri.as_ref()).set_form(&form);
    let res = runner.call(req).await;

    assert_eq!(res.status, http::StatusCode::FOUND);
    Ok(())
}

#[actix_rt::test]
async fn list_queues() -> TestResult {
    let runner = Runner::build().auth().to_runner().await;
    let path = format!("/{}/queues", runner.user.handle);
    let res = runner.get(path.as_ref()).await;

    assert_eq!(res.status, http::StatusCode::OK);
    assert_eq!(
        "Queues you are working on",
        res.doc.select_text("h2").unwrap()
    );
    Ok(())
}

#[actix_rt::test]
async fn user_signup() -> TestResult {
    let res = Runner::build().to_runner().await.get("/users/signup").await;

    assert_eq!(http::StatusCode::OK, res.status);
    assert_eq!(
        Some("Sign up"),
        res.doc
            .css("input[type=submit]")?
            .first()
            .unwrap()
            .value()
            .attr("value")
    );
    Ok(())
}

#[actix_rt::test]
async fn user_login() -> TestResult {
    let res = Runner::build().to_runner().await.get("/users/login").await;

    assert_eq!(http::StatusCode::OK, res.status);
    assert_eq!(
        Some("Your username"),
        res.doc
            .css("input[type=text]")?
            .first()
            .unwrap()
            .value()
            .attr("placeholder")
    );
    Ok(())
}

#[actix_rt::test]
async fn create_user() -> TestResult {
    let runner = Runner::build().to_runner().await;
    runner.reset_database().await?;

    #[derive(Serialize)]
    struct SimpleSignupForm<'a> {
        handle: &'a str,
        password: &'a str,
        password_confirmation: &'a str,
    }

    let form = web::Form(SimpleSignupForm {
        handle: "frotz",
        password: "Password1",
        password_confirmation: "Password1",
    });

    let req = test::TestRequest::post()
        .uri("/users/signup")
        .set_form(&form);
    let res = runner.call(req).await;

    assert_eq!(http::StatusCode::FOUND, res.status);
    Ok(())
}
