# bmo-voc

The customers' voice box (고객의 소리, VOC). For the people who use the platform, the platform's maker is the
maker: a person who meets a fault, or wants something changed, writes the maker a report. bmo-voc is where such a
report is written, carried and kept, and where the maker reads, judges and answers it. The unit is the master of
its own data, the reports with their attachments, their states and their replies, in its own SQLite store and file
folder. `docs/api.md` is its boundary.

The owner's words that set it up, on 2026-11-23:

> 고객이 제조사에게 버그 이슈 리포트나 건의 사항 송부하는 기능 만들고 우리는 voc 받는 서비스 세워놔라. 고객의 소리함.
> voc send 는 슈퍼어드민은 필요없는거 알지?

## The two sides

- **The sender** is every signed-in person who is not a platform administrator (bmo-web `docs/accounts.md`, 「The
  platform administrators」): a customer of the maker, of whichever company. The sender writes reports and reads
  their own, with the maker's answers, and answers back.
- **The maker** is the platform administrators. They read every report of every company, move a report through its
  states and answer it. A platform administrator never sends a report: they are the maker's side, and the send
  side is closed to them.
- **The side is the console's word on the reader's role,** the role the console hands every call: `administrator`
  is the maker, `member` the sender. Nothing else decides it.

## A report

- **What it holds:** its kind, a title, a body written in markdown, who sent it, the company they sent it from,
  where they sent it from, its attachments and the moment it was sent.
- **Its kind** is a bug, a fault the sender met, or a suggestion, something the sender wants changed or added.
- **Who sent it** is the sender's account, by the account service's user id, and the name the account service
  gave that person as the report was sent; **the company** is the company the account service placed the sender in
  then, by its unit id and its name. The names are written into the report as it is sent and kept with it, because
  the maker reads the report as a letter: it reads who wrote it as they signed, and a maker that stands apart from
  the sender's deployment (「A maker elsewhere」) has no account service of the sender's to ask.
- **Where it was sent from** is what the sender's console says of itself as the report is written: the console
  screen's path the sender came from, the console's version, and the browser it runs in, each as the screen read
  it, each possibly empty.
- **Attachments** are screenshots and other files, a few to a report and each under a size the settings bound. They
  are kept as their bytes, beside the name and the type the sender's browser gave them, and handed back to whoever
  may read the report. A picture is shown inline; any other file, and a picture of a type a browser could run, is
  handed as a download.
- **A report is never changed or removed** once sent. What changes is its state, and what grows is its thread.

## Its state

- **The maker's judgement of the report** is one of four:
  - **received** (접수), as it arrives;
  - **under review** (검토 중), while the maker looks into it;
  - **done** (완료), answered or carried out;
  - **declined** (반려), not to be carried out.
- **The maker moves it,** any administrator, from any state to any other. A change to the state it already stands
  in changes nothing.
- **Every change is kept,** with who made it, by user id and name as the account service names them, and when, so
  the report's history reads from its arrival to its state now. The arrival is the history's first entry, made by
  the sender.
- **The sender reads the state and its history** and never moves it.

## Replies

- **A report carries one thread.** The maker answers it, and the sender answers back on the same report; each reply
  is markdown, from one side, with its writer's user id and name and its moment. Any administrator answers for the
  maker. A reply is never changed or removed.
- **A reply moves no state.** The maker answers and moves the state as two acts.
- **Either side may write at any state,** a done or declined report included, so a sender who disagrees answers
  on the report rather than writing a new one.

## What each side has not read

- **Each side keeps one mark per report:** the moment that side last opened it. The maker's mark is the maker's as a
  whole, so a report one administrator opened is read for every administrator.
- **A report is unread by a side** where the other side acted on it after that side's mark: for the maker, the
  report's arrival or a reply of the sender's; for the sender, a reply of the maker's or a change of its state.
- **The unread count** is the number of reports unread by the reader's side, the sender's own reports alone for a
  sender. The console draws it on the entry (「The console」).

## Notices

The console raises a notice from the unit's events (bmo-ui `docs/console-bundles.md`, 「Notices」), each to the
readers the manifest names for its kind:

- **a report received** and **a sender's reply** ring the platform administrators;
- **a maker's reply** rings the sender of the report alone.

A state change rings nobody; the sender reads it on the report and in the unread count. A notice opens the report.

## Who reads

- **A sender reads their own reports alone,** by the user the console names; another's report is answered as if it
  did not stand. A head of a unit reads nothing of the people under it.
- **The maker reads every report** of every company, and filters the inbox by kind, state and company.
- **Nothing of a report is read by a caller who is neither,** but the service token, which reads every report and
  writes nothing.

## A maker elsewhere

The maker and the senders share one deployment today, so this one service is both the sender's road and the
maker's inbox. A customer may later run a deployment of their own, whose people still write to the same maker.
What that asks of this unit is kept true now:

- **The send side reaches the maker through the report routes alone.** A report is sent, read, answered and marked
  read through the routes `docs/api.md` names, and the sender's screen calls nothing else, so one configured maker
  address standing in front of those routes is the whole of what a deployment apart from the maker would change.
- **A report carries what the maker needs to read it without the sender's deployment:** the sender's and the
  company's names as well as their ids, and where it was sent from.
- **Not built now:** the setting naming a maker elsewhere, the relaying of a sender's calls to it, the maker's
  answers travelling back, and how two deployments trust each other.

## The console

The console is where a person signs in and sees every unit's screen, and how each unit reaches it is the console
plugin contract (bmo-ui `docs/console-bundles.md`). This unit serves its console face through bmo-chat's
`bmo-console-face`, and its browser bundle is built from the Nuxt project under `client/`.

- **One entry, placed by the reader's role.** The bundle's entry is 고객의 소리. For a sender it stands in the
  console's user menu, the reader's own menu (the contract's `account` section), since sending the maker a word is
  about the reader's own use of the console rather than an application they work in; for the maker it stands among
  the 관리자 서비스 (the contract's sections by role). So a platform administrator never meets the send side.
- **One screen path, two faces.** `/voc` opens the sender's own reports for a sender and the inbox for the maker;
  `/voc/{report}` opens one report's thread for either. The screen asks the service which side the reader is on.
- **The screen speaks the console's language.** Its words are the bundle's pack in English, Korean, Japanese and
  Simplified Chinese, one file to a locale (`en`, `ko`, `ja`, `zh-CN`), every file holding the same keys.

### The sender's screen

- **The list of their own reports,** newest activity first: each with its kind, its title, its state, when it was
  sent, how many replies it holds and a mark where the maker acted since the sender last opened it.
- **새 의견 보내기** opens the form: the kind, the title, the body in markdown, and screenshots or files attached,
  with the screen the sender came from, the console's version and the browser taken as the form opens. Sending
  opens the report sent.
- **A report's thread:** its body, its attachments, its state with its history, the replies of both sides in order,
  and a box to answer the maker.

### The maker's screen

- **The inbox:** every report, newest activity first, filtered by kind, state and company, each row with its kind,
  title, sender, company, state, when it was sent and a mark where it is unread for the maker.
- **Both lists, the sender's reports and the inbox, are read fifty at a time, the next fifty as the list's end
  comes into view** (bmo-ui's `NextPageFoot`, its `docs/design.md`, 「Lists」): placeholder rows stand at the end
  while a page is on its way, a page that fails leaves the reports already read and stands its reason with a retry
  in its place, and nothing stands under the last report. Nothing is pressed to read more.
- **A report's thread:** its body, where it was sent from, its attachments, its state with its history and the
  control that moves it, the replies of both sides, and a box to answer the sender.

## The store

One SQLite file and one folder of attachment bytes, both this unit's alone. The file holds the reports, their
attachments' names, types and sizes, their state histories, their replies and each side's read mark; the folder
holds each attachment's bytes under its id. The file's schema is moved by numbered migrations, applied in order as
the service opens the store, each once.
