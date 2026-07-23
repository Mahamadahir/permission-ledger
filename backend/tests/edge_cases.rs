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

/// Services are global and keyed by domain. Documents that a second user
/// creating a record for the same domain overwrites the service name the first
/// user sees. This is a known cross-user data-bleed, pinned here so a future fix
/// (e.g. per-user service names) deliberately changes it.
#[tokio::test]
async fn service_name_is_shared_across_users_known_bleed() {
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
        "Bob Name",
        "known issue: Bob's write overwrote the service name Alice sees"
    );
}

/// Documents that CSV export does not neutralise spreadsheet formula prefixes.
/// A note beginning with `=` is emitted verbatim (a formula-injection risk).
#[tokio::test]
async fn csv_does_not_neutralise_formula_injection_known_issue() {
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
        text.contains(",=1+1,") || text.contains(",=1+1\n"),
        "known issue: formula prefix emitted verbatim, not escaped"
    );
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
