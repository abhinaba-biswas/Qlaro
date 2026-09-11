# Qlaro

Most data workflows are a mess of fragmented cloud tools, operational silos, and ballooning compute bills. Data engineers live in one ecosystem; business analysts live in another.

This project is a unified, local-first data operating system built to fix that. It consolidates the entire data lifecycle into a single cohesive environment. We ripped out the bloated web apps and replaced them with highly optimized native interfaces, backed by an engine-agnostic architecture that lets you scale from local prototyping to out-of-core processing without changing a single line of business logic.

## The Interfaces

We believe tools should be fast, native, and tailored to the people actually using them.

* **For Engineers (TUI):** An ultra-responsive, keyboard-centric terminal UI built with [Ratatui](https://github.com/ratatui/ratatui).
* **For Analysts (GUI):** A hardware-accelerated desktop app built with [Iced](https://github.com/iced-rs/iced) for visual, drag-and-drop analytics.
* **For the Future (Web):** Full web deployment is on the roadmap for team flexibility, but local-first native clients are the priority.

## Architecture: "Freemium Compute"

Under the hood, user intent is strictly separated from physical execution.

When you interact with the UI—whether in the terminal or the desktop app—you aren't running direct queries. Instead, your actions generate a neutral, framework-agnostic **query plan**. This plan is handed off to a central routing gateway, which dynamically assigns the workload based on your dataset size and license tier.

### Two Engines, One Standard

1. **The Standard Engine (Python):** For everyday tasks and local prototyping, the gateway routes the plan to our standard Python execution engine. It's completely free.
2. **The Heavy-Lift Engine (Rust):** For massive, out-of-core data crunching, a cryptographically signed license unlocks the premium tier. The gateway routes these heavy workloads to our parallelized, multi-threaded Rust engine.

### Why build it this way?

Both execution engines read from the exact same standardized storage formats and output universal **Apache Arrow** memory structures. This means the handoff is completely invisible to the user.

The analytical results, the UI state, and the application behavior remain 100% consistent across tiers. You can build, test, and prototype for free on your local machine, and seamlessly scale to enterprise-grade crunching when you need it—without ever rewriting your dashboards, migrating your logic, or paying for idle cloud compute.
