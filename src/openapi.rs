//! The OpenAPI document, `docs/openapi.json`: every route the service answers, with its parameters, its body, its
//! answers and the codes it can give, the schemas generated from the service's own types, and the events it
//! publishes.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use schemars::generate::{SchemaGenerator, SchemaSettings};
use serde_json::{Map, Value, json};

use crate::error::CODES;
use crate::events::{ANSWERED, CHANGED, FOLLOWED, KINDS, RECEIVED};
use crate::wire::{
  Company, EventsQuery, Health, Me, NewReply, NewReport, Page, Reply, Report, ReportChanged, ReportEvent,
  ReportSummary, ReportsQuery, StateWrite, StreamEvent, Unread, WireError,
};

const SCHEMAS: &str = "/components/schemas";

const DESCRIPTION: &str = "The platform's customers' voice box. A person who uses the platform sends its maker \
  a bug report or a suggestion, with screenshots and other files; the platform administrators, the maker's side, \
  read every report of every company, move it through its states and answer it, and the sender answers back on \
  the same report. It keeps the reports, their attachments, their state histories, their replies and each side's \
  read mark in its own store, and reads the account service for each writer's name and a sender's company. \
  Bodies are JSON with camelCase keys, but a report sent, which is multipart form data; times are epoch \
  milliseconds, ids are strings, lists are pages of items and more, and every failure is the error shape with \
  its status.";

struct Route {
  method: &'static str,
  path: &'static str,
  id: &'static str,
  tag: &'static str,
  summary: &'static str,
  parameters: Vec<Value>,
  /// The request body's content, by media type.
  body: Option<Value>,
  answers: Vec<(u16, Option<Value>)>,
  codes: Vec<&'static str>,
}

fn error_answer(error: &Value, codes: &[&str]) -> Value {
  let description = codes
    .iter()
    .map(|code| {
      let meaning =
        CODES.iter().find(|(name, ..)| name == code).map(|(.., meaning)| *meaning).unwrap_or("");
      format!("`{code}`: {meaning}")
    })
    .collect::<Vec<_>>()
    .join("\n");
  json!({ "description": description, "content": { "application/json": { "schema": error } } })
}

impl Route {
  fn operation(self, error: &Value) -> Value {
    let mut codes = vec!["unauthorized", "forbidden"];
    codes.extend(self.codes);
    codes.sort_by_key(|code| CODES.iter().position(|(name, ..)| name == code));
    codes.dedup();
    let mut statuses: BTreeMap<u16, Vec<&str>> = BTreeMap::new();
    for code in &codes {
      let status = CODES
        .iter()
        .find(|(name, ..)| name == code)
        .map(|(_, status, _)| *status)
        .unwrap_or(500);
      statuses.entry(status).or_default().push(code);
    }
    let mut responses = Map::new();
    for (status, content) in self.answers {
      let description = match status {
        201 => "Created.",
        204 => "Done; no content.",
        _ => "Answered.",
      };
      let mut answer = json!({ "description": description });
      if let Some(content) = content {
        answer["content"] = content;
      }
      responses.insert(status.to_string(), answer);
    }
    for (status, codes) in statuses {
      responses.insert(status.to_string(), error_answer(error, &codes));
    }
    let mut operation = json!({
      "operationId": self.id,
      "tags": [self.tag],
      "summary": self.summary,
      "responses": responses,
      "x-codes": codes,
    });
    if !self.parameters.is_empty() {
      operation["parameters"] = self.parameters.into();
    }
    if let Some(body) = self.body {
      operation["requestBody"] = json!({ "required": true, "content": body });
    }
    operation
  }
}

fn content<T: JsonSchema>(g: &mut SchemaGenerator) -> Option<Value> {
  Some(json!({ "application/json": { "schema": g.subschema_for::<T>() } }))
}

fn path_parameter(g: &mut SchemaGenerator, name: &str, description: &str) -> Value {
  json!({
    "name": name,
    "in": "path",
    "required": true,
    "description": description,
    "schema": g.subschema_for::<String>(),
  })
}

fn query<Q: JsonSchema>(g: &mut SchemaGenerator) -> Vec<Value> {
  let schema = Q::json_schema(g).to_value();
  let required: Vec<&str> = schema["required"]
    .as_array()
    .map(|names| names.iter().filter_map(Value::as_str).collect())
    .unwrap_or_default();
  let Some(properties) = schema["properties"].as_object() else {
    return Vec::new();
  };
  properties
    .iter()
    .map(|(name, property)| {
      let mut parameter = json!({
        "name": name,
        "in": "query",
        "required": required.contains(&name.as_str()),
        "schema": property,
      });
      if let Some(description) = property.get("description") {
        parameter["description"] = description.clone();
      }
      parameter
    })
    .collect()
}

/// The form a report is sent as: its `report` part, JSON, and a `files` part for each attachment.
fn report_form(g: &mut SchemaGenerator) -> Option<Value> {
  Some(json!({
    "multipart/form-data": {
      "schema": {
        "type": "object",
        "properties": {
          "report": g.subschema_for::<NewReport>(),
          "files": {
            "type": "array",
            "description": "Each attachment, its file name and type as the browser gave them: at most the settings' `[attachments] max_files`, each 1 byte to `[attachments] max_bytes`, its name 1 to 200 characters.",
            "items": { "type": "string", "contentMediaType": "application/octet-stream" },
          },
        },
        "required": ["report"],
      },
      "encoding": { "report": { "contentType": "application/json" } },
    },
  }))
}

const REPORT: &str = "The report's id.";

fn routes(g: &mut SchemaGenerator) -> Vec<Route> {
  let report = |g: &mut SchemaGenerator| vec![path_parameter(g, "reportId", REPORT)];
  vec![
    Route {
      method: "get",
      path: "/health",
      id: "readHealth",
      tag: "health",
      summary: "Whether the service is ready, and its version.",
      parameters: vec![],
      body: None,
      answers: vec![(200, content::<Health>(g))],
      codes: vec![],
    },
    Route {
      method: "get",
      path: "/me",
      id: "readMe",
      tag: "reader",
      summary: "The reader: their user, and their side, the maker, a sender or a reader of everything.",
      parameters: vec![],
      body: None,
      answers: vec![(200, content::<Me>(g))],
      codes: vec![],
    },
    Route {
      method: "get",
      path: "/reports",
      id: "listReports",
      tag: "reports",
      summary: "The reports the caller reads, newest activity first: a sender's own alone; the maker and a reader of everything filter by kind, state and company, which a sender's call is answered as if it did not name.",
      parameters: query::<ReportsQuery>(g),
      body: None,
      answers: vec![(200, content::<Page<ReportSummary>>(g))],
      codes: vec!["bad-request"],
    },
    Route {
      method: "post",
      path: "/reports",
      id: "sendReport",
      tag: "reports",
      summary: "A sender sends a report with its files, whole or not at all; the sender's name and company are read from the account service as it is sent.",
      parameters: vec![],
      body: report_form(g),
      answers: vec![(201, content::<Report>(g))],
      codes: vec![
        "bad-request",
        "maker-side",
        "bad-report",
        "too-many-files",
        "file-too-large",
        "upstream-failed",
      ],
    },
    Route {
      method: "get",
      path: "/reports/{reportId}",
      id: "readReport",
      tag: "reports",
      summary: "One report whole; another sender's is not found.",
      parameters: report(g),
      body: None,
      answers: vec![(200, content::<Report>(g))],
      codes: vec!["bad-request", "not-found"],
    },
    Route {
      method: "get",
      path: "/reports/{reportId}/attachments/{attachmentId}",
      id: "readAttachment",
      tag: "reports",
      summary: "An attachment's bytes with its type and file name: a PNG, JPEG, GIF or WebP picture inline, any other file as an attachment to save, every answer with X-Content-Type-Options: nosniff.",
      parameters: vec![
        path_parameter(g, "reportId", REPORT),
        path_parameter(g, "attachmentId", "The attachment's id."),
      ],
      body: None,
      answers: vec![(200, Some(json!({ "*/*": { "schema": { "type": "string" } } })))],
      codes: vec!["bad-request", "not-found", "failed"],
    },
    Route {
      method: "post",
      path: "/reports/{reportId}/replies",
      id: "replyToReport",
      tag: "reports",
      summary: "A reply from the caller's side, the writer's name read from the account service; it moves no state.",
      parameters: report(g),
      body: content::<NewReply>(g),
      answers: vec![(201, content::<Reply>(g))],
      codes: vec!["bad-request", "not-found", "bad-report", "upstream-failed"],
    },
    Route {
      method: "put",
      path: "/reports/{reportId}/state",
      id: "moveReportState",
      tag: "reports",
      summary: "The maker moves the report to a state, the writer's name read from the account service; the state it already stands in changes nothing and keeps no entry.",
      parameters: report(g),
      body: content::<StateWrite>(g),
      answers: vec![(200, content::<Report>(g))],
      codes: vec!["bad-request", "not-found", "sender-side", "bad-report", "upstream-failed"],
    },
    Route {
      method: "put",
      path: "/reports/{reportId}/seen",
      id: "markReportSeen",
      tag: "reports",
      summary: "The caller's side's read mark on the report set to now.",
      parameters: report(g),
      body: None,
      answers: vec![(204, None)],
      codes: vec!["bad-request", "not-found"],
    },
    Route {
      method: "get",
      path: "/companies",
      id: "listCompanies",
      tag: "reports",
      summary: "The companies the reports were sent from, each named as the latest report from it names it, by name; the maker and a reader of everything alone.",
      parameters: vec![],
      body: None,
      answers: vec![(200, content::<Page<Company>>(g))],
      codes: vec!["sender-side"],
    },
    Route {
      method: "get",
      path: "/unread",
      id: "readUnread",
      tag: "reports",
      summary: "How many reports are unread by the caller's side; 0 for a reader of everything.",
      parameters: vec![],
      body: None,
      answers: vec![(200, content::<Unread>(g))],
      codes: vec![],
    },
    Route {
      method: "get",
      path: "/events",
      id: "readEvents",
      tag: "events-stream",
      summary: "The events the voc unit publishes, as a server-sent stream, or a page to Accept: application/json: every one to the maker and a reader of everything, a sender's own reports' alone to a sender.",
      parameters: {
        let mut parameters = query::<EventsQuery>(g);
        parameters.push(json!({
          "name": "Last-Event-ID",
          "in": "header",
          "required": false,
          "description": "The last event number seen, as a reconnecting stream sends it.",
          "schema": { "type": "integer", "format": "int64" },
        }));
        parameters
      },
      body: None,
      answers: vec![(
        200,
        Some(json!({
          "text/event-stream": { "schema": { "type": "string", "description": "Each event's data is a StreamEvent." } },
          "application/json": { "schema": g.subschema_for::<Page<StreamEvent>>() },
        })),
      )],
      codes: vec!["bad-request", "gone"],
    },
    Route {
      method: "get",
      path: "/console-manifest",
      id: "readConsoleManifest",
      tag: "console",
      summary: "The console face's manifest, by bmo-ui's console plugin contract, to the console's credential alone.",
      parameters: vec![],
      body: None,
      answers: vec![(200, Some(json!({ "application/json": { "schema": { "type": "object" } } })))],
      codes: vec!["no-route"],
    },
    Route {
      method: "get",
      path: "/console/{file}",
      id: "readConsoleFile",
      tag: "console",
      summary: "A file of the console bundle at the path the manifest names: the words to the console's credential alone, the module, the stylesheet and every other file to it with the manifest's Owner-Role and Owner-Key.",
      parameters: vec![path_parameter(g, "file", "The file's name, as the manifest gives it.")],
      body: None,
      answers: vec![(200, Some(json!({ "*/*": { "schema": { "type": "string" } } })))],
      codes: vec!["no-route", "not-found"],
    },
  ]
}

/// The OpenAPI document.
pub fn openapi() -> Value {
  let mut generator = SchemaSettings::draft2020_12()
    .for_serialize()
    .with(|settings| {
      settings.definitions_path = SCHEMAS.into();
      settings.meta_schema = None;
    })
    .into_generator();
  let g = &mut generator;
  let error = g.subschema_for::<WireError>().to_value();
  let mut tags: Vec<&str> = Vec::new();
  let mut paths = Map::new();
  for route in routes(g) {
    if !tags.contains(&route.tag) {
      tags.push(route.tag);
    }
    let (path, method) = (route.path, route.method);
    paths.entry(path).or_insert_with(|| json!({}))[method] = route.operation(&error);
  }
  let events = json!({
    RECEIVED: g.subschema_for::<ReportEvent>(),
    ANSWERED: g.subschema_for::<ReportEvent>(),
    FOLLOWED: g.subschema_for::<ReportEvent>(),
    CHANGED: g.subschema_for::<ReportChanged>(),
  });
  json!({
    "openapi": "3.1.0",
    "info": { "title": "bmo-voc", "version": env!("CARGO_PKG_VERSION"), "description": DESCRIPTION },
    "servers": [{ "url": "/", "description": "the voc service" }],
    "security": [{ "bearer": [] }],
    "x-layer": "downstream",
    "x-calls": ["the account service"],
    "x-events": KINDS,
    "x-event-data": events,
    "tags": tags.iter().map(|tag| json!({ "name": tag })).collect::<Vec<_>>(),
    "paths": paths,
    "components": {
      "schemas": generator.take_definitions(true),
      "securitySchemes": {
        "bearer": {
          "type": "http",
          "scheme": "bearer",
          "description": "The service token alone, a reader of everything that writes nothing, or the console's credential with Owner-Key and Owner-Role; the console bundle's routes take the console's credential alone.",
        },
      },
    },
  })
}

/// The document as the repository keeps it: two-space indented JSON with a closing newline.
pub fn openapi_json() -> String {
  let mut text = serde_json::to_string_pretty(&openapi()).expect("a JSON value always renders");
  text.push('\n');
  text
}
