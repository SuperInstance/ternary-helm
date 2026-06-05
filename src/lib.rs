#![forbid(unsafe_code)]

//! ternary-helm: Steering and control for fleet navigation.
//!
//! Provides helm-level abstractions for directing rooms through a ternary
//! fleet topology: steering control, rudder direction, manual override,
//! automated course keeping, ordered course changes, and a full action log.

use std::time::{Duration, Instant};

/// Ternary heading: port (-1), amidships (0), or starboard (+1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Heading {
    Port,
    Amidships,
    Starboard,
}

impl Heading {
    /// Convert to a numeric ternary value.
    pub fn to_ternary(self) -> i8 {
        match self {
            Heading::Port => -1,
            Heading::Amidships => 0,
            Heading::Starboard => 1,
        }
    }

    /// Convert from a numeric ternary value.
    pub fn from_ternary(v: i8) -> Option<Self> {
        match v {
            -1 => Some(Heading::Port),
            0 => Some(Heading::Amidships),
            1 => Some(Heading::Starboard),
            _ => None,
        }
    }
}

/// A single rudder providing directional control.
#[derive(Debug, Clone)]
pub struct Rudder {
    angle: i8, // -1, 0, +1
    max_deflection: i8,
}

impl Rudder {
    pub fn new() -> Self {
        Rudder {
            angle: 0,
            max_deflection: 1,
        }
    }

    /// Deflect the rudder. Clamps to [-max, +max].
    pub fn deflect(&mut self, amount: i8) {
        self.angle = amount.clamp(-self.max_deflection, self.max_deflection);
    }

    /// Center the rudder back to amidships.
    pub fn center(&mut self) {
        self.angle = 0;
    }

    pub fn angle(&self) -> i8 {
        self.angle
    }
}

impl Default for Rudder {
    fn default() -> Self {
        Self::new()
    }
}

/// Manual override for when the autopilot isn't cutting it.
#[derive(Debug, Clone)]
pub struct Tiller {
    engaged: bool,
    override_heading: Heading,
}

impl Tiller {
    pub fn new() -> Self {
        Tiller {
            engaged: false,
            override_heading: Heading::Amidships,
        }
    }

    /// Grab the tiller and set a heading.
    pub fn grab(&mut self, heading: Heading) {
        self.engaged = true;
        self.override_heading = heading;
    }

    /// Release the tiller back to autopilot.
    pub fn release(&mut self) {
        self.engaged = false;
    }

    pub fn is_engaged(&self) -> bool {
        self.engaged
    }

    pub fn heading(&self) -> Heading {
        self.override_heading
    }
}

impl Default for Tiller {
    fn default() -> Self {
        Self::new()
    }
}

/// Autopilot keeps the helm on a set course automatically.
#[derive(Debug, Clone)]
pub struct Autopilot {
    active: bool,
    target_heading: Heading,
    tolerance: i8,
}

impl Autopilot {
    pub fn new(target: Heading) -> Self {
        Autopilot {
            active: true,
            target_heading: target,
            tolerance: 0,
        }
    }

    /// Change the autopilot target.
    pub fn set_target(&mut self, heading: Heading) {
        self.target_heading = heading;
    }

    /// Activate autopilot.
    pub fn engage(&mut self) {
        self.active = true;
    }

    /// Deactivate autopilot.
    pub fn disengage(&mut self) {
        self.active = false;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn target(&self) -> Heading {
        self.target_heading
    }

    /// Compute the correction needed given the current heading.
    /// Returns None if autopilot is off. Returns Some(0) if on course.
    pub fn correction(&self, current: Heading) -> Option<i8> {
        if !self.active {
            return None;
        }
        let diff = self.target_heading.to_ternary() - current.to_ternary();
        Some(diff.clamp(-1, 1))
    }
}

/// A command to change the vessel's course.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelmOrder {
    target_heading: Heading,
    reason: String,
    issued_at: Instant,
}

impl HelmOrder {
    pub fn new(heading: Heading, reason: impl Into<String>) -> Self {
        HelmOrder {
            target_heading: heading,
            reason: reason.into(),
            issued_at: Instant::now(),
        }
    }

    pub fn target_heading(&self) -> Heading {
        self.target_heading
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }

    pub fn age(&self) -> Duration {
        self.issued_at.elapsed()
    }
}

/// A single entry in the helm log.
#[derive(Debug, Clone)]
pub struct HelmLogEntry {
    pub action: String,
    pub heading_before: Heading,
    pub heading_after: Heading,
    pub timestamp: Instant,
}

/// Record of all helm actions for auditing and replay.
#[derive(Debug, Clone)]
pub struct HelmLog {
    entries: Vec<HelmLogEntry>,
    max_entries: usize,
}

impl HelmLog {
    pub fn new() -> Self {
        HelmLog {
            entries: Vec::new(),
            max_entries: 1000,
        }
    }

    /// Record a helm action.
    pub fn record(&mut self, action: impl Into<String>, before: Heading, after: Heading) {
        if self.entries.len() >= self.max_entries {
            self.entries.remove(0);
        }
        self.entries.push(HelmLogEntry {
            action: action.into(),
            heading_before: before,
            heading_after: after,
            timestamp: Instant::now(),
        });
    }

    pub fn entries(&self) -> &[HelmLogEntry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all log entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

impl Default for HelmLog {
    fn default() -> Self {
        Self::new()
    }
}

/// The main helm: steering control for a room in the fleet.
#[derive(Debug)]
pub struct Helm {
    heading: Heading,
    rudder: Rudder,
    tiller: Tiller,
    autopilot: Autopilot,
    log: HelmLog,
}

impl Helm {
    pub fn new(initial_heading: Heading) -> Self {
        let target = initial_heading;
        Helm {
            heading: initial_heading,
            rudder: Rudder::new(),
            tiller: Tiller::new(),
            autopilot: Autopilot::new(target),
            log: HelmLog::new(),
        }
    }

    /// Get current heading.
    pub fn heading(&self) -> Heading {
        self.heading
    }

    /// Apply a helm order: change course and log it.
    pub fn apply_order(&mut self, order: &HelmOrder) {
        let before = self.heading;
        self.heading = order.target_heading;
        self.autopilot.set_target(order.target_heading);
        self.rudder.deflect(order.target_heading.to_ternary());
        self.log.record(
            format!("order: {}", order.reason),
            before,
            self.heading,
        );
    }

    /// Manual tiller override.
    pub fn grab_tiller(&mut self, heading: Heading) {
        let before = self.heading;
        self.tiller.grab(heading);
        self.heading = heading;
        self.autopilot.disengage();
        self.log.record("tiller override", before, heading);
    }

    /// Release tiller back to autopilot.
    pub fn release_tiller(&mut self) {
        self.tiller.release();
        self.autopilot.engage();
        self.rudder.center();
    }

    pub fn rudder(&self) -> &Rudder {
        &self.rudder
    }

    pub fn rudder_mut(&mut self) -> &mut Rudder {
        &mut self.rudder
    }

    pub fn tiller(&self) -> &Tiller {
        &self.tiller
    }

    pub fn autopilot(&self) -> &Autopilot {
        &self.autopilot
    }

    pub fn autopilot_mut(&mut self) -> &mut Autopilot {
        &mut self.autopilot
    }

    pub fn log(&self) -> &HelmLog {
        &self.log
    }

    pub fn log_mut(&mut self) -> &mut HelmLog {
        &mut self.log
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_to_ternary() {
        assert_eq!(Heading::Port.to_ternary(), -1);
        assert_eq!(Heading::Amidships.to_ternary(), 0);
        assert_eq!(Heading::Starboard.to_ternary(), 1);
    }

    #[test]
    fn heading_from_ternary() {
        assert_eq!(Heading::from_ternary(-1), Some(Heading::Port));
        assert_eq!(Heading::from_ternary(0), Some(Heading::Amidships));
        assert_eq!(Heading::from_ternary(1), Some(Heading::Starboard));
        assert_eq!(Heading::from_ternary(2), None);
        assert_eq!(Heading::from_ternary(-5), None);
    }

    #[test]
    fn rudder_new_centered() {
        let r = Rudder::new();
        assert_eq!(r.angle(), 0);
    }

    #[test]
    fn rudder_deflect() {
        let mut r = Rudder::new();
        r.deflect(1);
        assert_eq!(r.angle(), 1);
        r.deflect(-1);
        assert_eq!(r.angle(), -1);
    }

    #[test]
    fn rudder_center() {
        let mut r = Rudder::new();
        r.deflect(1);
        r.center();
        assert_eq!(r.angle(), 0);
    }

    #[test]
    fn tiller_grab_and_release() {
        let mut t = Tiller::new();
        assert!(!t.is_engaged());
        t.grab(Heading::Port);
        assert!(t.is_engaged());
        assert_eq!(t.heading(), Heading::Port);
        t.release();
        assert!(!t.is_engaged());
    }

    #[test]
    fn autopilot_correction_on_course() {
        let ap = Autopilot::new(Heading::Starboard);
        assert_eq!(ap.correction(Heading::Starboard), Some(0));
    }

    #[test]
    fn autopilot_correction_needed() {
        let ap = Autopilot::new(Heading::Starboard);
        assert_eq!(ap.correction(Heading::Amidships), Some(1));
    }

    #[test]
    fn autopilot_correction_off() {
        let mut ap = Autopilot::new(Heading::Starboard);
        ap.disengage();
        assert_eq!(ap.correction(Heading::Amidships), None);
    }

    #[test]
    fn autopilot_set_target() {
        let mut ap = Autopilot::new(Heading::Amidships);
        ap.set_target(Heading::Port);
        assert_eq!(ap.target(), Heading::Port);
    }

    #[test]
    fn helm_order_creation() {
        let order = HelmOrder::new(Heading::Port, "avoid obstacle");
        assert_eq!(order.target_heading(), Heading::Port);
        assert_eq!(order.reason(), "avoid obstacle");
    }

    #[test]
    fn helm_log_record() {
        let mut log = HelmLog::new();
        assert!(log.is_empty());
        log.record("test", Heading::Amidships, Heading::Starboard);
        assert_eq!(log.len(), 1);
        assert_eq!(log.entries()[0].heading_before, Heading::Amidships);
        assert_eq!(log.entries()[0].heading_after, Heading::Starboard);
    }

    #[test]
    fn helm_log_max_entries() {
        let mut log = HelmLog::new();
        for i in 0..1005 {
            log.record(format!("entry {}", i), Heading::Amidships, Heading::Port);
        }
        assert_eq!(log.len(), 1000);
    }

    #[test]
    fn helm_log_clear() {
        let mut log = HelmLog::new();
        log.record("a", Heading::Amidships, Heading::Port);
        log.record("b", Heading::Port, Heading::Starboard);
        log.clear();
        assert!(log.is_empty());
    }

    #[test]
    fn helm_apply_order() {
        let mut helm = Helm::new(Heading::Amidships);
        let order = HelmOrder::new(Heading::Starboard, "reposition");
        helm.apply_order(&order);
        assert_eq!(helm.heading(), Heading::Starboard);
        assert_eq!(helm.log().len(), 1);
    }

    #[test]
    fn helm_tiller_override() {
        let mut helm = Helm::new(Heading::Amidships);
        helm.grab_tiller(Heading::Port);
        assert_eq!(helm.heading(), Heading::Port);
        assert!(helm.tiller().is_engaged());
        assert!(!helm.autopilot().is_active());
    }

    #[test]
    fn helm_release_tiller() {
        let mut helm = Helm::new(Heading::Amidships);
        helm.grab_tiller(Heading::Port);
        helm.release_tiller();
        assert!(!helm.tiller().is_engaged());
        assert!(helm.autopilot().is_active());
        assert_eq!(helm.rudder().angle(), 0);
    }

    #[test]
    fn helm_multiple_orders() {
        let mut helm = Helm::new(Heading::Amidships);
        helm.apply_order(&HelmOrder::new(Heading::Port, "first"));
        helm.apply_order(&HelmOrder::new(Heading::Starboard, "second"));
        helm.apply_order(&HelmOrder::new(Heading::Amidships, "third"));
        assert_eq!(helm.heading(), Heading::Amidships);
        assert_eq!(helm.log().len(), 3);
    }

    #[test]
    fn autopilot_correction_port_to_starboard() {
        let ap = Autopilot::new(Heading::Starboard);
        // Port(-1) to Starboard(1): diff = 1 - (-1) = 2, clamped to 1
        assert_eq!(ap.correction(Heading::Port), Some(1));
    }

    #[test]
    fn helm_new_heading() {
        let helm = Helm::new(Heading::Port);
        assert_eq!(helm.heading(), Heading::Port);
        assert!(helm.autopilot().is_active());
    }
}
