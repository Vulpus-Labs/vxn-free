# tickets

One markdown file per ticket in `open/`, `closed/` or `deferred/`, named
`NNNN-slug.md`. Epics live alongside in `epics/{open,closed,deferred}/`,
named `ENNN-slug.md`. Architectural decisions live in `adrs/NNNN-slug.md`,
one per topic.

- Ticket numbers come from a single counter; take the next free number.
- A ticket moves from `open/` to `closed/` when its acceptance criteria are
  met. It gains a `## Close-out (YYYY-MM-DD)` section recording what landed,
  where, and any deviation from the plan.
- A change that alters a decision updates (or supersedes) the relevant ADR
  in the same change.
- `depends:` lists tickets that must close first. Work tickets in
  dependency order within an epic.
