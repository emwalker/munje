mod support;

use munje::routes;

use crate::support::{Runner, TestResult};

#[actix_rt::test]
async fn test_home() -> TestResult {
    let doc = Runner::new().await.get(routes::home, "/").await?;
    assert_eq!("Munje", doc.select_text("p.title").unwrap());
    Ok(())
}
