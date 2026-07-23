mod common;

use common::{authed, cookie_only, record_body};
use http::StatusCode;

/// Two users, each with a record and a paired device. User B must never reach
/// User A's resources, and cross-user access returns 404 (no existence leak).
struct TwoUsers {
    server: axum_test::TestServer,
    a: common::Auth,
    b: common::Auth,
    a_record: String,
    a_device: String,
}

async fn two_users(ctx: &common::TestContext) -> TwoUsers {
    let server = ctx.client();
    let a = common::register(&server, "alice@example.com", "correcthorse1").await;
    let b = common::register(&server, "bob@example.com", "correcthorse1").await;
    let category = common::first_category_id(&server).await;

    let a_record = common::create_record(
        &server,
        &a,
        &record_body("Alice Service", "https://alice.example", &category),
    )
    .await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let a_device = authed(server.post("/api/extension/pair"), &a)
        .json(&serde_json::json!({ "device_name": "Alice laptop" }))
        .await
        .json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .to_string();

    TwoUsers {
        server,
        a,
        b,
        a_record,
        a_device,
    }
}

#[tokio::test]
async fn cross_user_record_access_is_not_found() {
    let ctx = common::setup().await;
    let t = two_users(&ctx).await;
    let path = format!("/api/records/{}", t.a_record);

    cookie_only(t.server.get(&path), &t.b)
        .await
        .assert_status(StatusCode::NOT_FOUND);

    let category = common::first_category_id(&t.server).await;
    authed(t.server.put(&path), &t.b)
        .json(&record_body("Hijack", "https://alice.example", &category))
        .await
        .assert_status(StatusCode::NOT_FOUND);
    authed(t.server.post(&format!("{path}/revoke")), &t.b)
        .await
        .assert_status(StatusCode::NOT_FOUND);
    authed(t.server.delete(&path), &t.b)
        .await
        .assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn list_dashboard_and_export_exclude_other_users_rows() {
    let ctx = common::setup().await;
    let t = two_users(&ctx).await;

    let list = cookie_only(t.server.get("/api/records"), &t.b).await;
    assert_eq!(list.json::<Vec<serde_json::Value>>().len(), 0);

    let dashboard = cookie_only(t.server.get("/api/dashboard"), &t.b).await;
    assert_eq!(
        dashboard.json::<serde_json::Value>()["summary"]["active"],
        0
    );

    let export = cookie_only(t.server.get("/api/export.json"), &t.b).await;
    assert_eq!(
        export.json::<serde_json::Value>()["records"]
            .as_array()
            .unwrap()
            .len(),
        0
    );

    // Alice still sees her own record.
    let a_list = cookie_only(t.server.get("/api/records"), &t.a).await;
    assert_eq!(a_list.json::<Vec<serde_json::Value>>().len(), 1);
}

#[tokio::test]
async fn cross_user_device_revoke_is_not_found_and_list_is_scoped() {
    let ctx = common::setup().await;
    let t = two_users(&ctx).await;

    authed(
        t.server
            .delete(&format!("/api/extension/devices/{}", t.a_device)),
        &t.b,
    )
    .await
    .assert_status(StatusCode::NOT_FOUND);

    let b_devices = cookie_only(t.server.get("/api/extension/devices"), &t.b).await;
    assert_eq!(b_devices.json::<Vec<serde_json::Value>>().len(), 0);

    // Alice's device is untouched.
    let a_devices = cookie_only(t.server.get("/api/extension/devices"), &t.a).await;
    let devices = a_devices.json::<Vec<serde_json::Value>>();
    assert_eq!(devices.len(), 1);
    assert!(devices[0]["revoked_at"].is_null());
}
