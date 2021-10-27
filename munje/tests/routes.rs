mod support;

use munje::{
    questions,
    questions::{CreateQuestion, Question},
    routes,
};

use crate::support::{Runner, TestResult};

#[actix_rt::test]
async fn test_home() -> TestResult {
    let doc = Runner::new().await.get(routes::home, "/").await?;
    assert_eq!("Munje", doc.select_text("p.title").unwrap());
    Ok(())
}

#[actix_rt::test]
async fn test_list() -> TestResult {
    let doc = Runner::new()
        .await
        .get(questions::routes::list, "/questions")
        .await?;
    assert_eq!("Questions", doc.select_text("h2").unwrap());
    Ok(())
}

#[actix_rt::test]
async fn new() -> TestResult {
    let doc = Runner::new()
        .await
        .get(questions::routes::show_or_new, "/questions/new")
        .await?;
    assert_eq!("Add a question", doc.select_text("h2").unwrap());
    Ok(())
}

#[actix_rt::test]
async fn unknown() -> TestResult {
    let doc = Runner::new()
        .await
        .get(questions::routes::show_or_new, "/questions/unknown")
        .await?;
    let title = doc.select_text("title").unwrap();
    assert_eq!("Question not found", title);
    Ok(())
}

#[actix_rt::test]
async fn show() -> TestResult {
    let harness = Runner::new().await;
    let question = CreateQuestion {
        author_id: "21546b43-dcde-43b2-a251-e736194de0a0".to_string(),
        title: "some-title".to_string(),
        link: "some-link".to_string(),
        link_logo: Some("logo-url".to_string()),
    };
    let question = Question::create(question, &harness.db).await?;
    let path = format!("/questions/{}", question.id);
    let doc = harness.get(questions::routes::show_or_new, &path).await?;

    assert_eq!(
        "some-title",
        doc.css("span.title-span")?.first().unwrap().inner_html()
    );
    assert!(doc.css("span.link-logo")?.exists());
    assert!(doc.css("button.start-queue")?.exists());
    Ok(())
}

#[actix_rt::test]
async fn test_start_queue() -> TestResult {
    let harness = Runner::new().await;
    let question = CreateQuestion {
        author_id: "21546b43-dcde-43b2-a251-e736194de0a0".to_string(),
        title: "some-title".to_string(),
        link: "some-link".to_string(),
        link_logo: Some("logo-url".to_string()),
    };
    let question = Question::create(question, &harness.db).await?;
    let path = format!("/questions/{}/queues", question.id);
    harness.post(questions::routes::start_queue, &path).await?;
    Ok(())
}