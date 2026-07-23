mod common;

use common::{cookie_only, record_body};

/// Uppercase `WWW.` should normalise to the same domain as lowercase `www.`,
/// otherwise the same site dedups into two services.
#[tokio::test]
async fn www_normalisation_is_case_insensitive() {
    let ctx = common::setup().await;
    let server = ctx.client();
    let auth = common::register(&server, "www@example.com", "correcthorse1").await;
    let category = common::first_category_id(&server).await;

    common::create_record(
        &server,
        &auth,
        &record_body("Lower", "https://www.example.com/a", &category),
    )
    .await;
    common::create_record(
        &server,
        &auth,
        &record_body("Upper", "https://WWW.Example.COM/b", &category),
    )
    .await;

    let domains: Vec<String> =
        sqlx::query_scalar("SELECT normalized_domain FROM services ORDER BY normalized_domain")
            .fetch_all(&ctx.pool)
            .await
            .unwrap();
    assert_eq!(
        domains,
        vec!["example.com".to_string()],
        "www./WWW. must normalise to a single service"
    );
}

/// The service name lives on each user's record, so a second user recording a
/// consent for the same domain must not change the name the first user sees.
/// Both records still share one underlying service row (domain dedup).
#[tokio::test]
async fn service_name_is_isolated_per_user() {
    let ctx = common::setup().await;
    let server = ctx.client();
    let alice = common::register(&server, "a-bleed@example.com", "correcthorse1").await;
    let bob = common::register(&server, "b-bleed@example.com", "correcthorse1").await;
    let category = common::first_category_id(&server).await;

    let alice_record = common::create_record(
        &server,
        &alice,
        &record_body("Alice Name", "https://shared.example", &category),
    )
    .await;
    common::create_record(
        &server,
        &bob,
        &record_body("Bob Name", "https://shared.example", &category),
    )
    .await;

    let refetched = cookie_only(
        server.get(&format!(
            "/api/records/{}",
            alice_record["id"].as_str().unwrap()
        )),
        &alice,
    )
    .await;
    assert_eq!(
        refetched.json::<serde_json::Value>()["service_name"],
        "Alice Name",
        "Bob's write must not change the name Alice sees"
    );

    // The domain still dedups to a single shared service row.
    let services: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM services WHERE normalized_domain = 'shared.example'",
    )
    .fetch_one(&ctx.pool)
    .await
    .unwrap();
    assert_eq!(services, 1);
}

/// A note beginning with a formula character is neutralised with a leading
/// single quote so spreadsheets read it as text (CSV injection guard).
#[tokio::test]
async fn csv_neutralises_formula_injection() {
    let ctx = common::setup().await;
    let server = ctx.client();
    let auth = common::register(&server, "inj@example.com", "correcthorse1").await;
    let category = common::first_category_id(&server).await;

    let mut body = record_body("Acme", "https://acme.example", &category);
    body["notes"] = serde_json::json!("=1+1");
    common::create_record(&server, &auth, &body).await;

    let text = cookie_only(server.get("/api/export.csv"), &auth)
        .await
        .text();
    assert!(
        text.contains(",'=1+1,"),
        "formula prefix must be quoted as text"
    );
    assert!(!text.contains(",=1+1,"), "raw formula must not appear");
}

/// Documents that the search term is not escaped for LIKE wildcards, so `%`
/// matches every record rather than a literal percent sign.
#[tokio::test]
async fn search_percent_is_a_wildcard_known_quirk() {
    let ctx = common::setup().await;
    let server = ctx.client();
    let auth = common::register(&server, "like@example.com", "correcthorse1").await;
    let category = common::first_category_id(&server).await;
    common::create_record(
        &server,
        &auth,
        &record_body("One", "https://one.example", &category),
    )
    .await;
    common::create_record(
        &server,
        &auth,
        &record_body("Two", "https://two.example", &category),
    )
    .await;

    let response = cookie_only(server.get("/api/records?q=%25"), &auth).await;
    response.assert_status_ok();
    assert_eq!(response.json::<Vec<serde_json::Value>>().len(), 2);
}
