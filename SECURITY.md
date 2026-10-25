# Security policy

## Supported versions

Only the latest release gets fixes.

## Reporting a vulnerability

Use [private vulnerability reporting](https://github.com/dodoflix/blkbstr/security/advisories/new). Please do not open a public issue for a security problem.

Include what you found, how to reproduce it and the version. You will get a reply within a week.

## Scope and expectations

Blockbuster is two programs: an unprivileged GUI and `blkbstrd`, a service that runs as root, owns the firewall rules and the engine process, and is reached over a local socket. Anyone who can write that socket can change the firewall, so socket permissions are the authorisation model.

Reports about these are welcome: a way to reach the daemon without the permissions that should be needed, a config value or name that escapes validation and reaches the engine parameter file or the filesystem, rules left behind after the daemon stops, and anything that sends data off the machine (there is no telemetry and there must not be).

Reports that a user with write access to the socket can do what the daemon is for are known.
