mod support;

use actix_web::{http, test, web};
use munje::{
    questions::{CreateQuestion, Question},
    queues::routes::AnswerQuestionForm,
    queues::{CreateQueue, Queue},
};

use crate::support::{Runner, TestResult};

#[actix_rt::test]
async fn home() -> TestResult {
    let res = Runner::new().await.get("/").await?;
    assert_eq!(res.status, http::StatusCode::OK);
    assert_eq!("Munje", res.doc.select_text("p.title").unwrap());
    Ok(())
}

#[actix_rt::test]
async fn list_questions() -> TestResult {
    let res = Runner::new().await.get("/questions").await?;
    assert_eq!(res.status, http::StatusCode::OK);
    assert_eq!("Questions", res.doc.select_text("h2").unwrap());
    Ok(())
}

#[actix_rt::test]
async fn new_question() -> TestResult {
    let res = Runner::new().await.get("/questions/new").await?;
    assert_eq!(res.status, http::StatusCode::OK);
    assert_eq!("Add a question", res.doc.select_text("h2").unwrap());
    Ok(())
}

#[actix_rt::test]
async fn show_unknown_question() -> TestResult {
    let res = Runner::new().await.get("/questions/unknown").await?;
    assert_eq!(res.status, http::StatusCode::OK);
    let title = res.doc.select_text("title").unwrap();
    assert_eq!("Question not found", title);
    Ok(())
}

#[actix_rt::test]
async fn show_question() -> TestResult {
    let runner = Runner::new().await;
    let question = CreateQuestion {
        author_id: "21546b43-dcde-43b2-a251-e736194de0a0".to_string(),
        title: "some-title".to_string(),
        link: "some-link".to_string(),
        link_logo: Some("logo-url".to_string()),
    };

    let question = Question::create(question, &runner.db).await?;
    let path = format!("/questions/{}", question.id);
    let res = runner.get(&path).await?;
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
    let runner = Runner::new().await;
    let question = CreateQuestion {
        author_id: "21546b43-dcde-43b2-a251-e736194de0a0".to_string(),
        title: "some-title".to_string(),
        link: "some-link".to_string(),
        link_logo: Some("logo-url".to_string()),
    };

    let question = Question::create(question, &runner.db).await?;
    let req = test::TestRequest::post()
        .uri(format!("/questions/{}/queues", question.id).as_ref())
        .append_header(("Content-type", "application/x-www-form-urlencoded"))
        .to_request();
    let res = runner.post(req).await?;

    assert_eq!(res.status, http::StatusCode::SEE_OTHER);
    Ok(())
}

#[actix_rt::test]
async fn show_queue() -> TestResult {
    let runner = Runner::new().await;

    let question = Question::create(
        CreateQuestion {
            author_id: runner.user.id.to_string(),
            title: "some-title".to_string(),
            link: "some-link".to_string(),
            link_logo: Some("logo-url".to_string()),
        },
        &runner.db,
    )
    .await?;

    let queue = Queue::find_or_create(
        CreateQueue {
            user_id: runner.user.id.to_string(),
            starting_question_id: question.id,
            title: "Algorithms and data structures".to_string(),
            description: "A queue".to_string(),
        },
        &runner.db,
    )
    .await?
    .record;

    let path = format!("/queues/{}", queue.id);
    let res = runner.get(&path).await?;
    assert_eq!(res.status, http::StatusCode::OK);
    assert!(res.doc.css(".card")?.exists());
    Ok(())
}

#[actix_rt::test]
async fn answer_question() -> TestResult {
    let runner = Runner::new().await;

    let question = Question::create(
        CreateQuestion {
            author_id: runner.user.id.to_string(),
            title: "some-title".to_string(),
            link: "some-link".to_string(),
            link_logo: Some("logo-url".to_string()),
        },
        &runner.db,
    )
    .await?;

    let queue = Queue::find_or_create(
        CreateQueue {
            user_id: runner.user.id.to_string(),
            starting_question_id: question.id.clone(),
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

    let req = test::TestRequest::post()
        .uri(format!("/queues/{}/questions/{}", queue.id, question.id).as_ref())
        .append_header(("Content-type", "application/x-www-form-urlencoded"))
        .set_form(&form)
        .to_request();
    let res = runner.post(req).await?;

    assert_eq!(res.status, http::StatusCode::SEE_OTHER);
    Ok(())
}

#[actix_rt::test]
async fn list_queues() -> TestResult {
    let res = Runner::new().await.get("/gnusto/queues").await?;
    assert_eq!(res.status, http::StatusCode::OK);
    assert_eq!(
        "Queues you are working on",
        res.doc.select_text("h2").unwrap()
    );
    Ok(())
}
