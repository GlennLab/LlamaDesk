## What and why

<!-- What does this change do, and why is it needed? -->

## Related issue

<!-- e.g. Closes #12 -->

## Change scope

- [ ] The change is surgical: it touches only what the task requires, with no drive-by refactors or unrelated reformatting.

## Gates run locally

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `npm run test:run`
- [ ] `npm run build`

## Safety

- [ ] No secrets, tokens, API keys, signing keys, or credentials were added.
- [ ] The read-only invariant is intact: no new outbound target beyond the configured llama.cpp URL, no new listening port, and the frontend still does not talk to the server directly.

## Docs and screenshots

- [ ] Any documentation or screenshots are real captures of the running app — not mocked or edited to imply behaviour that does not exist.
- [ ] No mock data was added to the app.
