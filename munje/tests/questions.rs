mod support;
use munje::questions::{routes, CreateQuestion, Question};

use crate::support::{Runner, TestResult};

#[actix_rt::test]
async fn test_list() -> TestResult {
    let doc = Runner::new().await.get(routes::list, "/questions").await?;
    assert_eq!("Questions", doc.select_text("h2").unwrap());
    Ok(())
}

#[actix_rt::test]
async fn new() -> TestResult {
    let doc = Runner::new()
        .await
        .get(routes::show_or_new, "/questions/new")
        .await?;
    assert_eq!("Add a question", doc.select_text("h2").unwrap());
    Ok(())
}

#[actix_rt::test]
async fn unknown() -> TestResult {
    let doc = Runner::new()
        .await
        .get(routes::show_or_new, "/questions/unknown")
        .await?;
    let title = doc.select_text("title").unwrap();
    assert_eq!("Question not found", title);
    Ok(())
}

#[actix_rt::test]
async fn show() -> TestResult {
    let harness = Runner::new().await;
    let data = CreateQuestion {
        link: "some-link".to_string(),
    };
    let question = Question::create(&data, Some("logo-url".to_string()), &harness.db).await?;
    let path = format!("/questions/{}", question.id);
    let doc = harness.get(routes::show_or_new, &path).await?;

    assert_eq!("Question", doc.css("h2")?.first().unwrap().inner_html());
    assert_eq!(
        "some-link",
        doc.css("a.link")?
            .first()
            .unwrap()
            .value()
            .attr("href")
            .unwrap(),
    );
    assert!(doc.css("div.link-logo")?.exists());
    assert!(doc.css("button.start-queue")?.exists());
    Ok(())
}

#[actix_rt::test]
async fn test_start_queue() -> TestResult {
    let harness = Runner::new().await;
    let data = CreateQuestion {
        link: "some-link".to_string(),
    };
    let question = Question::create(&data, Some("logo-url".to_string()), &harness.db).await?;
    let path = format!("/questions/{}/queues", question.id);
    harness.post(routes::start_queue, &path).await?;
    Ok(())
}
