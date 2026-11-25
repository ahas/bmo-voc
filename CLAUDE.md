# Working in this repository

This repository is bmo-voc: the customers' voice box (고객의 소리). A person who uses the platform sends its maker a
bug report or a suggestion; the platform administrators, the maker's side, read every report, move it through its
states and answer it. The unit is the master of its own data — the reports, their attachments, their state
histories, their replies and each side's read marks — in its own SQLite store and attachment folder.
`docs/design.md` states what it is; `docs/api.md` is the boundary contract.

## Rules

- **`docs/` is canonical.** A change that would make a document wrong repairs the document first.
- **The unit owns its data.** Everything it masters stands in its own store and folder and nowhere else.
- **The side is the console's role.** `administrator` is the maker and never sends a report; `member` is a sender
  and reads their own reports alone, answered 404 for any other. No head of a unit reads the reports of the people
  under it.
- **A report carries the names it was signed with.** The sender's name and company, and each writer's name on a
  state change or a reply, are read from the account service as they are written and kept with them, so the maker
  reads a report without the sender's deployment (`docs/design.md`, 「A maker elsewhere」). A peer that does not
  answer fails that request, saying so, and writes nothing.
- **A report is never changed or removed;** its state moves and its thread grows, each act kept with who and when.
- **The sender's road is the report routes alone,** so a maker elsewhere is reached by putting one address in front
  of them; the sender's screen calls nothing else.
- **No clock bound on any request.** A call to a peer waits for its answer.
- **Tests live in `tests/`,** one harness `tests/main.rs`, over a stand-in of the account service that records the
  calls it takes; a test waits on a recorded call or an event, never on the clock. Checks: the testbed's own Cargo
  workspace check, which formats and lints the crate with every target compiled and every warning an error, and its
  change check, which compiles it, neither declared in `testbed.toml`; `cargo test` through the testbed; and the
  console bundle's own `bun run check` in `client/`.
- **English only** in code, documents, tests and commits; the only Korean stands in `docs/design.md` as the owner's
  quoted words and the screen's words it names, and in the bundle's `ko` words; Japanese and Chinese stand in the
  bundle's `ja` and `zh-CN` words alone, besides the README's translations.
- **No personal data, no names of agents, owners, hosts or companies, and no public addresses** in code, documents
  or commits; a public name, a port and a credential are deployment settings (`PERSONAL-DATA.md`).
- **Versions:** the set's, read off the git tags when a build is made (bmo-chat's `docs/conventions.md`,
  「Versions」); the manifests' version fields are placeholders that never move; no sibling version pins.
- Depend only on registry crates and the base libraries (bmo-console-face, taken by path, and bmo-ui); the console
  bundle vendors the bmo-ui release bmo-chat's `shared-versions.toml` pins, as its release file stands, beside its
  own lockfile; other units through their documented APIs.
- **The console face is bmo-console-face's** (bmo-chat `docs/console-face.md`): the service hands it its
  declaration, its built folder and the console's credential, and keeps no manifest, bundle route or console guard
  of its own.
- **The console bundle keeps to the console plugin contract** (bmo-ui `docs/console-bundles.md`): its screens read
  this unit's API through the console's relay and nothing else.
- **The screens are held in a browser** by `client/tests/browser/`, over the bundle's dev host and a stand-in of the
  service: what is drawn and reachable at a desktop's width and a phone's, never a component's state.
