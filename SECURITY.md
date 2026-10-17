# Security Policy

## Supported versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | Yes |
| < 0.1   | No  |

## Reporting a vulnerability

Please report security issues **privately** through GitHub:

1. Open the repository's **Security** tab.
2. Click **Report a vulnerability**.
3. Describe the issue in the draft advisory.

That is the only reporting channel. Do not open a public issue for a suspected vulnerability, and do not include exploit details in a public discussion.

We aim to acknowledge reports within a few working days and will coordinate a fix and disclosure timeline with you.

## What to include

- A description of the issue and the impact you believe it has.
- Steps to reproduce, or a minimal proof of concept.
- The LlamaDesk version or commit, and your OS and desktop environment.
- The relevant configuration (server URL scheme and host — redact anything sensitive).
- Any logs or error messages, with secrets removed.

## Scope

LlamaDesk is a **read-only local observer**. Reports are most useful when they show a way to break one of these invariants:

- **No listening socket.** The app binds no network port. It only makes outbound HTTP requests.
- **Backend-only networking.** All requests to the inference server are made by the Rust backend. The webview never contacts the server directly, and a Content Security Policy restricts what the webview can load or connect to.
- **User-configured target.** The only outbound destination is the llama.cpp URL you configure (default `http://localhost:8080`), validated as an absolute `http`/`https` URL.
- **Read-only.** The app never writes to, starts, stops, or reconfigures the server, and it never executes content reported by it.
- **Local data only.** History is stored in a local SQLite database; configuration is a local JSON file. Nothing is sent anywhere else.

In scope, for example: a way to make the app contact a host other than the configured one, a way for server-reported data to escape into code execution or HTML injection in the webview, or a path-traversal or injection issue in local file or database handling.

## Non-issues

The following are expected behaviour, not vulnerabilities:

- **The app displays what the server reports.** Model names, metric values, and error strings come from the configured server and are treated as untrusted display data. A confusing or hostile value is the server's content, not a LlamaDesk flaw.
- **Pointing the app at a malicious server.** The target URL is user-controlled configuration; choosing an untrusted server is the user's decision, not an attack on the app.
- **Missing `nvidia-smi` or no GPUs.** This is a normal degraded state: GPU pages report that monitoring is unavailable and server monitoring continues.
- **HTTP instead of HTTPS to localhost.** The default target is a local server; transport security for remote servers is the operator's responsibility.
- **No authentication.** LlamaDesk has no accounts or privilege boundary of its own, so there is nothing to escalate within the app.

If you are unsure whether something is in scope, report it privately and we will triage it.
