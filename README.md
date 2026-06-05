# ternary-helm: Steering and control for fleet navigation

`Helm`, `Rudder`, `Tiller`, `Autopilot`, `HelmOrder`, and `HelmLog` — everything a room needs to pick a direction and stay on it in a ternary fleet topology.

## Why This Exists

In a fleet of rooms connected by ternary links (`-1`, `0`, `+1`), rooms need to move through the topology with intention. A room that drifts randomly is useless; a room that steers deliberately can reposition to fill gaps, avoid congestion, or chase objectives. This crate maps the familiar mechanics of ship steering — helm, rudder, tiller, autopilot — onto how rooms navigate a fleet.

## Core Concepts

- **Helm** — The top-level steering controller for a room. Owns the rudder, tiller, autopilot, and action log.
- **Rudder** — Directional control surface. Deflects to `-1` (port), `0` (amidships), or `+1` (starboard).
- **Tiller** — Manual override. When engaged, the autopilot disengages and the human (or agent) steers directly.
- **Autopilot** — Automated course keeping. Computes the correction needed to return to a target heading.
- **HelmOrder** — A command to change course, with a reason and timestamp. Applied through the helm.
- **HelmLog** — Append-only record of all helm actions for auditing and replay.
- **Heading** — Ternary direction: `Port` (-1), `Amidships` (0), `Starboard` (+1).

## Quick Start

```toml
[dependencies]
ternary-helm = "0.1"
```

```rust
use ternary_helm::{Helm, HelmOrder, Heading};

let mut helm = Helm::new(Heading::Amidships);

// Issue an order to turn starboard
let order = HelmOrder::new(Heading::Starboard, "avoid congestion");
helm.apply_order(&order);
assert_eq!(helm.heading(), Heading::Starboard);

// Manual override with the tiller
helm.grab_tiller(Heading::Port);
assert!(helm.tiller().is_engaged());
assert!(!helm.autopilot().is_active());

// Release back to autopilot
helm.release_tiller();
assert!(helm.autopilot().is_active());

// Check the log
assert_eq!(helm.log().len(), 2);
```

## API Overview

| Type | What it is |
|------|-----------|
| `Helm` | Top-level steering controller for a room |
| `Rudder` | Directional control surface with deflection angle |
| `Tiller` | Manual override that bypasses autopilot |
| `Autopilot` | Automated course keeping with correction computation |
| `HelmOrder` | A command to change heading, with reason and timestamp |
| `HelmLog` | Append-only audit trail of helm actions |
| `Heading` | Ternary direction enum (Port, Amidships, Starboard) |

## How It Works

The `Helm` is the single entry point. When a `HelmOrder` is applied, the helm updates its heading, re-targets the autopilot, deflects the rudder, and logs the action. The autopilot computes corrections as a ternary delta clamped to `[-1, +1]` — it won't overshoot.

The `Tiller` is a safety valve: when grabbed, it immediately disengages the autopilot and locks the heading to whatever the operator sets. This models emergency manual control. Releasing the tiller re-engages the autopilot with its last known target.

The `HelmLog` is a bounded circular buffer (default 1000 entries). When full, the oldest entry is dropped. This prevents unbounded memory growth in long-running rooms.

## Known Limitations

- Headings are strictly ternary (`-1, 0, +1`). No intermediate angles or continuous steering.
- Autopilot corrections are clamped to ±1 per step, so a course change from Port to Starboard requires at least two corrections.
- `HelmLog` uses `Instant` for timestamps, which is not serializable. For persistence, wrap entries with your own timestamp type.
- No concept of speed or momentum — only direction.

## Use Cases

- **Fleet rebalancing**: A room detects it's in a congested zone and issues a `HelmOrder` to reposition.
- **Emergency avoidance**: An agent grabs the `Tiller` to manually steer away from a failing neighbor, then releases back to autopilot.
- **Audit trail**: The `HelmLog` records every course change for post-hoc analysis of fleet movement patterns.

## Ecosystem Context

Part of the SuperInstance ternary fleet library. This crate handles the *steering* layer — deciding which direction to go. It pairs naturally with `ternary-anchor` (holding position when not moving) and `ternary-current` (information flow that informs steering decisions).

## License

MIT
