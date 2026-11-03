# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

Developers and power users who work with AI agents and also browse the web themselves.

## Product Purpose

An open-source, Linux-first browser that lets a person browse while AI agents carry out separate browser tasks.

## Positioning

Personal browsing stays in its own profile while agents operate in isolated Spaces through local MCP tools. The user explicitly controls whether sign-in cookies are copied into an agent Space.

## Operating Context

The user works in a Linux desktop environment. They may keep using their personal tabs while agents work in the background, inspect an agent Space, take over, or stop it.

## Capabilities and Constraints

- The shipped product is a Linux-first Electron desktop browser. Impeccable's `web` platform value describes its HTML/CSS-rendered interface; there is no mobile app target.
- The current implementation uses Electron and Chromium, with a local MCP server and loopback API.
- Agent Spaces are isolated and ephemeral. Cookie copying from Personal is optional, explicit, and one-time; localStorage is not copied.
- Personal browsing data remains in the browser's local profile. Agent page content is untrusted input.
- Linux is the first target. Existing browser behavior and standard browser affordances must remain usable.
- The product is intended to be open source; the distribution license has not yet been selected.
- Current page snapshots cover the main document only; iframe support is not implemented yet.

## Brand Commitments

“Open Browser” is the current working name, not a confirmed final brand. No visual identity, palette, or type system has been approved.

## Evidence on Hand

The project has a working Electron prototype, agent MCP integration, and headless smoke tests. No customer logos, testimonials, external benchmarks, or other promotional evidence have been supplied; do not invent them.

## Product Principles

- Keep personal browsing separate from agent work by default.
- Make agent access and session sharing explicit and user-controlled.
- Keep browser and agent communication local to the user's machine.
- Preserve familiar browser navigation and let the page remain the main event.
