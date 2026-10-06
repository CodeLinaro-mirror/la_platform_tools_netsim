// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! Tests for OpenAPI 3.1 route dispatch and specification validation.

use hyper::Method;
use netsim_rest_api::openapi::{JSON, Route, find_route, validate};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

mod generated {
    use super::*;

    /// Widget color.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub enum Color {
        #[serde(rename = "RED")]
        Red,
        #[serde(rename = "GREEN")]
        Green,
    }

    /// Empty response body.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Empty {}

    /// A widget.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Widget {
        pub color: Color,
        pub name: String,
        /// Optional display label.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub label: Option<String>,
    }

    /// Response body for `ListWidgets`.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct ListWidgetsResponse {
        pub widgets: Vec<Widget>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Op {
        ListWidgets,
        CreateWidget,
        GetWidget,
        DeleteWidget,
        ResetWidget,
    }

    pub const ROUTES: &[Route<Op>] = &[
        Route::new(Method::GET, "/v1/widgets", Op::ListWidgets),
        Route::new(Method::POST, "/v1/widgets", Op::CreateWidget),
        Route::new(Method::GET, "/v1/widgets/{name}", Op::GetWidget),
        Route::new(Method::DELETE, "/v1/widgets/{name}", Op::DeleteWidget),
        Route::new(Method::POST, "/v1/widgets/{name}:reset", Op::ResetWidget),
    ];
}

use generated::{Color, Empty, ListWidgetsResponse, Op, ROUTES, Widget};

/// Checked-in `openapi.yaml` for the Widget test API.
const OPENAPI_YAML: &str = include_str!("openapi.yaml");

/// Parses [`OPENAPI_YAML`] into a JSON [`Value`].
fn spec() -> Value {
    let mut v: serde_yaml::Value = serde_yaml::from_str(OPENAPI_YAML).unwrap_or_default();
    let _ = v.apply_merge();
    serde_yaml::from_value(v).unwrap_or_default()
}

/// Serves one request the way the REST server would: `GET /v1/openapi.json`
/// returns [`spec`], and everything else is dispatched through [`ROUTES`].
fn handle(method: &Method, path: &str, body: &[u8]) -> (u16, Value) {
    if method == Method::GET && path == "/v1/openapi.json" {
        return (200, spec());
    }
    let Some((route, params)) = find_route(ROUTES, method, path) else {
        return (404, json!({}));
    };
    let name = params.iter().find_map(|&(k, v)| (k == "name").then_some(v));
    let widget = |name: &str| Widget { color: Color::Red, name: name.into(), label: None };
    let result = match route.op {
        Op::ListWidgets => {
            serde_json::to_value(ListWidgetsResponse { widgets: vec![widget("w1")] })
        }
        Op::CreateWidget => match serde_json::from_slice::<Widget>(body) {
            Ok(created) => serde_json::to_value(created),
            Err(e) => return (400, json!({ "error": e.to_string() })),
        },
        Op::GetWidget => serde_json::to_value(widget(name.unwrap_or_default())),
        Op::DeleteWidget | Op::ResetWidget => serde_json::to_value(Empty {}),
    };
    match result {
        Ok(value) => (200, value),
        Err(e) => (500, json!({ "error": e.to_string() })),
    }
}

#[test]
fn test_spec_is_cvd_env_friendly() {
    assert_eq!(validate(&spec()), Vec::<String>::new());
}

#[test]
fn test_validate_rejects_unfriendly_documents() {
    type Case = (&'static str, fn(&mut Value));
    let cases: &[Case] = &[
        ("required string path param", |d| {
            d["paths"]["/v1/widgets"]["get"]["parameters"] =
                json!([{ "name": "filter", "in": "query", "required": false }]);
        }),
        ("must be exactly application/json", |d| {
            d["paths"]["/v1/widgets"]["post"]["requestBody"]["content"]["text/plain"] = json!({});
        }),
        ("not unique", |d| {
            d["paths"]["/v1/widgets"]["get"]["operationId"] = json!("CreateWidget");
        }),
        ("does not resolve", |d| {
            d["paths"]["/v1/widgets"]["get"]["responses"]["200"]["content"][JSON]["schema"] =
                json!({ "$ref": "#/components/schemas/Missing" });
        }),
        ("also a body field", |d| {
            let post = d["paths"]["/v1/widgets"]["post"].clone();
            d["paths"]["/v1/widgets/{name}"]["post"] = post;
            d["paths"]["/v1/widgets/{name}"]["post"]["operationId"] = json!("RenameWidget");
            d["paths"]["/v1/widgets/{name}"]["post"]["parameters"] = json!([{
                "name": "name",
                "in": "path",
                "required": true,
                "schema": { "type": "string" }
            }]);
        }),
        ("needs an example", |d| {
            if let Some(media) =
                d["paths"]["/v1/widgets"]["post"]["requestBody"]["content"][JSON].as_object_mut()
            {
                media.remove("example");
            }
        }),
        ("is recursive", |d| {
            d["components"]["schemas"]["Empty"]["properties"] =
                json!({ "self_ref": { "$ref": "#/components/schemas/Empty" } });
        }),
        ("unsupported property type", |d| {
            d["components"]["schemas"]["Empty"]["properties"] =
                json!({ "nested": { "type": "object" } });
        }),
        ("invalid field identifier", |d| {
            d["components"]["schemas"]["Empty"]["properties"] =
                json!({ "explicit": { "type": "string" } });
        }),
        ("enum values must be distinct valid identifiers", |d| {
            d["components"]["schemas"]["Color"]["enum"] = json!(["in-progress"]);
        }),
        ("enum values must be distinct valid identifiers", |d| {
            d["components"]["schemas"]["Color"]["enum"] = json!(["FOO_BAR", "foo_bar"]);
        }),
        ("enum values must be distinct valid identifiers", |d| {
            d["components"]["schemas"]["Color"]["enum"] = json!(["_"]);
        }),
        ("method is not allowed", |d| {
            d["paths"]["/v1/widgets"]["summary"] = json!("Widget operations");
        }),
        ("requestBody example must be an object", |d| {
            d["paths"]["/v1/widgets"]["post"]["requestBody"]["content"][JSON]["example"] =
                json!("not-an-object");
        }),
        ("requestBody example must not be an object", |d| {
            d["paths"]["/v1/widgets"]["post"]["requestBody"]["content"][JSON]["schema"] =
                json!({ "$ref": "#/components/schemas/Color" });
            d["paths"]["/v1/widgets"]["post"]["requestBody"]["content"][JSON]["example"] =
                json!({});
        }),
        ("use OpenAPI 3.1 type array instead of nullable", |d| {
            d["components"]["schemas"]["Widget"]["properties"]["label"]["nullable"] = json!(true);
        }),
        ("omit empty required array", |d| {
            d["components"]["schemas"]["Empty"]["required"] = json!([]);
        }),
        ("responses must include \"200\"", |d| {
            d["paths"]["/v1/widgets"]["get"]["responses"] =
                json!({ "404": { "$ref": "#/components/responses/EmptyResponse" } });
        }),
    ];
    for &(expected, mutate) in cases {
        let mut doc = spec();
        mutate(&mut doc);
        let errors = validate(&doc);
        assert!(errors.iter().any(|e| e.contains(expected)), "{expected}: {errors:?}");
    }
}

#[test]
fn test_find_route_uses_table() {
    fn find(method: Method, path: &str) -> Option<(Op, Vec<(&str, &str)>)> {
        find_route(ROUTES, &method, path).map(|(r, params)| (r.op, params))
    }
    assert_eq!(find(Method::GET, "/v1/widgets"), Some((Op::ListWidgets, vec![])));
    assert_eq!(find(Method::POST, "/v1/widgets"), Some((Op::CreateWidget, vec![])));
    assert_eq!(find(Method::GET, "/v1/widgets/w1"), Some((Op::GetWidget, vec![("name", "w1")])));
    assert_eq!(
        find(Method::DELETE, "/v1/widgets/w1"),
        Some((Op::DeleteWidget, vec![("name", "w1")]))
    );
    assert_eq!(
        find(Method::POST, "/v1/widgets/w1:reset"),
        Some((Op::ResetWidget, vec![("name", "w1")]))
    );
    assert_eq!(find(Method::POST, "/v1/widgets/:reset"), None);
    assert_eq!(find(Method::PATCH, "/v1/widgets/w1"), None);
    assert_eq!(find(Method::GET, "/v1/widgets/w1/extra"), None);
    assert_eq!(find(Method::GET, "/v1/gadgets"), None);

    // Literal segments and AIP-136 `:verb` suffixes take precedence over bare
    // `{param}` placeholders even when listed later.
    let shadowed = [
        Route::new(Method::GET, "/v1/widgets/{name}", Op::GetWidget),
        Route::new(Method::GET, "/v1/widgets/all", Op::ListWidgets),
        Route::new(Method::POST, "/v1/widgets/{name}", Op::CreateWidget),
        Route::new(Method::POST, "/v1/widgets/{name}:reset", Op::ResetWidget),
    ];
    assert_eq!(
        find_route(&shadowed, &Method::GET, "/v1/widgets/all").map(|(r, _)| r.op),
        Some(Op::ListWidgets)
    );
    assert_eq!(
        find_route(&shadowed, &Method::POST, "/v1/widgets/w1:reset").map(|(r, p)| (r.op, p)),
        Some((Op::ResetWidget, vec![("name", "w1")]))
    );
}

#[test]
fn test_operation_ids_are_op_names() {
    let ids: Vec<String> = ROUTES.iter().map(Route::operation_id).collect();
    assert_eq!(ids, ["ListWidgets", "CreateWidget", "GetWidget", "DeleteWidget", "ResetWidget"]);
}

#[test]
fn test_handle_serves_document_and_dispatches() {
    assert_eq!(handle(&Method::GET, "/v1/openapi.json", b""), (200, spec()));

    let example =
        &spec()["paths"]["/v1/widgets"]["post"]["requestBody"]["content"][JSON]["example"];
    let body = serde_json::to_vec(example).unwrap_or_default();
    let (status, created) = handle(&Method::POST, "/v1/widgets", &body);
    assert_eq!(status, 200);
    assert_eq!(&created, example);

    assert_eq!(handle(&Method::POST, "/v1/widgets", b"{}").0, 400);
    assert_eq!(
        handle(&Method::GET, "/v1/widgets/w2", b""),
        (200, json!({ "name": "w2", "color": "RED" }))
    );
    assert_eq!(
        handle(&Method::GET, "/v1/widgets", b""),
        (200, json!({ "widgets": [{ "name": "w1", "color": "RED" }] }))
    );
    assert_eq!(handle(&Method::POST, "/v1/widgets/w2:reset", b""), (200, json!({})));
    assert_eq!(handle(&Method::DELETE, "/v1/widgets/w2", b""), (200, json!({})));
    assert_eq!(handle(&Method::GET, "/v1/gadgets", b"").0, 404);
}
