# The boundary API

Every route bmo-voc answers, the events it publishes, the calls it makes and its settings. The wire format is the
platform's (bmo-chat `docs/conventions.md`): camelCase keys, times in epoch milliseconds, ids as strings, a failure a
non-2xx status with `code` (kebab-case) and `error` (an English sentence), a list `{items, more}` read with `after`
(the number of rows read so far) and `limit` (50 where none is given, never past 200). A person is the account
service's user id and a company the account service's unit id, each written as a string. `docs/openapi.json` is
written from the service's own types by `bmo-voc openapi` (`bmo-voc openapi -` prints it instead), and a test fails
where the committed document differs.
`docs/design.md` states what every word below means.

## Who calls

The service listens on the loopback at the address its settings name (`[server] listen`, by convention
`127.0.0.1:13134`), under `Authorization: Bearer <token>`. A caller is one of two:

| Caller | Credential and headers | Acts as |
|---|---|---|
| the console | the console credential (`[console] token`), `Owner-Key` and `Owner-Role` (`administrator` or `member`) | the signed-in person: the maker for `administrator`, a sender for `member` |
| a reader of everything | the service token (`[server] token`) alone | nobody: it reads every report and writes nothing |

- **Refusals of the caller:** no credential or a wrong one is 401 `unauthorized`; a console call with no
  `Owner-Key`, a blank one or one longer than 200 characters, or an `Owner-Role` that is neither word is 403
  `forbidden`; a write by a reader of everything, a read mark among them, is 403 `forbidden`.
- **Refusals of the side:** a report sent by the maker is 403 `maker-side`; a state moved by a sender is 403
  `sender-side`.
- **What a caller reads** is `docs/design.md`, 「Who reads」. A sender's call naming another's report, or one of its
  attachments, is answered 404 `not-found`, as if it did not stand; so is a report that does not stand.

## Words on the wire

| What | Words |
|---|---|
| a report's kind | `bug`, `suggestion` |
| a report's state | `received`, `under-review`, `done`, `declined` |
| a side, on a state's entry and a reply | `maker`, `sender` |

## The reader

| Method | Path | Does |
|---|---|---|
| GET | `/me` | `user` (null for a reader of everything), `side`: `maker`, `sender`, or `reader` for a reader of everything, and `limits`: what one report may carry, `maxFiles` and `maxBytes` as the settings' `[attachments]` bound them |

## Reports

| Method | Path | Does |
|---|---|---|
| GET | `/reports` | The reports the caller reads, newest activity first — `{items, more}`, each a report's summary; the maker and a reader of everything filter by `kind`, `state` and `company` (a company's id), each optional, and a sender's call that names one is answered as if it did not; a kind or a state that is no word of the wire is 400 `bad-request` |
| POST | `/reports` | A sender sends a report, as `multipart/form-data`: one part `report`, a JSON object of at most 1 MiB holding `kind`, `title`, `body` and `context` (`path`, `consoleVersion`, `browser`, each a string; the object and each field optional, empty where the screen read none), and a part `files` for each attachment, its file name and type as the browser gave them — 201, the report; a body that is not a form, or a part of another name, is 400 `bad-request` |
| GET | `/reports/{reportId}` | One report whole |
| GET | `/reports/{reportId}/attachments/{attachmentId}` | An attachment's bytes, with its file name; a picture of a type a browser shows (PNG, JPEG, GIF, WebP) is answered inline under that type, any other file under the type it was kept with as an attachment to save; the file name is written both plain, each character a header cannot carry as `_`, and in full as UTF-8; every answer carries `X-Content-Type-Options: nosniff` |
| POST | `/reports/{reportId}/replies` | `body`: a reply from the caller's side — 201, the reply |
| PUT | `/reports/{reportId}/state` | `state`: the maker moves the report — 200, the report; the state it already stands in changes nothing and keeps no entry |
| PUT | `/reports/{reportId}/seen` | The caller's side's mark set to now — 204 |
| GET | `/companies` | The companies the reports were sent from, each `id` and `name` as the latest report from it names it, by name and then by id — `{items, more}`, `more` always false; a report sent from no company names none here; the maker and a reader of everything alone, a sender answered 403 `sender-side` |
| GET | `/unread` | `count`: the reports unread by the caller's side; 0 for a reader of everything |

- **A report's summary** is `id`, `kind`, `title`, `state`, `sender` (`user`, `name`), `company` (`id`, `name`, or
  null where the sender stood in no company), `sentAt`, `activityAt` (its latest act of either side: its arrival, a
  reply or a state change), `replies` (how many) and `unread` (whether it is unread by the caller's side; false for a
  reader of everything).
- **A report whole** is the summary and `body`, `context` (`path`, `consoleVersion`, `browser`), `attachments` (each
  `id`, `name`, `type`, `size` in bytes), `states` (each `state`, `by` (`user`, `name`, `side`) and `at`, oldest
  first, the arrival first) and `replies` (each `id`, `side`, `by` (`user`, `name`), `body` and `at`, oldest first);
  its `replies` is the list there, never the count.
- **A report's fields:** `title` is 1 to 200 characters once trimmed, and kept trimmed; `body` 1 to 20,000
  characters, and a reply's `body` the same, each refused where it is blank and kept as written. At most `[attachments] max_files` files, each from 1 byte to `[attachments] max_bytes`, with
  a file name of 1 to 200 characters; a type the browser named none of is kept as `application/octet-stream`.
- **A report is sent whole or not at all:** the report, its files and its arrival entry are written in one step, and
  a report refused for a file keeps nothing of the others. The caller and its side are judged before the form is
  read; the files' count and sizes as their parts come, the first part past either bound refusing the form; the
  `report` part once the whole form is read.
- **A sender's name and company** are read from the account service as the report is sent, and so is each writer's
  name as a reply or a state change is written (「Calls the voc unit makes」).

## Events the voc unit publishes

`GET /events` is a server-sent stream in the platform's shape (`after` or `Last-Event-ID`, `kinds`; the same events as
a page to `Accept: application/json`, with `after` and `limit`). Each event's data is `reportId`, `sender` (the
sender's user id), `title` and `kind`:

| Kind | When | Beside the data |
|---|---|---|
| `report.received` | a sender sent a report | |
| `report.answered` | the maker replied | |
| `report.followed` | the sender replied | |
| `report.changed` | the maker moved its state | `state` |

An event reaches the maker and a reader of everything always, and a sender for their own reports alone. The service
keeps the events since it started, at most the last 1000, numbered upward from its start's moment in milliseconds
times 1000; a resume from before them is 410 `gone`.

## Console

The service brings the 고객의 소리 screen to a console under bmo-ui's console plugin contract (bmo-ui
`docs/console-bundles.md`). Its console face is bmo-chat's `bmo-console-face` (bmo-chat `docs/console-face.md`), which
answers `GET /console-manifest` and every file of the bundle under `GET /console/{file}`, guards them, and lays the
same guard over the routes the screens call. This service supplies the face:

- **its declaration** of the manifest fields it decides: name `voc`, title `voc.title`, icon
  `i-material-symbols-campaign-outline`, screen path `/voc`, service name `voc`, API prefix `/`, role `member` and the
  owner key; the entry in the `account` section, and in `administration` for an `administrator` (the contract's
  sections by role); events (the stream above), with the notice kinds `report.received`, `report.answered` and
  `report.followed`, each opening `/voc/{reportId}`, the first and the last read by the readers holding the
  `administrator` role and `report.answered` by the account its `sender` names; the sidebar count read from
  `/unread` under `count`; no calls, socket, relays or share reads;
- **the bundle build's folder** (`[console] folder`), as bmo-ui's bundle build writes it: `bundle.js`, `bundle.css`,
  `words.json` and `layer.json`, read once at start;
- **the console's credential** (`[console] token`).

Where the credential is empty, or the folder cannot be served, no face stands and the console's routes answer 404
`no-route`; the service's own routes are answered as ever. `bmo-voc console-reach` prints, as one JSON object,
`address`, `consoleToken` and `token`, for a deployment's console probe (bmo-testbed `docs/deploy.md`, 「The console
probe」).

## Calls the voc unit makes

| What | Route |
|---|---|
| a person's name, as a report is sent and as a reply or a state change is written | account service `GET /users/{user}`: `fullName`, or `name`, the login, where it is empty |
| a sender's company, as a report is sent | account service `GET /users/{user}/unit`: `company`, or none where it is null or the account service answers 404 |

- **The credentials.** The account service's are its `[server] listen` and `token` from the data root's
  `env/web.toml`, read afresh for every call, each overridden by the setting that names it. Each call carries
  the caller's `Owner-Key` and `Owner-Role` as the console handed them.
- **Faults.** A call waits for its answer; no clock cuts one. A read that fails, or answers anything but a success
  or, for the unit, the 404 that says the person stands in none — a person the account service does not know among
  them — fails the request that needed it with 502
  `upstream-failed`, and nothing of it is written.

## Health

| Method | Path | Does |
|---|---|---|
| GET | `/health` | `ready`, `reason` (null when ready), `version` |

The service is ready while its store is open and its attachment folder stands, and answers the reason where either
does not; no upstream stands between it and
readiness. `bmo-voc health` asks it at the address its settings name and answers 0 where it is ready.

## Codes

Beside the common codes of bmo-chat's `docs/conventions.md` (`bad-request`, `unauthorized`, `forbidden`, `no-route`,
`not-found`, `conflict`, `gone`, `failed`, `upstream-failed`, `not-ready`):

| Code | Status | When |
|---|---|---|
| `maker-side` | 403 | a report sent by the maker |
| `sender-side` | 403 | a state moved, or the companies read, by a sender |
| `bad-report` | 422 | a report's or a reply's field out of its bounds, an unknown kind or state, or a `report` part missing or not the JSON it names |
| `too-many-files` | 413 | more files than `[attachments] max_files` |
| `file-too-large` | 413 | a file past `[attachments] max_bytes`, or an empty one |

## Settings

The settings are read at start in layers, each overriding the one before: the defaults built into the binary (the
repository's `settings.toml`); then `settings.toml` and `settings.local.toml` in the settings folder, the one
`BMO_VOC_SETTINGS` names, else the folder above the one holding the binary (a deployment's root, where
`settings.local.toml` links to the data root's `env/voc.toml`); then `BMO_VOC_<TABLE>__<KEY>` in the environment.

| Table | Key | What it is |
|---|---|---|
| `server` | `listen` | where the service answers, `host:port` |
| `server` | `token` | the service token; an empty one lets no reader of everything in |
| `server` | `data_root` | the data root whose `env/` the account service's address and token are read from |
| `console` | `token` | the console credential; empty, no console face stands |
| `console` | `folder` | the console bundle's built folder; one that cannot be served leaves no console face |
| `store` | `path` | the SQLite file of reports, states, replies and read marks |
| `store` | `files` | the folder of attachments' bytes |
| `attachments` | `max_files` | how many files one report carries at most |
| `attachments` | `max_bytes` | how large one file may be, in bytes |
| `accounts` | `address`, `token` | the account service's where `env/web.toml` is not read; empty, the file is read |
